#!/usr/bin/env node

/**
 * Bump version across all project files:
 * - package.json
 * - src-tauri/tauri.conf.json
 * - src-tauri/Cargo.toml
 *
 * Usage: node scripts/bump-version.js 0.2.0
 */

import { readFileSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");

const version = process.argv[2];
if (!version) {
  console.error("Usage: node scripts/bump-version.js <version>");
  process.exit(1);
}

console.log(`Bumping version to ${version}`);

// 1. package.json
const pkgPath = resolve(root, "package.json");
const pkg = JSON.parse(readFileSync(pkgPath, "utf-8"));
pkg.version = version;
writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");
console.log(`  Updated package.json -> ${version}`);

// 2. src-tauri/tauri.conf.json
const tauriPath = resolve(root, "src-tauri", "tauri.conf.json");
const tauri = JSON.parse(readFileSync(tauriPath, "utf-8"));
tauri.version = version;
writeFileSync(tauriPath, JSON.stringify(tauri, null, 2) + "\n");
console.log(`  Updated tauri.conf.json -> ${version}`);

// 3. src-tauri/Cargo.toml
const cargoPath = resolve(root, "src-tauri", "Cargo.toml");
let cargo = readFileSync(cargoPath, "utf-8");
cargo = cargo.replace(
  /^version\s*=\s*"[^"]*"/m,
  `version = "${version}"`
);
writeFileSync(cargoPath, cargo);
console.log(`  Updated Cargo.toml -> ${version}`);

console.log("Done.");
