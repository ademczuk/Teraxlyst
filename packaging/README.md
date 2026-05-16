# Packaging manifests

Community-driven distribution templates. None of these require paid signing certs.

## Scoop (Windows)

`scoop/teraxlyst.json` is the manifest for the Scoop bucket. To publish:

1. Cut a Teraxlyst release as documented in `docs/RELEASE.md`. The release workflow uploads `Teraxlyst_<version>_x64_en-US.msi` and `SHA256SUMS-windows.txt`.
2. Copy `scoop/teraxlyst.json` into your `ademczuk/scoop-bucket` repo as `bucket/teraxlyst.json`.
3. Replace `PLACEHOLDER_SHA256_REPLACE_AT_RELEASE_TIME` with the actual SHA256 from `SHA256SUMS-windows.txt` for the MSI line.
4. Users then run:
   ```powershell
   scoop bucket add ademczuk https://github.com/ademczuk/scoop-bucket
   scoop install teraxlyst
   ```

The `autoupdate` block in the manifest means future releases get picked up by `scoop update` automatically once you bump the `version` field.

## Homebrew (macOS)

`homebrew/teraxlyst.rb` is a Cask formula. To publish:

1. Cut a release. The workflow uploads `Teraxlyst_<version>_aarch64.dmg` and `Teraxlyst_<version>_x64.dmg`.
2. Copy `homebrew/teraxlyst.rb` into your `ademczuk/homebrew-tap` repo as `Casks/teraxlyst.rb`.
3. Replace both SHA256 placeholders with the actual values from the macOS SHA256SUMS files.
4. Users then run:
   ```bash
   brew tap ademczuk/tap
   brew install --cask teraxlyst
   ```

The `postflight` block strips the macOS quarantine attribute so users don't have to do the right-click then Open dance. That's the same effect as `xattr -dr com.apple.quarantine` documented in INSTALL.md.

## AUR (Arch Linux) and Flathub

Not yet templated. Both are post-0.1.0 work. AUR is a PKGBUILD that wraps the AppImage; Flathub needs a manifest in `org.flathub.Teraxlyst` and a review pass.

## Why not the official package managers?

- **Microsoft Store** and **Mac App Store** both require paid developer programs ($99/year each) and a signed binary. Same blocker as cert-based code signing. Skip until install base justifies it.
- **WinGet** technically accepts unsigned binaries but the submission process is heavier than Scoop. Scoop is the right default for v0.1.
- **Snap Store** would work for Linux but the snapcraft build path adds CI minutes; AppImage covers the same use case with less friction.

The community-driven channels above all work fine without paid certs and without store reviews.
