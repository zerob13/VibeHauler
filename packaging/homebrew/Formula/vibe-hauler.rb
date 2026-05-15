class VibeHauler < Formula
  desc "Local-first cleaner for AI agent clients"
  homepage "https://github.com/zerob13/VibeHauler"
  url "https://github.com/zerob13/VibeHauler/archive/refs/tags/v0.2.0.tar.gz"
  sha256 "REPLACE_WITH_SOURCE_TARBALL_SHA256"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/vibe-hauler")
  end

  test do
    assert_match "vhaul", shell_output("#{bin}/vhaul --version")
    assert_match "Launch the VibeHauler TUI", shell_output("#{bin}/vhaul --help")
  end
end
