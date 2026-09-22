#!/usr/bin/env node
'use strict';

/**
 * Set lockstep version on the meta package (including optionalDependencies).
 *
 * Usage: node set-version.cjs 0.1.0
 */

const fs = require('node:fs');
const path = require('node:path');

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  console.error('Usage: node set-version.cjs <semver>');
  process.exit(2);
}

const pkgPath = path.join(__dirname, '..', 'minesweeper', 'package.json');
const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
pkg.version = version;

const optional = pkg.optionalDependencies ?? {};
for (const name of Object.keys(optional)) {
  optional[name] = version;
}
pkg.optionalDependencies = optional;

fs.writeFileSync(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`);
console.log(`Set ${pkg.name}@${version}`);
