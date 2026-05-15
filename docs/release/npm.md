# npm Release

Goal:

```bash
npm install -g vibe-hauler
vhaul
```

The npm package is a distribution wrapper for the Rust binary, not a Node rewrite.

## Recommended Package Layout

Use one top-level package plus platform packages:

```txt
packaging/npm/
├── vibe-hauler/
│   ├── package.json
│   ├── bin/
│   │   └── vhaul.js
│   └── README.md
├── vibe-hauler-darwin-arm64/
├── vibe-hauler-darwin-x64/
├── vibe-hauler-linux-x64/
├── vibe-hauler-win32-x64/
└── scripts/
```

Public packages:

| Package | Purpose |
|---|---|
| `vibe-hauler` | user-facing wrapper with `bin.vhaul` |
| `@vibehauler/vhaul-darwin-arm64` | native binary package |
| `@vibehauler/vhaul-darwin-x64` | native binary package |
| `@vibehauler/vhaul-linux-x64` | native binary package |
| `@vibehauler/vhaul-win32-x64` | native binary package |

The wrapper uses `optionalDependencies` to install the right native package.

Optional release targets such as `aarch64-unknown-linux-gnu` and `aarch64-pc-windows-msvc` should only get npm packages after CI produces and smoke-tests those binaries.

## Wrapper `package.json`

```json
{
  "name": "vibe-hauler",
  "version": "0.1.0",
  "description": "A local-first cleaner for AI agent clients.",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "git+https://github.com/zerob13/VibeHauler.git"
  },
  "bin": {
    "vhaul": "bin/vhaul.js"
  },
  "files": [
    "bin/",
    "README.md",
    "package.json"
  ],
  "optionalDependencies": {
    "@vibehauler/vhaul-darwin-arm64": "0.1.0",
    "@vibehauler/vhaul-darwin-x64": "0.1.0",
    "@vibehauler/vhaul-linux-x64": "0.1.0",
    "@vibehauler/vhaul-win32-x64": "0.1.0"
  }
}
```

## Platform Package `package.json`

Example:

```json
{
  "name": "@vibehauler/vhaul-darwin-arm64",
  "version": "0.1.0",
  "description": "VibeHauler native binary for macOS arm64",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "git+https://github.com/zerob13/VibeHauler.git"
  },
  "os": ["darwin"],
  "cpu": ["arm64"],
  "files": [
    "bin/vhaul"
  ]
}
```

Windows package should ship `bin/vhaul.exe`.

## JS Shim

The wrapper `bin/vhaul.js` should:

1. detect `process.platform` and `process.arch`;
2. resolve the matching optional dependency;
3. spawn the native binary with inherited stdio;
4. forward exit code and signals.

No install-time download script is recommended for v0.1. Optional dependency packages are easier to audit and friendlier to locked-down environments.

## Trusted Publishing

Use npm Trusted Publishing from GitHub Actions when possible:

- no long-lived npm automation token;
- OIDC-based publish;
- automatic provenance for public packages from public repositories;
- `repository.url` in every `package.json` must match the GitHub repo.

Each package needs its own trusted publisher configuration on npm.

## Publish Order

1. Build Rust binaries.
2. Copy each binary into its platform package.
3. `npm pack --dry-run` each package.
4. Publish platform packages.
5. Publish wrapper package.
6. Smoke test:

```bash
npm install -g vibe-hauler
vhaul --version
vhaul --help
```

## Versioning

All npm packages and the Rust release should use the same semver:

```txt
Rust tag:        v0.1.0
Cargo package:  vibe-hauler 0.1.0
npm wrapper:    vibe-hauler 0.1.0
npm native:     @vibehauler/vhaul-* 0.1.0
Homebrew:       v0.1.0
```

## Docs References

- [npm package.json `bin`](https://docs.npmjs.com/cli/v11/configuring-npm/package-json/#bin)
- [npm publish](https://docs.npmjs.com/cli/v10/commands/npm-publish/)
- [npm Trusted Publishing](https://docs.npmjs.com/trusted-publishers/)
