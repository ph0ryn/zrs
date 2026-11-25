#!/usr/bin/env zsh
set -eu

zmodload zsh/zpty
zmodload zsh/zselect

repo_root=${0:A:h:h:h}
binary=${1:-$repo_root/target/debug/zrs}
export PATH="${binary:A:h}:$PATH"
export ZRS_TEST_DIR=$(mktemp -d)
export ZDOTDIR=$ZRS_TEST_DIR
export XDG_CONFIG_HOME=$ZRS_TEST_DIR
export TERM=xterm-256color
export LC_ALL=en_US.UTF-8
trap 'zpty -d test 2>/dev/null || true; rm -rf -- "$ZRS_TEST_DIR"' EXIT

mkdir "$ZRS_TEST_DIR/zrs"
cat > "$ZRS_TEST_DIR/zrs/config.yaml" <<'YAML'
abbreviations:
  - keyword: gs
    expand: git status
  - keyword: gc
    expand: 'git commit -m {{message}} {{file}}'
  - keyword: bad
    expand: 'exit 5'
    evaluate: true
  - keyword: jp
    expand: 'print 日本{{}}語'
  - keyword: done
    expand: 'print expanded > "$ZRS_TEST_DIR/executed"'
snippets:
  - name: print snippet
    body: 'print {{text}}'
completions:
  - name: branch
    pattern: '^pick '
    source: >-
      printf '%s\0' 'branch/one' $'branch two\nline'
    preview: 'printf "%s" {} > "$ZRS_TEST_DIR/preview"'
    multi: true
  - name: empty
    pattern: '^empty '
    source: ':'
YAML

zrs init --no-keybinds > "$ZRS_TEST_DIR/widgets.zsh"
zrs init > "$ZRS_TEST_DIR/init.zsh"
cat > "$ZRS_TEST_DIR/start.zsh" <<'ZSH'
PS1='zrs-test> '
stty rows 12 cols 100
bindkey -e
_test_tab() { LBUFFER+='[fallback]'; }
_test_clear() { BUFFER=; }
_test_space() {
  print -r -- "$WIDGET" > "$ZRS_TEST_DIR/space-widget"
  zle .self-insert
}
_test_enter() {
  print -r -- "$WIDGET" > "$ZRS_TEST_DIR/enter-widget"
  zle .accept-line
}
_test_snapshot() { print -r -- "$BUFFER|$CURSOR" > "$ZRS_TEST_DIR/snapshot"; }
zle -N _test_tab
zle -N _test_clear
zle -N _test_space
zle -N _test_enter
zle -N _test_snapshot
bindkey '^I' _test_tab
bindkey '^U' _test_clear
bindkey ' ' _test_space
bindkey '^M' _test_enter
bindkey '^G' _test_snapshot
source "$ZRS_TEST_DIR/widgets.zsh"
[[ $(bindkey '^I') == *'_test_tab' ]] || exit 90
source "$ZRS_TEST_DIR/init.zsh"
source "$ZRS_TEST_DIR/init.zsh"
print ready > "$ZRS_TEST_DIR/ready"
ZSH

zpty -b test zsh -dfi
zpty -w test 'source "$ZRS_TEST_DIR/start.zsh"'

drain() {
  local chunk
  while zpty -r test chunk; do
    print -rn -- "$chunk" >> "$ZRS_TEST_DIR/terminal"
  done
}

wait_for() {
  local file=$1 pattern=$2
  local -i attempt
  for attempt in {1..150}; do
    drain
    if [[ -f $file && $(<"$file") == *"$pattern"* ]]; then
      return
    fi
    zselect -t 5 || true
  done
  print -ru2 -- "Timed out waiting for $pattern"
  cat "$ZRS_TEST_DIR/terminal" >&2
  exit 1
}

snapshot() {
  local keys=$1 expected=$2 actual
  rm -f "$ZRS_TEST_DIR/snapshot"
  zpty -w -n test "$keys"$'\x07'
  wait_for "$ZRS_TEST_DIR/snapshot" '|'
  actual=$(<"$ZRS_TEST_DIR/snapshot")
  if [[ $actual != "$expected" ]]; then
    print -ru2 -- "Expected <$expected>, got <$actual>"
    cat "$ZRS_TEST_DIR/terminal" >&2
    exit 1
  fi
}

wait_for "$ZRS_TEST_DIR/ready" ready
snapshot 'gs ' 'git status |11'
wait_for "$ZRS_TEST_DIR/space-widget" '_test_space'
snapshot $'\x15echo gs ' 'echo gs |8'
snapshot $'\x15gc\r' 'git commit -m  {{file}}|14'
snapshot $'\x18n' 'git commit -m  |15'
snapshot $'\x15jp ' 'print 日本語|8'
snapshot $'\x15bad ' 'bad|3'
snapshot $'\x15unknown\t' 'unknown[fallback]|17'
snapshot $'\x15empty \t' 'empty |6'
snapshot $'\x15\x18tgs ' 'gs |3'
snapshot $'\x15\x18tgs\x18 ' 'gs |3'

finish_fzf() {
  zpty -w -n test "$1"
  wait_for "$ZRS_TEST_DIR/terminal" $'\e[?1049l'
  snapshot '' "$2"
}

: > "$ZRS_TEST_DIR/terminal"
zpty -w -n test $'\x15\x18s'
wait_for "$ZRS_TEST_DIR/terminal" 'print snippet'
finish_fzf $'\r' 'print |6'

: > "$ZRS_TEST_DIR/terminal"
zpty -w -n test $'\x15pick bra\t'
wait_for "$ZRS_TEST_DIR/terminal" 'nch/one'
wait_for "$ZRS_TEST_DIR/preview" 'branch/one'
zpty -w -n test $'\t'
wait_for "$ZRS_TEST_DIR/terminal" '2/2 (1)'
zpty -w -n test $'\e[A\t'
wait_for "$ZRS_TEST_DIR/terminal" '2/2 (2)'
finish_fzf $'\r' "pick branch/one 'branch two"$'\n'"line'|33"

: > "$ZRS_TEST_DIR/terminal"
zpty -w -n test $'\x15pick "bra"\t'
wait_for "$ZRS_TEST_DIR/terminal" 'nch/one'
finish_fzf $'\e' 'pick "bra"|10'

zpty -w -n test $'\x15done\r'
wait_for "$ZRS_TEST_DIR/executed" expanded
wait_for "$ZRS_TEST_DIR/enter-widget" '_test_enter'

print 'ZLE and fzf checks passed'
