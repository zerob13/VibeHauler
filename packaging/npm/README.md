# npm Packaging

npm wrapper and platform package work should follow [docs/release/npm.md](../../docs/release/npm.md).

The checked-in package folders are skeletons. Release automation must copy built native binaries into each platform package before `npm pack` or publish.
