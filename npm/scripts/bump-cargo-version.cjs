#!/usr/bin/env node
'use strict';

/**
 * Set [package].version in Cargo.toml.
 *
 * Usage: node bump-cargo-version.cjs <semver> [path/to/Cargo.toml]
 */

const fs = require('node:fs');
const path = require('node:path');

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error('Usage: node bump-cargo-version.cjs <semver> [Cargo.toml]');
  process.exit(2);
}

const cargoPath =
  process.argv[3] ?? path.join(__dirname, '..', '..', 'Cargo.toml');
const original = fs.readFileSync(cargoPath, 'utf8');

if (!/^\[package\]/m.test(original)) {
  console.error('Cargo.toml: missing [package] section');
  process.exit(1);
}

let seenPackage = false;
let replaced = false;
const updated = original
  .split('\n')
  .map((line) => {
    if (line.startsWith('[')) {
      seenPackage = line.trim() === '[package]';
      return line;
    }
    if (seenPackage && /^version\s*=\s*".*"\s*$/.test(line)) {
      replaced = true;
      return `version = "${version}"`;
    }
    return line;
  })
  .join('\n');

if (!replaced) {
  console.error('Cargo.toml: could not find [package] version');
  process.exit(1);
}

fs.writeFileSync(cargoPath, updated);
console.log(`Set Cargo.toml package version to ${version}`);
