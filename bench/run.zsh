#!/usr/bin/env zsh
set -euo pipefail

zmodload zsh/datetime

script_dir="${0:A:h}"
repo_root="${script_dir:h}"
zrs_bin="${1:-${ZRS_BIN:-${repo_root}/target/release/zrs}}"
warmup="${WARMUP:-10}"
runs="${RUNS:-100}"

if [[ ! -x "$zrs_bin" ]]; then
  print -u2 "bench: executable not found: $zrs_bin"
  print -u2 "bench: run 'cargo build --release' or pass a binary path as \$1"
  exit 1
fi

if ! command -v hyperfine >/dev/null 2>&1; then
  print -u2 "bench: hyperfine is required (run inside 'nix develop')"
  exit 1
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

generate_config() {
  local out_file="$1"
  local -i n_abbr="$2" n_ctx="$3" n_snip="$4" n_comp="$5" i
  {
    print "abbreviations:"
    print "  - keyword: gs"
    print "    expand: git status --short --branch"
    for (( i = 1; i < n_abbr; i++ )); do
      local id="$(printf '%03d' "$i")"
      print "  - keyword: kw${id}"
      print "    expand: echo expanded-${id} --flag=${id}"
      if (( i <= n_ctx )); then
        print "    context:"
        print "      lbuffer: '^git\\s+(checkout|switch|merge|rebase)\\s+.*--opt-${id}\\s+'"
        if (( i % 3 == 0 )); then
          print "      buffer: '^git\\s+.*(main|develop|feat/[a-z0-9_-]+)'"
        fi
      elif (( i % 5 == 0 )); then
        print "    global: true"
      fi
    done

    print "snippets:"
    for (( i = 0; i < n_snip; i++ )); do
      local id="$(printf '%03d' "$i")"
      print "  - name: snippet-${id}"
      print "    body: \"docker run --rm -it '{{image_${id}}}' --name '{{name_${id}}}'\""
    done

    print "completions:"
    for (( i = 0; i < n_comp; i++ )); do
      local id="$(printf '%03d' "$i")"
      print "  - name: comp-${id}"
      print "    pattern: '^cmd${id}\\s+(sub1|sub2|sub3)\\s+[a-z0-9_/-]*'"
      print "    source: \"printf 'a\\nb\\nc\\n'\""
      print "    preview: 'echo {}'"
    done
  } > "$out_file"
}

mkdir -p \
  "$tmpdir/none" \
  "$tmpdir/small/zrs" \
  "$tmpdir/standard/zrs" \
  "$tmpdir/large/zrs"

cp "${repo_root}/examples/config.yaml" "$tmpdir/small/zrs/config.yaml"
generate_config "$tmpdir/standard/zrs/config.yaml" 100 20 30 20
generate_config "$tmpdir/large/zrs/config.yaml" 300 50 100 50

printf 'echo\0hello\0' > "$tmpdir/tok_miss.bin"
printf 'gs\0' > "$tmpdir/tok_hit.bin"

verify_exit() {
  local label="$1" xdg="$2" lbuf="$3" tok_file="$4" expected="$5"
  local -i rc=0
  XDG_CONFIG_HOME="$xdg" "$zrs_bin" expand --lbuffer="$lbuf" --rbuffer="" < "$tok_file" >/dev/null 2>&1 || rc=$?
  if (( rc != expected )); then
    print -u2 "bench: sanity check failed for '$label': expected exit $expected, got $rc"
    exit 1
  fi
}

verify_exit "miss/none" "$tmpdir/none" "echo hello" "$tmpdir/tok_miss.bin" 2
verify_exit "miss/standard" "$tmpdir/standard" "echo hello" "$tmpdir/tok_miss.bin" 2
verify_exit "hit/small" "$tmpdir/small" "gs" "$tmpdir/tok_hit.bin" 0
verify_exit "hit/standard" "$tmpdir/standard" "gs" "$tmpdir/tok_hit.bin" 0
verify_exit "hit/large" "$tmpdir/large" "gs" "$tmpdir/tok_hit.bin" 0

print "Binary: $zrs_bin (warmup=${warmup}, runs=${runs})"
print ""
print "=== 1. Direct binary latency (hyperfine --shell=none) ==="

run_hf() {
  local label="$1" xdg="$2" cmd="$3" tok_file="${4:-}"
  local -a args=(
    --warmup "$warmup"
    --runs "$runs"
    --shell=none
    --ignore-failure
    --style=basic
    --command-name "$label"
  )
  [[ -n "$tok_file" ]] && args+=(--input "$tok_file")
  XDG_CONFIG_HOME="$xdg" hyperfine "${args[@]}" "$cmd"
}

run_hf "baseline: zrs --version"          "$tmpdir/none"     "$zrs_bin --version"
run_hf "expand miss [none]"               "$tmpdir/none"     "$zrs_bin expand --lbuffer='echo hello' --rbuffer=" "$tmpdir/tok_miss.bin"
run_hf "expand miss [small 4/1/2]"        "$tmpdir/small"    "$zrs_bin expand --lbuffer='echo hello' --rbuffer=" "$tmpdir/tok_miss.bin"
run_hf "expand miss [standard 100/30/20]" "$tmpdir/standard" "$zrs_bin expand --lbuffer='echo hello' --rbuffer=" "$tmpdir/tok_miss.bin"
run_hf "expand miss [large 300/100/50]"   "$tmpdir/large"    "$zrs_bin expand --lbuffer='echo hello' --rbuffer=" "$tmpdir/tok_miss.bin"
run_hf "expand hit(gs) [small 4/1/2]"     "$tmpdir/small"    "$zrs_bin expand --lbuffer=gs --rbuffer="           "$tmpdir/tok_hit.bin"
run_hf "expand hit(gs) [standard]"        "$tmpdir/standard" "$zrs_bin expand --lbuffer=gs --rbuffer="           "$tmpdir/tok_hit.bin"
run_hf "expand hit(gs) [large]"           "$tmpdir/large"    "$zrs_bin expand --lbuffer=gs --rbuffer="           "$tmpdir/tok_hit.bin"

bench_zsh_path() {
  local label="$1" xdg="$2" lbuf="$3" mode="$4" expected="$5"
  local -a tokens fields sorted times
  local response
  local -i i rc mid_idx p95_idx
  local -F t0 t1 dt

  local -x XDG_CONFIG_HOME="$xdg"
  local LBUFFER="$lbuf"
  local RBUFFER=""

  times=()
  for (( i = 0; i < warmup + runs; i++ )); do
    t0=$EPOCHREALTIME
    rc=0
    if [[ "$mode" == "version" ]]; then
      response=$(command "$zrs_bin" --version 2>&1) || rc=$?
    else
      tokens=("${(z)LBUFFER}")
      response=$(command "$zrs_bin" expand --lbuffer="$LBUFFER" --rbuffer="$RBUFFER" < <(printf '%s\0' "${tokens[@]}") 2>&1) || rc=$?
      if (( rc == 0 )); then
        fields=("${(@0)response}")
      fi
    fi
    t1=$EPOCHREALTIME

    if (( rc != expected )); then
      print -u2 "bench: '$label' returned exit $rc (expected $expected)"
      exit 1
    fi
    if (( i >= warmup )); then
      dt=$(( (t1 - t0) * 1000.0 ))
      times+=("$(printf '%.6f' "$dt")")
    fi
  done

  sorted=("${(n)times[@]}")
  mid_idx=$(( (runs + 1) / 2 ))
  p95_idx=$(( (runs * 95 + 99) / 100 ))
  (( p95_idx > runs )) && p95_idx=$runs

  printf "%-40s %7.2fms %7.2fms %7.2fms %7.2fms\n" \
    "$label" "${sorted[$mid_idx]}" "${sorted[$p95_idx]}" "${sorted[1]}" "${sorted[$runs]}"
}

print ""
print "=== 2. Running-Zsh end-to-end latency (_zrs_expand path) ==="
printf "%-40s %9s %9s %9s %9s\n" "Scenario" "median" "p95" "min" "max"
print -- "--------------------------------------------------------------------------------"
bench_zsh_path "zsh: \$(zrs --version) baseline"   "$tmpdir/none"     "echo hello" "version" 0
bench_zsh_path "zsh _zrs_expand miss [none]"        "$tmpdir/none"     "echo hello" "expand"  2
bench_zsh_path "zsh _zrs_expand miss [small]"       "$tmpdir/small"    "echo hello" "expand"  2
bench_zsh_path "zsh _zrs_expand miss [standard]"    "$tmpdir/standard" "echo hello" "expand"  2
bench_zsh_path "zsh _zrs_expand miss [large]"       "$tmpdir/large"    "echo hello" "expand"  2
bench_zsh_path "zsh _zrs_expand hit(gs) [small]"    "$tmpdir/small"    "gs"         "expand"  0
bench_zsh_path "zsh _zrs_expand hit(gs) [standard]" "$tmpdir/standard" "gs"         "expand"  0
bench_zsh_path "zsh _zrs_expand hit(gs) [large]"    "$tmpdir/large"    "gs"         "expand"  0
