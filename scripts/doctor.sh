#!/usr/bin/env bash
# Teraxlyst doctor: verify build prerequisites are in place before
# someone tries to `pnpm tauri dev` and hits a confusing platform-
# specific failure.

set -u

# Color helpers (skip if not a tty)
if [ -t 1 ]; then
  GREEN='\033[0;32m'
  RED='\033[0;31m'
  YELLOW='\033[1;33m'
  RESET='\033[0m'
else
  GREEN='' RED='' YELLOW='' RESET=''
fi

ok=0
warn=0
fail=0

check() {
  local label="$1"
  local cmd="$2"
  local want="$3"
  local hint="${4:-}"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    printf "${RED}MISS${RESET}  %s (%s not found)\n" "$label" "$cmd"
    [ -n "$hint" ] && printf "      hint: %s\n" "$hint"
    fail=$((fail+1))
    return
  fi
  local got
  got=$("$cmd" --version 2>&1 | head -1)
  printf "${GREEN}OK${RESET}    %s: %s (want %s)\n" "$label" "$got" "$want"
  ok=$((ok+1))
}

check_optional() {
  local label="$1"
  local cmd="$2"
  local hint="${3:-}"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    printf "${YELLOW}WARN${RESET}  %s ($cmd not found, optional)\n" "$label"
    [ -n "$hint" ] && printf "      hint: %s\n" "$hint"
    warn=$((warn+1))
    return
  fi
  local got
  got=$("$cmd" --version 2>&1 | head -1)
  printf "${GREEN}OK${RESET}    %s: %s\n" "$label" "$got"
  ok=$((ok+1))
}

echo "Teraxlyst doctor (Linux/macOS variant)"
echo

check "Node"   node   ">= 22"   "install from https://nodejs.org/ or via nvm"
check "pnpm"   pnpm   ">= 10"   "npm install -g pnpm@10 (or use corepack enable)"
check "Rust"   rustc  "stable"  "curl --proto =https --tlsv1.2 -sSf https://sh.rustup.rs | sh"
check "Cargo"  cargo  "stable"  "comes with rustup"
check "Git"    git    ">= 2.30" "install from your package manager"

echo
echo "Optional but recommended:"
check_optional "GitHub CLI"  gh   "useful for running the refresh-cargo-lock workflow: gh workflow run refresh-cargo-lock.yml"
check_optional "Python 3.10+" python3 "used by the AI-marker scan script"

# Platform-specific webview deps
echo
case "$(uname -s)" in
  Linux)
    echo "Linux system libraries (Tauri 2 / webkit2gtk 4.1):"
    for pkg in libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libssl-dev; do
      if dpkg -s "$pkg" >/dev/null 2>&1 || rpm -q "$pkg" >/dev/null 2>&1; then
        printf "${GREEN}OK${RESET}    %s installed\n" "$pkg"
        ok=$((ok+1))
      else
        printf "${YELLOW}WARN${RESET}  %s not detected (may use a different package name on your distro)\n" "$pkg"
        warn=$((warn+1))
      fi
    done
    ;;
  Darwin)
    echo "macOS toolchain (Xcode Command Line Tools):"
    if xcode-select -p >/dev/null 2>&1; then
      printf "${GREEN}OK${RESET}    Xcode CLT: $(xcode-select -p)\n"
      ok=$((ok+1))
    else
      printf "${RED}MISS${RESET}  Xcode Command Line Tools\n"
      printf "      hint: xcode-select --install\n"
      fail=$((fail+1))
    fi
    ;;
  MINGW*|MSYS*|CYGWIN*)
    echo "On Windows, use scripts/doctor.ps1 from PowerShell for the MSVC checks."
    ;;
  *)
    echo "(unknown platform $(uname -s), skipping platform-specific checks)"
    ;;
esac

# Repo-local check
echo
if [ -f "src-tauri/Cargo.lock" ]; then
  printf "${GREEN}OK${RESET}    src-tauri/Cargo.lock is tracked (deterministic builds enabled)\n"
  ok=$((ok+1))
else
  printf "${RED}MISS${RESET}  src-tauri/Cargo.lock not found\n"
  printf "      hint: trigger the 'Refresh Cargo.lock' workflow once from the GitHub Actions UI\n"
  fail=$((fail+1))
fi

# Pre-commit hook activation
hookpath=$(git config --get core.hooksPath 2>/dev/null || true)
echo
if [ "$hookpath" = ".githooks" ]; then
  printf "${GREEN}OK${RESET}    pre-commit hooks active (.githooks)\n"
  ok=$((ok+1))
else
  printf "${YELLOW}WARN${RESET}  pre-commit hooks NOT active\n"
  printf "      hint: git config core.hooksPath .githooks  (one-time setup)\n"
  warn=$((warn+1))
fi

# Summary
echo
echo "----"
echo "ok=$ok warn=$warn fail=$fail"
if [ $fail -gt 0 ]; then
  echo
  echo "$fail critical check(s) failed. Fix the hints above before running 'pnpm tauri dev'."
  exit 1
fi
if [ $warn -gt 0 ]; then
  echo
  echo "$warn warning(s). The build should work but some workflows may not."
fi
exit 0
