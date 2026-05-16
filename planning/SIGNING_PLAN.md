# Teraxlyst release + signing plan

The shipping plan for tagged releases. The original 2026-05 draft assumed paid CA certs ($179/year) and an Apple Developer Program enrollment ($99/year). That plan is parked. The active plan ships unsigned binaries plus free supply-chain attestation, which is the right trade for an early-access OSS desktop tool with no install base to protect.

## Active plan: no paid certs

The release pipeline at `.github/workflows/release.yml` fires on `v*` tag push and builds:

| Platform | Bundle | Signed? | Warning |
|---|---|---|---|
| Windows x64 | NSIS `.exe` installer | No (or self-signed) | SmartScreen "Windows protected your PC" until reputation builds |
| macOS Apple Silicon | `.app.tar.gz`, `.dmg` | Ad-hoc only | Right-click then Open on first launch |
| macOS Intel | `.app.tar.gz`, `.dmg` | Ad-hoc only | Right-click then Open on first launch |
| Linux x64 | `.deb`, `.rpm`, AppImage | No | None |

Plus, for every asset:

- A consolidated `SHA256SUMS-<platform>.txt` checksums file uploaded to the same GitHub Release.
- A GitHub Actions OIDC build provenance attestation via `actions/attest-build-provenance`. Verifiable by anyone with `gh attestation verify --owner ademczuk <file>`. This is Sigstore-backed, free, and gives a chain of custody from the source commit to the binary without involving a CA.

The release notes link to `INSTALL.md` for the per-platform "bypass the warning" instructions.

## Auto-updater (minisign, free)

Tauri's updater plugin signs releases with a minisign keypair. The keypair is generated locally and the public key is committed to `tauri.conf.json`. No CA. To enable:

```bash
pnpm tauri signer generate -w ~/.tauri/teraxlyst-updater.key
```

Two outputs:
- A private key file (path you specified). Store it in your password manager or an air-gapped USB stick. Never commit it.
- A public key string printed to stdout. Paste it into `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.

Then add these as GitHub Actions secrets so the release workflow can sign update artifacts:

| Secret | Source |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | contents of the private key file |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | password set during `signer generate` |

When both secrets are present, the workflow produces `latest.json` plus `.sig` files for each updater bundle. When they are absent, the workflow still produces the bundles, just without updater signatures. Users still get a working app; they just have to download new releases manually until the keypair lands.

## Status of paid-cert path

Parked. If we ever decide to spend money, the seven Apple secrets and two Windows cert secrets the workflow needs are all named in this section. The workflow already references them; they currently evaluate to empty strings and the steps are no-ops.

### macOS notarization secrets

| Secret | What it is |
|---|---|
| `APPLE_CERTIFICATE` | base64-encoded `.p12` of the Developer ID Application cert |
| `APPLE_CERTIFICATE_PASSWORD` | password for the `.p12` |
| `APPLE_SIGNING_IDENTITY` | string like `Developer ID Application: Your Name (TEAMID)` |
| `APPLE_TEAM_ID` | 10-character team identifier |
| `APPLE_API_ISSUER` | App Store Connect API issuer UUID |
| `APPLE_API_KEY` | App Store Connect API key id |
| `APPLE_API_KEY_PATH` | base64-encoded `.p8` private key |

Cost: $99/year. Skip until install base justifies it.

### Windows signing secrets

| Secret | What it is |
|---|---|
| `WINDOWS_CERTIFICATE` | base64-encoded `.pfx` |
| `WINDOWS_CERTIFICATE_PASSWORD` | password for the `.pfx` |

Cost: $80-500/year depending on CA. Azure Trusted Signing is the newer managed alternative at ~$10/month with Microsoft business verification. Skip until install base justifies it.

## What we get without paid certs

- **Provenance**: `gh attestation verify` proves the binary was built from a specific commit by GitHub Actions, not a random box. This is what supply-chain attacks try to fake; Sigstore makes it cheap to detect.
- **Integrity**: SHA256SUMS file lets users verify the download wasn't tampered with in transit.
- **Updates**: Minisign-signed updater artifacts. The app's embedded public key rejects any update not signed by the keypair holder. So even without a CA cert on the installer, the auto-update path is cryptographically guarded.

## What we don't get

- **SmartScreen reputation on Windows.** Users see the warning on first install. Workaround: a one-line "More info then Run anyway" instruction in INSTALL.md.
- **Gatekeeper pass on macOS.** Users do right-click then Open on first launch.
- **No Apple Store / Microsoft Store distribution.** Both require paid programs.

## Release process

See `docs/RELEASE.md` for the operator-facing checklist. Short version:

1. Tag the commit: `git tag v0.1.0 && git push origin v0.1.0`
2. Wait for the release workflow to finish (about 30 minutes wall-clock for the matrix).
3. Open the draft release on GitHub, edit the body, publish.
4. Test the unsigned bundles on each OS before announcing.

## 0.1.0 punch list (pre-release)

Items that block on this plan:

- [x] Replace `SIGNING_PLAN.md` to drop paid-cert assumption (this doc).
- [x] Move `.github/workflows-pending/release.yml` into `.github/workflows/` as the unsigned variant.
- [x] Add SHA256SUMS + Sigstore attestation steps.
- [x] Document INSTALL.md bypass instructions for each platform.
- [ ] Generate the minisign updater keypair, paste pubkey into `tauri.conf.json`, add the two secrets to the repo.
- [ ] First release candidate tag: `v0.1.0-rc1`. Verify all three platforms install and run from a clean machine.
- [ ] CHANGELOG draft.
- [ ] `v0.1.0` proper.

Items still needed at the feature level (independent of signing):

- [ ] Onboarding wizard: first-run workspace + provider setup.
- [ ] M4.1 inline tracker refs.
- [ ] M5.1 DiffInbox route.
- [ ] Real-Claude integration test enabled on Windows (needs WebView2Loader.dll copy for `tauri::test::mock_app`).

## Distribution channels (community, all free)

After the GitHub Release works:

- **Scoop bucket** (Windows power users): a JSON manifest in a separate repo `ademczuk/scoop-bucket` pointing at the GitHub Release. `scoop install teraxlyst`. No signing requirement.
- **Homebrew tap** (macOS power users): a Ruby formula in `ademczuk/homebrew-tap` pointing at the ad-hoc-signed `.app.tar.gz`. `brew install ademczuk/tap/teraxlyst`. No paid Apple program required.
- **AUR `teraxlyst-bin`** (Arch users): PKGBUILD that fetches the AppImage. No signing.
- **Flathub** (Linux desktop integration): higher friction, requires the app to pass freedesktop guidelines and a review. Worth doing after 0.1.0 lands.

None of these require paid certs.

## Cost summary

| Item | Annual cost (current plan) | Annual cost (paid-cert plan) |
|---|---|---|
| GitHub Actions | $0 (free tier) | $0 (free tier) |
| Apple Developer Program | $0 (skip) | $99 |
| Windows code-signing cert | $0 (skip) | $80 |
| Sigstore provenance | $0 (built into Actions) | $0 |
| Minisign updater keypair | $0 | $0 |
| **Total** | **$0** | **$179** |

The free-tier path is the realistic starting point. Reassess at 1,000+ active users.
