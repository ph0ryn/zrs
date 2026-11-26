# zrs ZLE integration script
# Usage: eval "$(zrs-zsh init)"

typeset -gi _ZRS_ENABLED=${_ZRS_ENABLED:-1}
typeset -gA _zrs_previous

_zrs_apply() {
  local response
  local -a fields
  local -i result
  response=$(command zrs-zsh "$@" --lbuffer="$LBUFFER" --rbuffer="$RBUFFER" 2>&1)
  result=$?
  case $result in
    0)
      fields=("${(@0)response}")
      if (( $#fields != 4 )) || [[ -n $fields[4] ||
        ( $fields[1] != normal && $fields[1] != placeholder ) ||
        $fields[3] != <-> ]] || (( fields[3] > ${#fields[2]} )); then
        zle -M 'zrs-zsh: invalid buffer update'
        return 1
      fi
      BUFFER=$fields[2]
      CURSOR=$fields[3]
      _zrs_kind=$fields[1]
      ;;
    2|3) ;;
    *) zle -M "${response:-zrs-zsh: command failed (status $result)}" ;;
  esac
  return $result
}

_zrs_fallback() {
  local entry="$KEYMAP:$1"
  zle "${_zrs_previous[$entry]:-$2}" -w
}

_zrs_expand() {
  (( _ZRS_ENABLED )) || return 2
  [[ -n $LBUFFER && $LBUFFER != *[[:space:]] ]] || return 2
  local -a tokens
  tokens=("${(z)LBUFFER}")
  (( $#tokens )) || return 2
  _zrs_apply expand < <(printf '%s\0' "${tokens[@]}")
}

_zrs_space() {
  emulate -L zsh
  local _zrs_kind
  local -i result
  _zrs_expand
  result=$?
  if (( result == 2 )) || { (( result == 0 )) && [[ $_zrs_kind == normal ]]; }; then
    _zrs_fallback ' ' self-insert
  fi
}

_zrs_enter() {
  emulate -L zsh
  local _zrs_kind
  local -i result
  _zrs_expand
  result=$?
  if (( result == 2 )) || { (( result == 0 )) && [[ $_zrs_kind == normal ]]; }; then
    _zrs_fallback '^M' accept-line
  fi
}

_zrs_complete() {
  emulate -L zsh
  local _zrs_kind word=
  local -i result
  local -a tokens
  if [[ $LBUFFER != *[[:space:]] ]]; then
    tokens=("${(z)LBUFFER}")
    word=${tokens[-1]}
    # (z) normalizes some operators, including newlines, to semicolons.
    [[ $LBUFFER == *"$word" ]] || word=
  fi
  _zrs_apply complete --word="$word" --query="${(Q)word}"
  result=$?
  case $result in
    2) _zrs_fallback '^I' expand-or-complete ;;
    0|3) zle reset-prompt ;;
  esac
}

_zrs_snippet() {
  emulate -L zsh
  local _zrs_kind
  _zrs_apply snippet
  case $? in
    0|3) zle reset-prompt ;;
  esac
}

_zrs_history() {
  emulate -L zsh
  local _zrs_kind
  zmodload -F zsh/parameter p:history
  (( $#history )) || return 0
  _zrs_apply history < <(printf '%s\0' "${(@)history}")
  case $? in
    0|3) zle reset-prompt ;;
  esac
}

_zrs_placeholder() {
  emulate -L zsh
  local _zrs_kind
  _zrs_apply placeholder
}

_zrs_literal_space() {
  LBUFFER+=' '
}

_zrs_toggle() {
  if (( _ZRS_ENABLED )); then
    _ZRS_ENABLED=0
    zle -M 'zrs-zsh: abbreviation expansion disabled'
  else
    _ZRS_ENABLED=1
    zle -M 'zrs-zsh: abbreviation expansion enabled'
  fi
}

_zrs_bind_defaults() {
  emulate -L zsh
  local key widget map entry binding
  local -a fields
  local -a maps=(main emacs viins)
  local -A defaults=(' ' _zrs_space '^M' _zrs_enter '^I' _zrs_complete '^R' _zrs_history)

  local -a keys=( "${(@k)defaults}" ) bindings
  # Snapshot all bindings before updating possibly aliased maps.
  bindings=( "${(@f)$(for map in $maps; do
    for key in "${keys[@]}"; do
      bindkey -M "$map" "$key" || print -r -- ""
    done
  done)}" )
  local -i index=0
  for map in $maps; do
    for key in "${keys[@]}"; do
      entry="$map:$key"
      binding=${bindings[++index]-}
      [[ -n $binding ]] || continue
      fields=("${(z)binding}")
      widget=${(Q)fields[2]}
      if [[ -n $widget && $widget != _zrs_* ]]; then
        _zrs_previous[$entry]=$widget
      fi
    done
  done

  for map in $maps; do
    for key in "${keys[@]}"; do
      bindkey -M "$map" "$key" "$defaults[$key]"
    done
    bindkey -M "$map" '^X ' _zrs_literal_space
    bindkey -M "$map" '^Xs' _zrs_snippet
    bindkey -M "$map" '^Xn' _zrs_placeholder
    bindkey -M "$map" '^Xt' _zrs_toggle
  done
}

zle -N _zrs_space
zle -N _zrs_enter
zle -N _zrs_complete
zle -N _zrs_history
zle -N _zrs_snippet
zle -N _zrs_placeholder
zle -N _zrs_literal_space
zle -N _zrs_toggle
