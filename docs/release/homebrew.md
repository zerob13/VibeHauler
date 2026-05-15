# Homebrew Release

Goal:

```bash
brew install vibehauler/tap/vibe-hauler
vhaul scan
```

## Recommended Channel

Start with a custom tap:

```txt
github.com/vibehauler/homebrew-tap
└── Formula/
    └── vibe-hauler.rb
```

Later, consider `homebrew/core` only after the project has stable releases, demand, tests, and accepted naming.

## Formula Name

Formula file:

```txt
Formula/vibe-hauler.rb
```

Formula class:

```ruby
class VibeHauler < Formula
end
```

Binary installed:

```txt
bin/vhaul
```

## Source-build Formula

The simplest first formula builds from the GitHub release source tarball:

```ruby
class VibeHauler < Formula
  desc "Local-first cleaner for AI agent clients"
  homepage "https://github.com/vibehauler/vibe-hauler"
  url "https://github.com/vibehauler/vibe-hauler/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "<source-tarball-sha256>"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/vibe-hauler")
  end

  test do
    assert_match "VibeHauler", shell_output("#{bin}/vhaul --version")
  end
end
```

Source builds are easy to audit but slower for users.

## Bottles

Once the formula is stable, publish bottles for:

- macOS arm64;
- macOS x86_64;
- Linux x86_64 if supported.

Bottles reduce install time. Homebrew bottle metadata must be kept in sync with the formula and release assets.

## Release Assets

For Homebrew, publish:

```txt
vibe-hauler-v0.1.0-source.tar.gz
vibe-hauler-v0.1.0-source.tar.gz.sha256
vhaul-v0.1.0-aarch64-apple-darwin.tar.gz
vhaul-v0.1.0-x86_64-apple-darwin.tar.gz
vhaul-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
checksums.txt
checksums.txt.sig
```

The source tarball is enough for a source-build formula. Binary tarballs are useful for a custom tap strategy or for bottle automation.

## Formula Test

Homebrew prefers tests that exercise real functionality. For v0.1:

```ruby
test do
  (testpath/".claude/projects/demo").mkpath
  (testpath/".claude/projects/demo/session.jsonl").write(%({"type":"summary","summary":"demo"}\n))
  assert_match "Claude", shell_output("#{bin}/vhaul scan --portable-root #{testpath}")
end
```

This is better than only checking `--help`.

## CI Tasks

Release workflow should:

1. run Rust tests;
2. build release binaries;
3. generate checksums;
4. create GitHub Release;
5. update tap formula URL/version/sha256;
6. run `brew audit --strict --online vibe-hauler`;
7. run `brew test vibe-hauler`;
8. open or push a tap PR.

## Docs References

- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [Homebrew Bottles](https://docs.brew.sh/Bottles)

