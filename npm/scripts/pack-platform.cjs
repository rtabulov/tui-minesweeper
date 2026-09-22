#!/usr/bin/env node
'use strict';

/**
 * Build a publishable platform package directory from a compiled binary.
 *
 * Usage:
 *   node pack-platform.cjs --platform linux-x64 --version 0.1.0 \
 *     --binary ../../target/release/tui-minesweeper --out ../dist
 */

const fs = require('node:fs');
const path = require('node:path');
const {
  PLATFORMS,
  packageName,
  platformMeta,
} = require('../minesweeper/lib/platform.cjs');

function usage() {
  console.error(
    'Usage: node pack-platform.cjs --platform <id> --version <semver> --binary <path> --out <dir>',
  );
  process.exit(2);
}

function argValue(argv, name) {
  const i = argv.indexOf(name);
  if (i === -1 || i + 1 >= argv.length) {
    return null;
  }
  return argv[i + 1];
}

function main() {
  const argv = process.argv.slice(2);
  const platform = argValue(argv, '--platform');
  const version = argValue(argv, '--version');
  const binary = argValue(argv, '--binary');
  const outRoot = argValue(argv, '--out');

  if (!platform || !version || !binary || !outRoot) {
    usage();
  }
  if (!Object.hasOwn(PLATFORMS, platform)) {
    console.error(`Unknown platform '${platform}'. Known: ${Object.keys(PLATFORMS).join(', ')}`);
    process.exit(1);
  }
  if (!fs.existsSync(binary)) {
    console.error(`Binary not found: ${binary}`);
    process.exit(1);
  }

  const meta = platformMeta(platform);
  const name = packageName(platform);
  const pkgDir = path.join(outRoot, `minesweeper-${platform}`);
  const binDir = path.join(pkgDir, 'bin');
  const binName = meta.binary;
  const licenseSrc = path.join(__dirname, '..', '..', 'LICENSE');

  fs.rmSync(pkgDir, { recursive: true, force: true });
  fs.mkdirSync(binDir, { recursive: true });
  fs.copyFileSync(binary, path.join(binDir, binName));
  if (!binName.endsWith('.exe')) {
    fs.chmodSync(path.join(binDir, binName), 0o755);
  }
  fs.copyFileSync(licenseSrc, path.join(pkgDir, 'LICENSE'));

  const pkg = {
    name,
    version,
    description: `${platform} binary for @rassul/minesweeper`,
    license: 'MIT',
    repository: {
      type: 'git',
      url: 'git+https://github.com/rtabulov/tui-minesweeper.git',
    },
    os: meta.os,
    cpu: meta.cpu,
    engines: { node: '>=18' },
    publishConfig: { access: 'public' },
    files: ['bin/', 'LICENSE'],
  };

  fs.writeFileSync(path.join(pkgDir, 'package.json'), `${JSON.stringify(pkg, null, 2)}\n`);
  console.log(`Packed ${name}@${version} -> ${pkgDir}`);
}

main();
