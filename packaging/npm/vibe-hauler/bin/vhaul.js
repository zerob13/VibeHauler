#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const { join } = require("node:path");

const packages = {
  "darwin-arm64": "@vibehauler/vhaul-darwin-arm64",
  "darwin-x64": "@vibehauler/vhaul-darwin-x64",
  "linux-x64": "@vibehauler/vhaul-linux-x64",
  "win32-x64": "@vibehauler/vhaul-win32-x64"
};

const key = `${process.platform}-${process.arch}`;
const packageName = packages[key];

if (!packageName) {
  console.error(`Unsupported platform for VibeHauler: ${key}`);
  process.exit(1);
}

let packageRoot;
try {
  packageRoot = require.resolve(`${packageName}/package.json`);
} catch (error) {
  console.error(`Missing native VibeHauler package: ${packageName}`);
  console.error("Reinstall vibe-hauler so optionalDependencies can be resolved.");
  process.exit(1);
}

const exe = process.platform === "win32" ? "vhaul.exe" : "vhaul";
const bin = join(packageRoot, "..", "bin", exe);
const result = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status ?? 1);
