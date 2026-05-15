# Teraxlyst signing + release plan (M6)

What is required to ship signed installers on macOS and Windows, and the steps that block first release. Captured here so the v0.1.0 release work has a concrete checklist instead of guesswork.

## TL;DR

Three external dependencies block a signed release:

1. **Apple Developer Program membership** ($99/year) for macOS notarization.
2. **Windows code-signing certificate** ($200-500/year from Sectigo or DigiCert, or free EV via a hardware token).
3. **Tauri updater minisign keypair** (free, but the private key must be generated and stored securely offline).

Until those exist, Teraxlyst releases are unsigned. Users get Gatekeeper and SmartScreen warnings. That is acceptable for early-access binaries; document it in the release notes.

## macOS

### Notarization (the goal)

Tauri's `tauri-action` GitHub Action handles macOS signing + notarization automatically when these secrets are present:

| Secret name | What it is |
|---|---|
| `APPLE_CERTIFICATE` | base64-encoded `.p12` of the Developer ID Application cert |
| `APPLE_CERTIFICATE_PASSWORD` | password for the `.p12` |
| `APPLE_SIGNING_IDENTITY` | string like `Developer ID Application: Your Name (TEAMID)` |
| `APPLE_TEAM_ID` | 10-character team identifier |
| `APPLE_API_ISSUER` | App Store Connect API issuer UUID |
| `APPLE_API_KEY` | App Store Connect API key id |
| `APPLE_API_KEY_PATH` | base64-encoded `.p8` private key |

### Steps to get there

1. Join the Apple Developer Program (https://developer.apple.com/programs/). Costs $99/year.
2. In Xcode -> Settings -> Accounts, sign in. Create a Developer ID Application certificate.
3. Export the cert as a `.p12` from Keychain Access. Set a strong password.
4. Create an App Store Connect API key with the "Developer" role. Download the `.p8` file.
5. Run `base64 -i AuthKey_XXX.p8 | pbcopy` to encode the key for GitHub secret storage.
6. Add all seven secrets to the repo settings (Settings -> Secrets and variables -> Actions).
7. Restore `.github/workflows-pending/release.yml` to `.github/workflows/release.yml`. It already references these secrets.

### Unsigned build escape hatch

Until the developer program is paid for, Tauri can produce unsigned `.app` bundles. Users run them with right-click -> Open the first time to bypass Gatekeeper. Document this in the README install section.

## Windows

### Code signing (the goal)

For SmartScreen reputation, the binary must be signed with an Extended Validation (EV) code-signing certificate. Standard OV certificates work for signing but do not get an immediate SmartScreen reputation; users still see a warning until the binary builds reputation via downloads.

| Option | Cost | Trade-offs |
|---|---|---|
| EV cert from Sectigo / DigiCert | $300-500/year | Hardware token required, instant SmartScreen reputation |
| OV cert from Certum / SSL.com | $80-200/year | Software cert, slow SmartScreen reputation buildup |
| Free Azure Trusted Signing (new 2024) | $9.99/month | Requires Microsoft business verification |

Tauri's release workflow accepts:

| Secret name | What it is |
|---|---|
| `WINDOWS_CERTIFICATE` | base64-encoded `.pfx` |
| `WINDOWS_CERTIFICATE_PASSWORD` | password for the `.pfx` |

### Steps to get there

1. Purchase an OV certificate from Certum (cheapest non-token option, but slow SmartScreen reputation). Or Azure Trusted Signing for a managed flow.
2. Download the `.pfx`.
3. Encode + add to GitHub secrets.
4. Tauri picks it up automatically during the windows-latest matrix job.

### Unsigned escape hatch

NSIS installer works unsigned. SmartScreen shows "Windows protected your PC" the first few hundred downloads. Users click "More info" -> "Run anyway". Document in README.

## Auto-updater (minisign keypair)

Tauri's updater plugin verifies new releases against a public key embedded in the app. Generation:

```bash
pnpm tauri signer generate -w ~/.tauri/teraxlyst-updater.key
```

This produces:
- A private key at the path you specified. Store it offline (encrypted password manager, hardware key, or air-gapped USB). Do NOT commit to the repo.
- A public key printed to stdout. Embed this in `tauri.conf.json` under `plugins.updater.pubkey`.

The release workflow then needs:

| Secret name | What it is |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | content of the private key file |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | password set during generation |

Once the keypair exists and the public key is embedded:

1. Restore the updater config in `src-tauri/tauri.conf.json` (removed in M0 commit).
2. Point `endpoints` at `https://github.com/ademczuk/Teraxlyst/releases/latest/download/latest.json`.
3. Tauri's release workflow generates `latest.json` automatically during the build.

## Linux

No signing required for `.deb` / `.rpm` / AppImage. Users may verify the binary via a checksums file published alongside the release. Trivial to add to the release workflow.

The AUR `teraxlyst-bin` package is currently referenced as broken in `UpdaterDialog.tsx` (we inherited the install command from upstream). M6.1 task: create an actual AUR package and update the displayed command, or short-circuit the Linux manual-update path to "Download from GitHub Releases" instead.

## Release workflow restoration

After secrets and keypair are in place:

1. `git mv .github/workflows-pending/release.yml .github/workflows/release.yml`
2. Update the `releaseName` field from `"Terax ${version}"` to `"Teraxlyst ${version}"` (already noted in `planning/M0_TODO.md`).
3. Push a `v0.1.0` tag. The release workflow fires on `v*` tag push.
4. The workflow produces signed installers for macOS (aarch64 + x86_64), Linux (deb + rpm + AppImage), and Windows (MSI + NSIS). All uploaded as draft release assets.
5. Edit the draft release notes, publish.

## 0.1.0 punch list (excluding signing dependencies)

Items I can land before any external signing setup:

- [ ] Lockfile refresh: run `pnpm install` and commit the resulting `pnpm-lock.yaml`. Then re-enable `--frozen-lockfile` in CI.
- [ ] Cargo.lock commit: run `cargo generate-lockfile` in src-tauri, commit. Re-enable `--locked` in CI.
- [ ] Real Claude Code adapter: verify the actual CLI invocation and stdout format. Replace the M2 stub parser with proper tool-call / permission-prompt detection.
- [ ] M2.1 watcher-to-DB sync: fix the Completed-marker race the M2 test currently papers over.
- [ ] M5.1 DiffInbox panel: wire `DiffInbox` into a route or panel in App.tsx.
- [ ] M4.1 inline tracker refs: markdown extension that renders `#tracker[BUG-014]` as a pill.
- [ ] Onboarding wizard: first-run flow to create a workspace, configure a provider, smoke-test.
- [ ] README install instructions covering all three platforms with the unsigned-binary escape hatches.
- [ ] CHANGELOG cleanup: rewrite as a user-facing release-notes draft.

Items that block on signing setup:

- [ ] Apple Developer Program enrollment.
- [ ] Windows code-signing certificate (or Azure Trusted Signing).
- [ ] Minisign keypair generation and secret storage.
- [ ] Restore `release.yml` to `.github/workflows/`.
- [ ] Restore updater pubkey + endpoint in `tauri.conf.json`.
- [ ] First tagged release.

## Cost summary

| Item | Annual cost |
|---|---|
| Apple Developer Program | $99 |
| Windows OV cert (Certum) | ~$80 |
| GitHub Actions minutes | $0 within free tier for now |
| Total to ship signed installers | ~$179/year |

EV cert + Azure Trusted Signing variants push this higher. Free tier is the realistic starting point for a one-person early-access project.
