#!/usr/bin/env node
'use strict';

const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const { SUPPORTED, platformKey, binaryName, packageName } = require('../lib/platform.cjs');

function fail(message) {
  console.error(`@rassul/minesweeper: ${message}`);
  console.error(`Supported platforms: ${SUPPORTED.join(', ')}`);
  process.exit(1);
}

function resolveBinary() {
  const key = platformKey(process.platform, process.arch);
  if (!key) {
    fail(`unsupported platform ${process.platform}-${process.arch}`);
  }

  const pkg = packageName(key);
  let pkgRoot;
  try {
    pkgRoot = path.dirname(require.resolve(`${pkg}/package.json`));
  } catch {
    fail(
      `native package not installed (${pkg}). Reinstall with network access so optionalDependencies can resolve.`,
    );
  }

  const bin = path.join(pkgRoot, 'bin', binaryName(key));
  if (!fs.existsSync(bin)) {
    fail(`binary missing in ${pkg} (expected bin/${binaryName(key)})`);
  }
  return bin;
}

const binary = resolveBinary();
const result = spawnSync(binary, process.argv.slice(2), { stdio: 'inherit' });
if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}
process.exit(result.status === null ? 1 : result.status);
