# Releasing Teraxlyst

Operator-facing checklist for cutting a release. The companion design doc is `planning/SIGNING_PLAN.md`. This file is the procedure.

## One-time setup

Done once per repo:

1. **Generate the minisign updater keypair** (free, no CA).
   ```bash
   pnpm tauri signer generate -w ~/.tauri/teraxlyst-updater.key
   ```
   Copy the printed public key. Paste it into `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`, replacing the empty string. Commit.

2. **Add GitHub Actions secrets** at https://github.com/ademczuk/Teraxlyst/settings/secrets/actions:
   - `TAURI_SIGNING_PRIVATE_KEY` = the contents of `~/.tauri/teraxlyst-updater.key`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` = the password set during generation

3. **Stash the private key offline.** Password manager, encrypted USB, paper backup, your call. If you lose it, you cannot ship signed updates ever again without bumping the embedded pubkey, which breaks the update path for everyone who already installed.

## Release candidate

```bash
# 1. Bump versions
#    src-tauri/tauri.conf.json -> "version": "0.1.0-rc1"
#    package.json              -> "version": "0.1.0-rc1"
#    src-tauri/Cargo.toml      -> version = "0.1.0-rc1"
#    Cargo.lock will rebuild on next cargo command

# 2. Confirm CI green on main
gh run list --branch main --limit 1

# 3. Commit + tag
git add src-tauri/tauri.conf.json package.json src-tauri/Cargo.toml
git commit -m "chore(release): bump to 0.1.0-rc1"
git tag v0.1.0-rc1
git push origin main
git push origin v0.1.0-rc1
```

The tag push fires `.github/workflows/release.yml`. It runs the 4-job matrix (macOS aarch64, macOS x86_64, ubuntu-22.04, windows-latest) and uploads bundles to a draft Release named `Teraxlyst v0.1.0-rc1`.

Watch it run:

```bash
gh run watch
# or open https://github.com/ademczuk/Teraxlyst/actions
```

Expect 25-40 minutes wall-clock for the full matrix. The cache hits make subsequent releases faster.

## Verifying a release candidate

Before promoting the draft to a published release:

1. **Download each platform's bundle from the draft release.** Linux deb + rpm + AppImage, Windows MSI + NSIS, macOS dmg for both architectures.

2. **Verify the SHA256SUMS file matches the bundles.**
   ```bash
   # Linux
   sha256sum -c SHA256SUMS-ubuntu-22.04.txt
   # macOS
   shasum -a 256 -c SHA256SUMS-macos-latest-aarch64-apple-darwin.txt
   # Windows (in PowerShell)
   Get-Content SHA256SUMS-windows.txt | ForEach-Object {
     $hash, $file = $_ -split '\s+', 2
     $actual = (Get-FileHash $file.Trim() -Algorithm SHA256).Hash.ToLower()
     if ($actual -ne $hash) { Write-Error "MISMATCH: $file" } else { "OK: $file" }
   }
   ```

3. **Verify the build provenance.**
   ```bash
   gh attestation verify --owner ademczuk teraxlyst_0.1.0-rc1_amd64.deb
   gh attestation verify --owner ademczuk Teraxlyst_0.1.0-rc1_x64-setup.exe
   ```
   Both should print "Verification succeeded".

4. **Install + smoke-test on each OS from a clean account.**
   - Workspace creation works
   - A `claude` session spawns and writes events to the DB
   - The settings window opens
   - The app survives a relaunch (DB persisted)

5. **Confirm the unsigned-binary bypass docs are accurate** by following INSTALL.md as if you'd never seen the app before. If the SmartScreen wording changed or macOS now requires a different right-click flow, fix the doc before publishing.

## Promoting to a real release

When the rc is green on all three platforms:

```bash
# Bump again, drop the -rc suffix
# Same files as before
git commit -m "chore(release): 0.1.0"
git tag v0.1.0
git push origin main
git push origin v0.1.0
```

When the workflow finishes:

1. Open the draft release `Teraxlyst v0.1.0` on GitHub.
2. Replace the auto-generated body with a hand-written CHANGELOG entry. The auto body is a placeholder.
3. Tick "Set as the latest release" only after the bundles are tested.
4. Hit Publish.

## Hotfixes after a release

If a published release has a bug:

1. Branch off the tag: `git checkout -b hotfix/0.1.1 v0.1.0`
2. Fix the bug. Tests green.
3. Bump to `0.1.1` in the three version files.
4. Merge to main, tag `v0.1.1`, push.

The auto-updater will offer the new version to anyone who installed the previous release. The minisign signature guarantees the update came from someone holding the offline private key, not from a compromised CI run.

## Rolling back a release

If a release is broken:

1. Mark the draft as a pre-release (so `latest.json` no longer points at it).
2. Or, if it already went out: tag a `v0.1.0.1` hotfix as above.
3. Add a note to INSTALL.md and the GitHub Release body warning users away from the broken bundle.

Do not delete the GitHub Release. Users may already have downloaded the bundles and the SHA256SUMS file; deleting the page would make their checksum verification fail without explanation.

## Adding paid-cert signing later

If we ever decide to spend the $179/year, the workflow already references the seven Apple secrets and two Windows cert secrets. Generate the certs per the procedures in `planning/SIGNING_PLAN.md`, add the secrets, and the next release picks them up automatically. The SmartScreen warning disappears once a few hundred installs accumulate on the signed binary; Gatekeeper accepts the notarized app immediately.

No code changes needed. Just secrets and a new tag.
