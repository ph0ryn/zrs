#!/usr/bin/env zsh
set -euo pipefail

script_dir="${0:A:h}"
repo_root="${script_dir:h:h}"

zsh -n "${repo_root}/shell/zrs.zsh"
print "zsh syntax check passed"
