# zrs ZLE integration script
# Usage: eval "$(zrs init)"

typeset -gi _ZRS_ENABLED=1

_zrs_toggle() {
  if (( _ZRS_ENABLED )); then
    _ZRS_ENABLED=0
    zle -M "zrs: abbreviation expansion disabled"
  else
    _ZRS_ENABLED=1
    zle -M "zrs: abbreviation expansion enabled"
  fi
}

zle -N _zrs_toggle
