# Release Checklist

This checklist covers a normal public release of VibeHauler.

## Before Release

- Product name is consistently `VibeHauler`.
- Binary is consistently `vhaul`.
- Rust package is consistently `vibe-hauler`.
- `CHANGELOG.md` has release notes.
- `README.md` includes install commands for Cargo, Homebrew, npm, and direct binary.
- CLI help examples are current.
- Fixtures pass on macOS/Linux/Windows CI.
- `cargo fmt`, `clippy`, and tests pass.
- `cargo deny` or equivalent dependency audit passes.
- Release build embeds version, git SHA, and target triple.

## Artifacts

Build target matrix:

| Target | Required for v0.1 |
|---|---:|
| `aarch64-apple-darwin` | yes |
| `x86_64-apple-darwin` | yes |
| `x86_64-unknown-linux-gnu` | yes |
| `aarch64-unknown-linux-gnu` | optional |
| `x86_64-pc-windows-msvc` | yes |
| `aarch64-pc-windows-msvc` | optional |

Each archive should contain:

```txt
vhaul or vhaul.exe
README.md
LICENSE
```

Shell completions and manpages are optional until the TUI stabilizes. Add them to archives only after generators exist in CI.

## Checksums and Provenance

- Generate `checksums.txt`.
- Sign release artifacts if signing key policy is ready.
- Generate GitHub provenance/attestations if CI supports it.
- npm releases should use Trusted Publishing where possible.

## Homebrew

- Update `Formula/vibe-hauler.rb`.
- Verify source tarball sha256.
- Run formula install locally.
- Run formula test.
- Run formula audit.
- Push or PR to `zerob13/homebrew-tap`.

## npm

- Package native binaries into platform packages.
- Run `npm pack --dry-run` for each package.
- Publish platform packages first.
- Publish wrapper package last.
- Install globally on at least one macOS and one Linux runner.

## Smoke Tests

```bash
vhaul --version
vhaul --help
```

Manual release smoke should also open `vhaul` in a real terminal and complete a read-only fixture walkthrough. Automated fixture smoke tests should drive the TUI harness directly rather than relying on public subcommands.

## After Release

- Verify GitHub Release assets download.
- Verify `brew install zerob13/tap/vibe-hauler`.
- Verify `npm install -g vibe-hauler`.
- Create a short release note with risks and known limitations.
- Open issues for any parser limitations discovered during smoke tests.
