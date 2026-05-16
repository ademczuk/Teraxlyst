cask "teraxlyst" do
  version "0.1.0"
  sha256 arm:   "PLACEHOLDER_ARM64_SHA256_REPLACE_AT_RELEASE_TIME",
         intel: "PLACEHOLDER_X86_64_SHA256_REPLACE_AT_RELEASE_TIME"

  url "https://github.com/ademczuk/Teraxlyst/releases/download/v#{version}/Teraxlyst_#{version}_#{Hardware::CPU.arm? ? 'aarch64' : 'x64'}.dmg",
      verified: "github.com/ademczuk/Teraxlyst/"
  name "Teraxlyst"
  desc "Desktop workspace for AI coding agents (Tauri 2 + Rust + React)"
  homepage "https://github.com/ademczuk/Teraxlyst"

  livecheck do
    url :url
    strategy :github_latest
  end

  app "Teraxlyst.app"

  # Unsigned binary, so strip the quarantine attribute on first install.
  # Same effect as the user running `xattr -dr com.apple.quarantine`,
  # documented in INSTALL.md.
  postflight do
    system_command "/usr/bin/xattr",
                   args: ["-dr", "com.apple.quarantine", "#{appdir}/Teraxlyst.app"],
                   sudo: false
  end

  zap trash: [
    "~/Library/Application Support/com.ademczuk.teraxlyst",
    "~/Library/Caches/com.ademczuk.teraxlyst",
    "~/Library/Preferences/com.ademczuk.teraxlyst.plist",
    "~/Library/Saved Application State/com.ademczuk.teraxlyst.savedState",
    "~/Library/Logs/Teraxlyst",
  ]
end
