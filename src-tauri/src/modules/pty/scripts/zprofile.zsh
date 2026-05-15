# teraxlyst-shell-integration (zprofile)
#
# See zshenv.zsh for the rationale on the trailing `:`.
{
  _teraxlyst_user_zdotdir="${TERAXLYST_USER_ZDOTDIR:-$HOME}"
  [ -f "$_teraxlyst_user_zdotdir/.zprofile" ] && source "$_teraxlyst_user_zdotdir/.zprofile"
  unset _teraxlyst_user_zdotdir
}
:
