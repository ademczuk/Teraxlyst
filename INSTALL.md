# Installing Teraxlyst

Teraxlyst is in early development. No tagged release yet. To run it you build from source.

## Prerequisites

- **Node 22+** and **pnpm 10** (https://pnpm.io/installation)
- **Rust stable** via `rustup` (https://rustup.rs/)
- Platform build tools (see below).

### macOS

- Xcode Command Line Tools: `xcode-select --install`

### Linux (Debian / Ubuntu)

```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  librsvg2-dev \
  libssl-dev
```

### Windows

- Visual Studio 2022 Build Tools with the C++ workload.
- WebView2 runtime is shipped with current Windows; older Win10 needs it installed separately.

## Build from source

```bash
git clone https://github.com/ademczuk/Teraxlyst
cd Teraxlyst
pnpm install
pnpm tauri dev   # for development with hot reload
# or
pnpm tauri build # for an unsigned release binary in src-tauri/target/release/bundle/
```

The first build downloads and compiles a few hundred Rust crates. Expect 5-15 minutes on a fast machine. Subsequent builds are incremental.

## Running an unsigned build

Teraxlyst ships unsigned by design. We do not pay for code-signing certificates. See `planning/SIGNING_PLAN.md` for the rationale and `docs/RELEASE.md` for what we ship instead (SHA256 checksums + Sigstore build provenance attestation, both free). On first launch the OS will warn; the bypass is one click per platform.

You can verify any GitHub Release bundle came from this repo's CI:

```bash
gh attestation verify --owner ademczuk <downloaded-file>
```

### macOS

The first time, right-click the `.app` and choose Open. Click Open again in the security dialog. After that, double-click works normally.

If `xattr` shows a quarantine flag and Gatekeeper still refuses:

```bash
xattr -dr com.apple.quarantine /Applications/Teraxlyst.app
```

### Windows

SmartScreen shows "Windows protected your PC" because the binary lacks code-signing reputation. Click "More info" then "Run anyway". The warning goes away once the binary builds reputation (a few hundred installs on the same signed cert).

### Linux

No warning. `.deb` and `.rpm` packages install via your distro tools. AppImage is `chmod +x` then run.

## Configuration

- **API keys** are stored in the OS keychain. Set them in Settings -> Models.
- **Workspace** is auto-selected on first run from your home directory.
- **Trackers** live in `<workspace>/.teraxlyst/trackers/*.yaml`. See `planning/M4_IMPLEMENTATION.md` (when published) for the schema.

## Troubleshooting

### `pnpm tauri dev` fails on Linux with "libwebkit2gtk-4.1.so not found"

You need the dev package, not just the runtime. Run the apt-get command above.

### `cargo build` fails on Windows with "link.exe not found"

Visual Studio Build Tools missing or not on PATH. Install from https://visualstudio.microsoft.com/visual-cpp-build-tools/ and select the "Desktop development with C++" workload.

### The app crashes on startup with "database locked"

A previous Teraxlyst process is still running (or didn't clean up). Find it in Task Manager / Activity Monitor / `ps aux | grep teraxlyst` and kill it. The DB uses single-instance locking; one process at a time per machine for v1.

### Shell integration not loading

The PTY backend writes shell integration scripts to `~/.cache/teraxlyst/shell-integration/` on first run. Delete that directory to force regeneration.

## Reporting issues

https://github.com/ademczuk/Teraxlyst/issues. Include OS, Teraxlyst version (from About), and steps to reproduce. For security issues see `SECURITY.md`.
