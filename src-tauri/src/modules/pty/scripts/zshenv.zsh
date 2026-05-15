# teraxlyst-shell-integration (zshenv)
#
# Trailing `:` is load-bearing - without it, a missing user .zshenv leaves $?=1,
# which propagates through the rest of init and ultimately into the first
# prompt's `%?` (rendering robbyrussell's prompt arrow red on a clean shell start).
{
  _teraxlyst_user_zdotdir="${TERAXLYST_USER_ZDOTDIR:-$HOME}"
  [ -f "$_teraxlyst_user_zdotdir/.zshenv" ] && source "$_teraxlyst_user_zdotdir/.zshenv"
  unset _teraxlyst_user_zdotdir
}
:
