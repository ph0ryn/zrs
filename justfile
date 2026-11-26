default:
  @just --list

fmt:
  cargo fmt

fmt-check:
  cargo fmt --check

lint:
  cargo clippy --all-targets -- -D warnings

test:
  cargo test

zsh-check:
  zsh -n shell/zrs.zsh
  zsh -n bench/run.zsh
  zsh -n tests/zsh/run.zsh

check: fmt-check lint test zsh-check

zsh-test:
  cargo build
  zsh tests/zsh/run.zsh

build:
  cargo build --release

# Measure release binary and running-zsh expand latency
bench *args:
  cargo build --release
  ./bench/run.zsh {{args}}

# Launch an isolated interactive zsh with the debug build of zrs
try config="examples/config.yaml":
  #!/usr/bin/env bash
  set -euo pipefail
  cargo build
  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT
  mkdir -p "$tmpdir/zdotdir" "$tmpdir/config/zrs-zsh"
  cp "{{config}}" "$tmpdir/config/zrs-zsh/config.yaml"
  cat > "$tmpdir/zdotdir/.zshrc" <<'EOF'
  autoload -Uz compinit && compinit -d "$ZDOTDIR/.zcompdump"
  PROMPT='[zrs-try] %~ %# '
  eval "$(zrs-zsh init)"
  EOF
  env \
    PATH="$PWD/target/debug:$PATH" \
    ZDOTDIR="$tmpdir/zdotdir" \
    XDG_CONFIG_HOME="$tmpdir/config" \
    zsh -i

nix-check:
  nix flake check

nix-build:
  nix build
