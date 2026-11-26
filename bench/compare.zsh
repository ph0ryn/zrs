#!/usr/bin/env zsh
set -euo pipefail

zmodload zsh/datetime

script_dir="${0:A:h}"
repo_root="${script_dir:h}"
zrs_bin="${1:-${ZRS_BIN:-${repo_root}/target/release/zrs-zsh}}"

deps_dir="${BENCH_DEPS_DIR:-${TMPDIR:-/tmp}/zrs-bench-deps}"
zeno_dir="${ZENO_DIR:-${deps_dir}/zeno.zsh}"
zsh_abbr_dir="${ZSH_ABBR_DIR:-${deps_dir}/zsh-abbr}"
auto_clone="${AUTO_CLONE:-1}"

startup_warmup="${STARTUP_WARMUP:-3}"
startup_runs="${STARTUP_RUNS:-20}"
warmup="${WARMUP:-10}"
runs="${RUNS:-100}"
scale_1000_runs="${SCALE_1000_RUNS:-30}"
scale_10000_runs="${SCALE_10000_RUNS:-10}"
cold_runs="${COLD_RUNS:-5}"
scales=(${=BENCH_SCALES:-100 1000 10000})

if [[ ! -x "$zrs_bin" ]]; then
  print -u2 "bench/compare: executable not found: $zrs_bin"
  print -u2 "bench/compare: run 'cargo build --release' or pass a binary path as \$1"
  exit 1
fi

has_deno=0
if command -v deno >/dev/null 2>&1; then
  has_deno=1
fi

ensure_repo() {
  local name="$1" url="$2" dest="$3" extra_flag="${4:-}"
  if [[ -d "$dest" ]]; then
    return 0
  fi
  if (( ! auto_clone )); then
    print -u2 "bench/compare: $name not found at $dest (set AUTO_CLONE=1 or pass its directory)"
    return 1
  fi
  mkdir -p "${dest:h}"
  print -u2 "bench/compare: cloning $name into $dest ..."
  if [[ -n "$extra_flag" ]]; then
    git clone --quiet --depth 1 "$extra_flag" "$url" "$dest"
  else
    git clone --quiet --depth 1 "$url" "$dest"
  fi
}

have_zeno=0
if (( has_deno )) && ensure_repo "zeno.zsh" "https://github.com/yuki-yano/zeno.zsh.git" "$zeno_dir"; then
  have_zeno=1
elif (( ! has_deno )); then
  print -u2 "bench/compare: deno not found; skipping zeno.zsh"
fi

have_zsh_abbr=0
if ensure_repo "zsh-abbr" "https://github.com/olets/zsh-abbr.git" "$zsh_abbr_dir" "--recurse-submodules"; then
  have_zsh_abbr=1
fi

tmpdir="$(mktemp -d)"
cleanup() {
  # Stop any stray zeno-server processes associated with this benchmark run.
  pkill -f "zeno-server" 2>/dev/null || true
  rm -rf -- "$tmpdir"
}
trap cleanup EXIT INT TERM

# Generate equivalent abbreviation configs for zrs, zeno.zsh, and zsh-abbr.
generate_configs() {
  local -i n="$1"
  local dir="$2"
  mkdir -p "$dir/zrs/zrs-zsh" "$dir/zeno" "$dir/abbr" "$dir/abbr-tmp"

  awk -v n="$n" 'BEGIN {
    print "abbreviations:"
    print "  - keyword: gs"
    print "    expand: git status --short --branch"
    for (i = 1; i < n; i++) {
      id = sprintf("%05d", i)
      printf "  - keyword: kw%s\n", id
      printf "    expand: echo expanded-%s --flag=%s\n", id, id
      if (i % 5 == 0) {
        print "    global: true"
      }
    }
  }' > "$dir/zrs/zrs-zsh/config.yaml"

  awk -v n="$n" 'BEGIN {
    print "snippets:"
    print "  - name: gs"
    print "    keyword: gs"
    print "    snippet: git status --short --branch"
    for (i = 1; i < n; i++) {
      id = sprintf("%05d", i)
      printf "  - name: kw%s\n", id
      printf "    keyword: kw%s\n", id
      printf "    snippet: echo expanded-%s --flag=%s\n", id, id
      if (i % 5 == 0) {
        print "    context:"
        print "      lbuffer: '\''.+\\s'\''"
      }
    }
  }' > "$dir/zeno/config.yml"

  awk -v n="$n" 'BEGIN {
    print "abbr \"gs\"=\"git status --short --branch\""
    for (i = 1; i < n; i++) {
      id = sprintf("%05d", i)
      if (i % 5 == 0) {
        printf "abbr -g \"kw%s\"=\"echo expanded-%s --flag=%s\"\n", id, id, id
      } else {
        printf "abbr \"kw%s\"=\"echo expanded-%s --flag=%s\"\n", id, id, id
      }
    }
  }' > "$dir/abbr/user-abbreviations"
}

# Compute p50 / p95 / p99 / min / max from newline-separated millisecond values on stdin.
format_stats() {
  local -a raw sorted
  local -i n p50_i p95_i p99_i
  raw=("${(@f)$(cat)}")
  sorted=("${(n)raw[@]}")
  n=${#sorted}
  if (( n == 0 )); then
    printf "%9s %9s %9s %9s %9s" "N/A" "N/A" "N/A" "N/A" "N/A"
    return
  fi
  p50_i=$(( (n + 1) / 2 ))
  p95_i=$(( (n * 95 + 99) / 100 ))
  p99_i=$(( (n * 99 + 99) / 100 ))
  (( p95_i > n )) && p95_i=$n
  (( p99_i > n )) && p99_i=$n
  printf "%7.2fms %7.2fms %7.2fms %7.2fms %7.2fms" \
    "${sorted[$p50_i]}" "${sorted[$p95_i]}" "${sorted[$p99_i]}" "${sorted[1]}" "${sorted[$n]}"
}

# Measure zsh -di -c exit startup latency for a given .zshrc snippet (with global RCS disabled).
bench_startup() {
  local label="$1"
  local zdotdir="$2"
  local verify_cmd="${3:-}"
  local -a times
  local -i i
  local -F t0 t1 dt

  # Warmup (also populates caches like zsh-abbr and deno)
  for (( i = 0; i < startup_warmup; i++ )); do
    ZDOTDIR="$zdotdir" zsh -di -c exit >/dev/null 2>&1
  done

  if [[ -n "$verify_cmd" ]]; then
    if ! ZDOTDIR="$zdotdir" zsh -di -c "$verify_cmd" >/dev/null 2>&1; then
      print -u2 "bench/compare: startup sanity check failed for '$label'"
      exit 1
    fi
  fi

  times=()
  for (( i = 0; i < startup_runs; i++ )); do
    t0=$EPOCHREALTIME
    ZDOTDIR="$zdotdir" zsh -di -c exit >/dev/null 2>&1
    t1=$EPOCHREALTIME
    dt=$(( (t1 - t0) * 1000.0 ))
    times+=("$(printf '%.6f' "$dt")")
  done

  printf "%-30s %s\n" "$label" "$(printf '%s\n' "${times[@]}" | format_stats)"
}

# Measure in-session expansion latency (hit or miss) in a clean interactive-style zsh process.
bench_expand() {
  local tool="$1"
  local cfg_dir="$2"
  local mode="$3" # hit | miss
  local -i n_warmup="$4"
  local -i n_runs="$5"
  local lbuf="gs"
  local expected_prefix="git status --short --branch"
  if [[ "$mode" == "miss" ]]; then
    lbuf="echo hello"
    expected_prefix=""
  fi

  case "$tool" in
    zrs)
      env \
        PATH="${zrs_bin:A:h}:$PATH" \
        XDG_CONFIG_HOME="$cfg_dir/zrs" \
        LBUF="$lbuf" \
        EXPECTED="$expected_prefix" \
        N_WARMUP="$n_warmup" \
        N_RUNS="$n_runs" \
        zsh -df -c '
          zmodload zsh/datetime
          eval "$(zrs-zsh init --no-keybinds)"
          local -i i rc
          local -F t0 t1 dt
          local _zrs_kind
          for (( i = 0; i < N_WARMUP + N_RUNS; i++ )); do
            LBUFFER="$LBUF"
            RBUFFER=""
            BUFFER=""
            CURSOR=0
            t0=$EPOCHREALTIME
            rc=0
            _zrs_expand || rc=$?
            t1=$EPOCHREALTIME
            if [[ -n "$EXPECTED" ]]; then
              if (( rc != 0 )) || [[ "$BUFFER" != "$EXPECTED" ]]; then
                print -u2 "bench/compare: zrs hit sanity failed: rc=$rc BUFFER=$BUFFER"
                exit 1
              fi
            else
              if (( rc != 2 )); then
                print -u2 "bench/compare: zrs miss sanity failed: rc=$rc"
                exit 1
              fi
            fi
            if (( i >= N_WARMUP )); then
              dt=$(( (t1 - t0) * 1000.0 ))
              printf "%.6f\n" "$dt"
            fi
          done
        '
      ;;
    zsh-abbr)
      env \
        ABBR_USER_ABBREVIATIONS_FILE="$cfg_dir/abbr/user-abbreviations" \
        ABBR_TMPDIR="$cfg_dir/abbr-tmp" \
        ABBR_DEFAULT_BINDINGS=0 \
        ABBR_QUIETER=1 \
        ZSH_ABBR_SCRIPT="$zsh_abbr_dir/zsh-abbr.zsh" \
        LBUF="$lbuf" \
        EXPECTED="$expected_prefix" \
        N_WARMUP="$n_warmup" \
        N_RUNS="$n_runs" \
        zsh -df -c '
          zmodload zsh/datetime
          source "$ZSH_ABBR_SCRIPT"
          local -i i rc
          local -F t0 t1 dt
          local -A reply
          for (( i = 0; i < N_WARMUP + N_RUNS; i++ )); do
            reply=()
            t0=$EPOCHREALTIME
            rc=0
            abbr-expand-line "$LBUF" "" || rc=$?
            t1=$EPOCHREALTIME
            if [[ -n "$EXPECTED" ]]; then
              if (( rc != 0 )) || [[ "${reply[loutput]-}" != "$EXPECTED" ]]; then
                print -u2 "bench/compare: zsh-abbr hit sanity failed: rc=$rc out=${reply[loutput]-}"
                exit 1
              fi
            else
              if (( rc == 0 )); then
                print -u2 "bench/compare: zsh-abbr miss sanity failed: rc=$rc out=${reply[loutput]-}"
                exit 1
              fi
            fi
            if (( i >= N_WARMUP )); then
              dt=$(( (t1 - t0) * 1000.0 ))
              printf "%.6f\n" "$dt"
            fi
          done
        '
      ;;
    zeno.zsh)
      local sock_dir="$tmpdir/zeno-sock-$$-$RANDOM"
      mkdir -p "$sock_dir"
      env \
        ZENO_HOME="$cfg_dir/zeno" \
        ZENO_SOCK_DIR="$sock_dir" \
        ZENO_DISABLE_EXECUTE_CACHE_COMMAND=1 \
        ZENO_SCRIPT="$zeno_dir/zeno.zsh" \
        LBUF="$lbuf" \
        EXPECTED="$expected_prefix" \
        N_WARMUP="$n_warmup" \
        N_RUNS="$n_runs" \
        zsh -df -c '
          zmodload zsh/datetime
          source "$ZENO_SCRIPT"
          trap "zeno-stop-server >/dev/null 2>&1 || true" EXIT
          # Start daemon before timed loop so warm runs measure steady-state IPC latency.
          zeno-start-server >/dev/null 2>&1
          local -i i
          local -F t0 t1 dt
          local -a out
          for (( i = 0; i < N_WARMUP + N_RUNS; i++ )); do
            t0=$EPOCHREALTIME
            out=( "${(f)$(zeno-call-client-and-fallback --zeno-mode=auto-snippet --input.lbuffer="$LBUF" --input.rbuffer="")}" )
            t1=$EPOCHREALTIME
            if [[ -n "$EXPECTED" ]]; then
              if [[ "${out[1]-}" != "success" || "${out[2]-}" != "${EXPECTED}"* ]]; then
                print -u2 "bench/compare: zeno hit sanity failed: out=(${out[*]})"
                exit 1
              fi
            else
              if [[ "${out[1]-}" == "success" ]]; then
                print -u2 "bench/compare: zeno miss sanity failed: out=(${out[*]})"
                exit 1
              fi
            fi
            if (( i >= N_WARMUP )); then
              dt=$(( (t1 - t0) * 1000.0 ))
              printf "%.6f\n" "$dt"
            fi
          done
        '
      rm -rf -- "$sock_dir"
      ;;
  esac
}

# Measure idle RSS (KB) after 1 expansion in a running shell.
bench_rss() {
  local tool="$1"
  local cfg_dir="$2"
  local out=""
  case "$tool" in
    bare)
      out=$(zsh -df -c '
        rss=$(ps -o rss= -p $$ | tr -d " ")
        print "$rss 0 $rss"
      ')
      ;;
    zrs)
      out=$(env \
        PATH="${zrs_bin:A:h}:$PATH" \
        XDG_CONFIG_HOME="$cfg_dir/zrs" \
        zsh -df -c '
          eval "$(zrs-zsh init --no-keybinds)"
          LBUFFER="gs"; RBUFFER=""; _zrs_expand >/dev/null
          rss=$(ps -o rss= -p $$ | tr -d " ")
          print "$rss 0 $rss"
        ')
      ;;
    zsh-abbr)
      out=$(env \
        ABBR_USER_ABBREVIATIONS_FILE="$cfg_dir/abbr/user-abbreviations" \
        ABBR_TMPDIR="$cfg_dir/abbr-tmp" \
        ABBR_DEFAULT_BINDINGS=0 \
        ABBR_QUIETER=1 \
        ZSH_ABBR_SCRIPT="$zsh_abbr_dir/zsh-abbr.zsh" \
        zsh -df -c '
          source "$ZSH_ABBR_SCRIPT"
          local -A reply
          abbr-expand-line "gs" "" >/dev/null
          rss=$(ps -o rss= -p $$ | tr -d " ")
          print "$rss 0 $rss"
        ')
      ;;
    zeno.zsh)
      local sock_dir="$tmpdir/zeno-rss-$$"
      mkdir -p "$sock_dir"
      out=$(env \
        ZENO_HOME="$cfg_dir/zeno" \
        ZENO_SOCK_DIR="$sock_dir" \
        ZENO_DISABLE_EXECUTE_CACHE_COMMAND=1 \
        ZENO_SCRIPT="$zeno_dir/zeno.zsh" \
        zsh -df -c '
          source "$ZENO_SCRIPT"
          trap "zeno-stop-server >/dev/null 2>&1 || true" EXIT
          zeno-call-client-and-fallback --zeno-mode=auto-snippet --input.lbuffer="gs" --input.rbuffer="" >/dev/null
          zeno-set-pid
          shell_rss=$(ps -o rss= -p $$ | tr -d " ")
          daemon_rss=0
          if [[ -n "${ZENO_PID-}" ]]; then
            daemon_rss=$(ps -o rss= -p "$ZENO_PID" 2>/dev/null | tr -d " ")
            daemon_rss=${daemon_rss:-0}
          fi
          total=$(( shell_rss + daemon_rss ))
          print "$shell_rss $daemon_rss $total"
        ')
      rm -rf -- "$sock_dir"
      ;;
  esac
  local -a cols=(${=out})
  printf "%-20s %8.1f MB %9.1f MB %9.1f MB\n" \
    "$tool" \
    "$(( cols[1] / 1024.0 ))" \
    "$(( cols[2] / 1024.0 ))" \
    "$(( cols[3] / 1024.0 ))"
}

# Measure cold-start first expansion latency (fresh shell -> first expand call).
bench_cold_start() {
  local tool="$1"
  local cfg_dir="$2"
  local -a times
  local -i i
  local dt

  times=()
  for (( i = 0; i < cold_runs; i++ )); do
    case "$tool" in
      zrs)
        dt=$(env \
          PATH="${zrs_bin:A:h}:$PATH" \
          XDG_CONFIG_HOME="$cfg_dir/zrs" \
          zsh -df -c '
            zmodload zsh/datetime
            eval "$(zrs-zsh init --no-keybinds)"
            LBUFFER="gs"; RBUFFER=""
            t0=$EPOCHREALTIME
            _zrs_expand
            t1=$EPOCHREALTIME
            printf "%.6f\n" $(( (t1 - t0) * 1000.0 ))
          ')
        ;;
      zsh-abbr)
        local fresh_abbr_tmp="$tmpdir/cold-abbr-$i"
        mkdir -p "$fresh_abbr_tmp"
        # Pre-warm cache just like normal shell; cold here is first expansion in a fresh shell.
        cp -a "$cfg_dir/abbr-tmp/." "$fresh_abbr_tmp/" 2>/dev/null || true
        dt=$(env \
          ABBR_USER_ABBREVIATIONS_FILE="$cfg_dir/abbr/user-abbreviations" \
          ABBR_TMPDIR="$fresh_abbr_tmp" \
          ABBR_DEFAULT_BINDINGS=0 \
          ABBR_QUIETER=1 \
          ZSH_ABBR_SCRIPT="$zsh_abbr_dir/zsh-abbr.zsh" \
          zsh -df -c '
            zmodload zsh/datetime
            source "$ZSH_ABBR_SCRIPT"
            local -A reply
            t0=$EPOCHREALTIME
            abbr-expand-line "gs" ""
            t1=$EPOCHREALTIME
            printf "%.6f\n" $(( (t1 - t0) * 1000.0 ))
          ')
        rm -rf -- "$fresh_abbr_tmp"
        ;;
      zeno.zsh)
        local sock_dir="$tmpdir/zeno-cold-$i"
        mkdir -p "$sock_dir"
        dt=$(env \
          ZENO_HOME="$cfg_dir/zeno" \
          ZENO_SOCK_DIR="$sock_dir" \
          ZENO_DISABLE_EXECUTE_CACHE_COMMAND=1 \
          ZENO_SCRIPT="$zeno_dir/zeno.zsh" \
          zsh -df -c '
            zmodload zsh/datetime
            source "$ZENO_SCRIPT"
            trap "zeno-stop-server >/dev/null 2>&1 || true" EXIT
            t0=$EPOCHREALTIME
            out=( "${(f)$(zeno-call-client-and-fallback --zeno-mode=auto-snippet --input.lbuffer="gs" --input.rbuffer="")}" )
            t1=$EPOCHREALTIME
            [[ "${out[1]-}" == "success" ]] || exit 1
            printf "%.6f\n" $(( (t1 - t0) * 1000.0 ))
          ')
        rm -rf -- "$sock_dir"
        ;;
    esac
    times+=("$dt")
  done

  printf "%-20s %s\n" "$tool" "$(printf '%s\n' "${times[@]}" | format_stats)"
}

# Check whether editing the config file takes effect immediately without reload/restart.
check_config_reload() {
  local tool="$1"
  local work_dir="$tmpdir/reload-$tool"
  generate_configs 10 "$work_dir"

  local status_text=""
  case "$tool" in
    zrs)
      status_text=$(env \
        PATH="${zrs_bin:A:h}:$PATH" \
        XDG_CONFIG_HOME="$work_dir/zrs" \
        CFG_FILE="$work_dir/zrs/zrs-zsh/config.yaml" \
        zsh -df -c '
          eval "$(zrs-zsh init --no-keybinds)"
          LBUFFER="dyn_bench"; RBUFFER=""
          _zrs_expand && exit 1
          printf "  - keyword: dyn_bench\n    expand: echo dynamic-ok\n" >> "$CFG_FILE"
          LBUFFER="dyn_bench"; RBUFFER=""; BUFFER=""
          if _zrs_expand && [[ "$BUFFER" == "echo dynamic-ok" ]]; then
            print "immediate (no reload needed)"
          else
            print "not reflected (requires reload/restart)"
          fi
        ')
      ;;
    zsh-abbr)
      status_text=$(env \
        ABBR_USER_ABBREVIATIONS_FILE="$work_dir/abbr/user-abbreviations" \
        ABBR_TMPDIR="$work_dir/abbr-tmp" \
        ABBR_DEFAULT_BINDINGS=0 \
        ABBR_QUIETER=1 \
        ZSH_ABBR_SCRIPT="$zsh_abbr_dir/zsh-abbr.zsh" \
        zsh -df -c '
          source "$ZSH_ABBR_SCRIPT"
          local -A reply
          abbr-expand-line "dyn_bench" "" && exit 1
          printf "abbr \"dyn_bench\"=\"echo dynamic-ok\"\n" >> "$ABBR_USER_ABBREVIATIONS_FILE"
          reply=()
          if abbr-expand-line "dyn_bench" "" && [[ "${reply[loutput]-}" == "echo dynamic-ok" ]]; then
            print "immediate (no reload needed)"
          else
            print "not reflected (requires abbr-load / new shell)"
          fi
        ')
      ;;
    zeno.zsh)
      local sock_dir="$tmpdir/zeno-reload"
      mkdir -p "$sock_dir"
      status_text=$(env \
        ZENO_HOME="$work_dir/zeno" \
        ZENO_SOCK_DIR="$sock_dir" \
        ZENO_DISABLE_EXECUTE_CACHE_COMMAND=1 \
        ZENO_SCRIPT="$zeno_dir/zeno.zsh" \
        CFG_FILE="$work_dir/zeno/config.yml" \
        zsh -df -c '
          source "$ZENO_SCRIPT"
          trap "zeno-stop-server >/dev/null 2>&1 || true" EXIT
          zeno-start-server >/dev/null 2>&1
          out=( "${(f)$(zeno-call-client-and-fallback --zeno-mode=auto-snippet --input.lbuffer="dyn_bench" --input.rbuffer="")}" )
          [[ "${out[1]-}" != "success" ]] || exit 1
          printf "  - name: dyn_bench\n    keyword: dyn_bench\n    snippet: echo dynamic-ok\n" >> "$CFG_FILE"
          out=( "${(f)$(zeno-call-client-and-fallback --zeno-mode=auto-snippet --input.lbuffer="dyn_bench" --input.rbuffer="")}" )
          if [[ "${out[1]-}" == "success" && "${out[2]-}" == "echo dynamic-ok"* ]]; then
            print "immediate (no reload needed)"
          else
            print "not reflected (requires zeno-restart-server / reload)"
          fi
        ')
      rm -rf -- "$sock_dir"
      ;;
  esac
  printf "%-20s %s\n" "$tool" "$status_text"
}

# --- Header / environment info ---
print "=== Environment ==="
print "OS:       $(uname -srm)"
print "Zsh:      $(zsh --version)"
print "zrs:      $("$zrs_bin" --version) ($zrs_bin)"
if (( have_zsh_abbr )); then
  print "zsh-abbr: $(git -C "$zsh_abbr_dir" describe --tags --always 2>/dev/null || git -C "$zsh_abbr_dir" rev-parse --short HEAD)"
fi
if (( have_zeno )); then
  print "zeno.zsh: $(git -C "$zeno_dir" describe --tags --always 2>/dev/null || git -C "$zeno_dir" rev-parse --short HEAD) ($(deno --version | head -n 1))"
fi
print "Config:   startup_runs=${startup_runs}, warmup=${warmup}, runs=${runs} (1000:${scale_1000_runs}, 10000:${scale_10000_runs}), cold_runs=${cold_runs}"
print ""

# Pre-generate base 100-abbreviation config
base_cfg="$tmpdir/scale-100"
generate_configs 100 "$base_cfg"

# Prime deno cache once for zeno.zsh so startup timings are reproducible
if (( have_zeno )); then
  command deno cache --node-modules-dir=auto --no-lock --no-check "${zeno_dir}/src/cli.ts" >/dev/null 2>&1 || true
fi

# --- 1. Zsh startup time (100 abbreviations) ---
print "=== 1. Zsh Startup Time (zsh -di -c exit, 100 abbreviations) ==="
printf "%-30s %9s %9s %9s %9s %9s\n" "Tool" "p50" "p95" "p99" "min" "max"
print -- "--------------------------------------------------------------------------------"

mkdir -p "$tmpdir/zdot-bare" "$tmpdir/zdot-zrs" "$tmpdir/zdot-abbr" "$tmpdir/zdot-zeno-default" "$tmpdir/zdot-zeno-nocache"
: > "$tmpdir/zdot-bare/.zshrc"
cat > "$tmpdir/zdot-zrs/.zshrc" <<EOF
export PATH="${zrs_bin:A:h}:\$PATH"
export XDG_CONFIG_HOME="$base_cfg/zrs"
eval "\$(zrs-zsh init)"
EOF
bench_startup "bare zsh" "$tmpdir/zdot-bare"
bench_startup "zrs" "$tmpdir/zdot-zrs" '(( $+functions[_zrs_expand] ))'

if (( have_zsh_abbr )); then
  cat > "$tmpdir/zdot-abbr/.zshrc" <<EOF
export ABBR_USER_ABBREVIATIONS_FILE="$base_cfg/abbr/user-abbreviations"
export ABBR_TMPDIR="$base_cfg/abbr-tmp"
export ABBR_QUIETER=1
source "$zsh_abbr_dir/zsh-abbr.zsh"
EOF
  bench_startup "zsh-abbr (cached)" "$tmpdir/zdot-abbr" '(( $+functions[abbr-expand-line] ))'
fi

if (( have_zeno )); then
  cat > "$tmpdir/zdot-zeno-default/.zshrc" <<EOF
export ZENO_HOME="$base_cfg/zeno"
export ZENO_SOCK_DIR="$tmpdir/zdot-zeno-sock"
source "$zeno_dir/zeno.zsh"
EOF
  cat > "$tmpdir/zdot-zeno-nocache/.zshrc" <<EOF
export ZENO_HOME="$base_cfg/zeno"
export ZENO_SOCK_DIR="$tmpdir/zdot-zeno-sock"
export ZENO_DISABLE_EXECUTE_CACHE_COMMAND=1
source "$zeno_dir/zeno.zsh"
EOF
  bench_startup "zeno.zsh (default)" "$tmpdir/zdot-zeno-default" '[[ ${ZENO_LOADED-} == 1 ]]'
  bench_startup "zeno.zsh (no deno cache)" "$tmpdir/zdot-zeno-nocache" '[[ ${ZENO_LOADED-} == 1 ]]'
fi

# --- 2 & 3. Expansion latency (hit & miss) at 100 abbreviations ---
print ""
print "=== 2. Abbreviation Expansion Latency (hit: 'gs', 100 abbreviations) ==="
printf "%-20s %9s %9s %9s %9s %9s\n" "Tool" "p50" "p95" "p99" "min" "max"
print -- "----------------------------------------------------------------------"
printf "%-20s %s\n" "zrs" "$(bench_expand zrs "$base_cfg" hit "$warmup" "$runs" | format_stats)"
if (( have_zsh_abbr )); then
  printf "%-20s %s\n" "zsh-abbr" "$(bench_expand zsh-abbr "$base_cfg" hit "$warmup" "$runs" | format_stats)"
fi
if (( have_zeno )); then
  printf "%-20s %s\n" "zeno.zsh" "$(bench_expand zeno.zsh "$base_cfg" hit "$warmup" "$runs" | format_stats)"
fi

print ""
print "=== 3. Abbreviation Miss Latency (miss: 'echo hello', 100 abbreviations) ==="
printf "%-20s %9s %9s %9s %9s %9s\n" "Tool" "p50" "p95" "p99" "min" "max"
print -- "----------------------------------------------------------------------"
printf "%-20s %s\n" "zrs" "$(bench_expand zrs "$base_cfg" miss "$warmup" "$runs" | format_stats)"
if (( have_zsh_abbr )); then
  printf "%-20s %s\n" "zsh-abbr" "$(bench_expand zsh-abbr "$base_cfg" miss "$warmup" "$runs" | format_stats)"
fi
if (( have_zeno )); then
  printf "%-20s %s\n" "zeno.zsh" "$(bench_expand zeno.zsh "$base_cfg" miss "$warmup" "$runs" | format_stats)"
fi

# --- 4. Idle RSS ---
print ""
print "=== 4. Idle RSS (after 1 expansion, 100 abbreviations) ==="
printf "%-20s %11s %12s %12s\n" "Tool" "Shell RSS" "Daemon RSS" "Total RSS"
print -- "----------------------------------------------------------"
bench_rss bare "$base_cfg"
bench_rss zrs "$base_cfg"
if (( have_zsh_abbr )); then
  bench_rss zsh-abbr "$base_cfg"
fi
if (( have_zeno )); then
  bench_rss zeno.zsh "$base_cfg"
fi

# --- 5. Scalability: 100 / 1,000 / 10,000 abbreviations ---
print ""
print "=== 5. Scalability (abbreviations count) ==="
printf "%-26s %9s %9s %9s %9s %9s\n" "Scenario" "p50" "p95" "p99" "min" "max"
print -- "----------------------------------------------------------------------------"

for scale in "${scales[@]}"; do
  local cfg="$tmpdir/scale-${scale}"
  if [[ ! -d "$cfg" ]]; then
    generate_configs "$scale" "$cfg"
  fi
  local -i s_runs="$runs" s_warmup="$warmup"
  if (( scale >= 10000 )); then
    s_runs="$scale_10000_runs"
    s_warmup=2
  elif (( scale >= 1000 )); then
    s_runs="$scale_1000_runs"
    s_warmup=5
  fi

  printf "%-26s %s\n" "zrs hit [${scale}]"  "$(bench_expand zrs "$cfg" hit "$s_warmup" "$s_runs" | format_stats)"
  printf "%-26s %s\n" "zrs miss [${scale}]" "$(bench_expand zrs "$cfg" miss "$s_warmup" "$s_runs" | format_stats)"
  if (( have_zsh_abbr )); then
    printf "%-26s %s\n" "zsh-abbr hit [${scale}]"  "$(bench_expand zsh-abbr "$cfg" hit "$s_warmup" "$s_runs" | format_stats)"
    printf "%-26s %s\n" "zsh-abbr miss [${scale}]" "$(bench_expand zsh-abbr "$cfg" miss "$s_warmup" "$s_runs" | format_stats)"
  fi
  if (( have_zeno )); then
    printf "%-26s %s\n" "zeno.zsh hit [${scale}]"  "$(bench_expand zeno.zsh "$cfg" hit "$s_warmup" "$s_runs" | format_stats)"
    printf "%-26s %s\n" "zeno.zsh miss [${scale}]" "$(bench_expand zeno.zsh "$cfg" miss "$s_warmup" "$s_runs" | format_stats)"
  fi
done

# --- 6. Cold start ---
print ""
print "=== 6. Cold Start (first expansion in a fresh shell, 100 abbreviations) ==="
printf "%-20s %9s %9s %9s %9s %9s\n" "Tool" "p50" "p95" "p99" "min" "max"
print -- "----------------------------------------------------------------------"
bench_cold_start zrs "$base_cfg"
if (( have_zsh_abbr )); then
  bench_cold_start zsh-abbr "$base_cfg"
fi
if (( have_zeno )); then
  bench_cold_start zeno.zsh "$base_cfg"
fi

# --- 7. Config change immediate reflection ---
print ""
print "=== 7. Config Change Behavior (append rule while shell is running) ==="
printf "%-20s %s\n" "Tool" "Result"
print -- "----------------------------------------------------------------------"
check_config_reload zrs
if (( have_zsh_abbr )); then
  check_config_reload zsh-abbr
fi
if (( have_zeno )); then
  check_config_reload zeno.zsh
fi
