# Teraxlyst doctor (Windows PowerShell variant). Run from the repo root:
#   pwsh scripts\doctor.ps1
# Or:
#   powershell -ExecutionPolicy Bypass -File scripts\doctor.ps1

$ErrorActionPreference = 'Continue'

$script:ok = 0
$script:warn = 0
$script:fail = 0

function Test-Cmd {
    param([string]$Label, [string]$Cmd, [string]$Want, [string]$Hint = '')
    $exists = Get-Command $Cmd -ErrorAction SilentlyContinue
    if (-not $exists) {
        Write-Host ("MISS  {0} ({1} not found)" -f $Label, $Cmd) -ForegroundColor Red
        if ($Hint) { Write-Host ("      hint: {0}" -f $Hint) }
        $script:fail++
        return
    }
    try {
        $got = & $Cmd --version 2>&1 | Select-Object -First 1
        Write-Host ("OK    {0}: {1} (want {2})" -f $Label, $got, $Want) -ForegroundColor Green
        $script:ok++
    } catch {
        Write-Host ("WARN  {0} found but --version failed: {1}" -f $Label, $_.Exception.Message) -ForegroundColor Yellow
        $script:warn++
    }
}

function Test-Optional {
    param([string]$Label, [string]$Cmd, [string]$Hint = '')
    $exists = Get-Command $Cmd -ErrorAction SilentlyContinue
    if (-not $exists) {
        Write-Host ("WARN  {0} ({1} not found, optional)" -f $Label, $Cmd) -ForegroundColor Yellow
        if ($Hint) { Write-Host ("      hint: {0}" -f $Hint) }
        $script:warn++
        return
    }
    $got = & $Cmd --version 2>&1 | Select-Object -First 1
    Write-Host ("OK    {0}: {1}" -f $Label, $got) -ForegroundColor Green
    $script:ok++
}

Write-Host "Teraxlyst doctor (Windows variant)"
Write-Host ""

Test-Cmd "Node"  "node"  ">= 22"  "winget install OpenJS.NodeJS.LTS"
Test-Cmd "pnpm"  "pnpm"  ">= 10"  "npm install -g pnpm@10  (or corepack enable)"
Test-Cmd "Rust"  "rustc" "stable" "Download rustup-init.exe from https://rustup.rs/"
Test-Cmd "Cargo" "cargo" "stable" "comes with rustup"
Test-Cmd "Git"   "git"   ">= 2.30" "winget install Git.Git"

Write-Host ""
Write-Host "Optional but recommended:"
Test-Optional "GitHub CLI" "gh" "winget install GitHub.cli"
Test-Optional "Python 3.10+" "python" "winget install Python.Python.3.12"

# MSVC linker check
Write-Host ""
Write-Host "Windows MSVC toolchain (required by Rust on Windows):"
$cl = Get-Command cl -ErrorAction SilentlyContinue
if (-not $cl) {
    # Try to find VS Build Tools via vswhere
    $vswhere = "$env:ProgramFiles (x86)\Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path $vswhere) {
        $vsPath = & $vswhere -latest -products * -requires Microsoft.VisualCpp.Tools.HostX64.TargetX64 -property installationPath 2>$null
        if ($vsPath) {
            Write-Host ("OK    MSVC C++ tools found via vswhere: {0}" -f $vsPath) -ForegroundColor Green
            Write-Host "      Open a 'Developer PowerShell for VS' to put cl.exe on PATH"
            $script:ok++
        } else {
            Write-Host "MISS  MSVC C++ build tools not found" -ForegroundColor Red
            Write-Host "      hint: install 'Build Tools for Visual Studio' with the 'Desktop development with C++' workload"
            Write-Host "      hint: https://visualstudio.microsoft.com/visual-cpp-build-tools/"
            $script:fail++
        }
    } else {
        Write-Host "MISS  cl.exe not on PATH and vswhere not found" -ForegroundColor Red
        Write-Host "      hint: install 'Build Tools for Visual Studio' with the C++ workload"
        $script:fail++
    }
} else {
    Write-Host ("OK    cl.exe on PATH: {0}" -f $cl.Source) -ForegroundColor Green
    $script:ok++
}

# WebView2 runtime
$webview2Key = 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
if (Test-Path $webview2Key) {
    $ver = (Get-ItemProperty $webview2Key -ErrorAction SilentlyContinue).pv
    if ($ver) {
        Write-Host ("OK    WebView2 runtime: {0}" -f $ver) -ForegroundColor Green
        $script:ok++
    } else {
        Write-Host "WARN  WebView2 runtime key present but no version" -ForegroundColor Yellow
        $script:warn++
    }
} else {
    Write-Host "WARN  WebView2 runtime not detected" -ForegroundColor Yellow
    Write-Host "      Modern Windows usually ships this. Install from https://developer.microsoft.com/en-us/microsoft-edge/webview2/ if Tauri dev fails."
    $script:warn++
}

# Repo-local checks
Write-Host ""
if (Test-Path "src-tauri/Cargo.lock") {
    Write-Host "OK    src-tauri/Cargo.lock is tracked (deterministic builds enabled)" -ForegroundColor Green
    $script:ok++
} else {
    Write-Host "MISS  src-tauri/Cargo.lock not found" -ForegroundColor Red
    Write-Host "      hint: trigger the 'Refresh Cargo.lock' workflow from the GitHub Actions UI"
    $script:fail++
}

$hookpath = git config --get core.hooksPath 2>$null
Write-Host ""
if ($hookpath -eq '.githooks') {
    Write-Host "OK    pre-commit hooks active (.githooks)" -ForegroundColor Green
    $script:ok++
} else {
    Write-Host "WARN  pre-commit hooks NOT active" -ForegroundColor Yellow
    Write-Host "      hint: git config core.hooksPath .githooks  (one-time setup)"
    $script:warn++
}

# Summary
Write-Host ""
Write-Host "----"
Write-Host ("ok={0} warn={1} fail={2}" -f $script:ok, $script:warn, $script:fail)
if ($script:fail -gt 0) {
    Write-Host ""
    Write-Host ("{0} critical check(s) failed. Fix the hints above before running 'pnpm tauri dev'." -f $script:fail) -ForegroundColor Red
    exit 1
}
if ($script:warn -gt 0) {
    Write-Host ""
    Write-Host ("{0} warning(s). The build should work but some workflows may not." -f $script:warn) -ForegroundColor Yellow
}
exit 0
