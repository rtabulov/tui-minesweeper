'use strict';

const { describe, it, before, after } = require('node:test');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const scripts = path.join(__dirname, '..', '..', 'scripts');
const packScript = path.join(scripts, 'pack-platform.cjs');
const setVersionScript = path.join(scripts, 'set-version.cjs');
const metaPkgPath = path.join(__dirname, '..', 'package.json');

describe('pack-platform', () => {
  let tmp;
  let fakeBin;

  before(() => {
    tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'minesweeper-pack-'));
    fakeBin = path.join(tmp, 'fake-bin');
    fs.writeFileSync(fakeBin, '#!/bin/sh\necho ok\n');
    fs.chmodSync(fakeBin, 0o755);
  });

  after(() => {
    fs.rmSync(tmp, { recursive: true, force: true });
  });

  it('writes scoped package.json with os/cpu and copies binary', () => {
    const out = path.join(tmp, 'out');
    const result = spawnSync(
      process.execPath,
      [
        packScript,
        '--platform',
        'linux-x64',
        '--version',
        '1.2.3',
        '--binary',
        fakeBin,
        '--out',
        out,
      ],
      { encoding: 'utf8' },
    );
    assert.equal(result.status, 0, result.stderr);

    const pkgDir = path.join(out, 'minesweeper-linux-x64');
    const pkg = JSON.parse(fs.readFileSync(path.join(pkgDir, 'package.json'), 'utf8'));
    assert.equal(pkg.name, '@rassul/minesweeper-linux-x64');
    assert.equal(pkg.version, '1.2.3');
    assert.deepEqual(pkg.os, ['linux']);
    assert.deepEqual(pkg.cpu, ['x64']);
    assert.ok(fs.existsSync(path.join(pkgDir, 'bin', 'tui-minesweeper')));
  });

  it('names the Windows binary with .exe', () => {
    const out = path.join(tmp, 'out-win');
    const result = spawnSync(
      process.execPath,
      [
        packScript,
        '--platform',
        'win32-x64',
        '--version',
        '1.2.3',
        '--binary',
        fakeBin,
        '--out',
        out,
      ],
      { encoding: 'utf8' },
    );
    assert.equal(result.status, 0, result.stderr);
    assert.ok(
      fs.existsSync(path.join(out, 'minesweeper-win32-x64', 'bin', 'tui-minesweeper.exe')),
    );
    assert.ok(fs.existsSync(path.join(out, 'minesweeper-win32-x64', 'LICENSE')));
  });
});

describe('set-version', () => {
  it('locksteps meta version and optionalDependencies', () => {
    const original = fs.readFileSync(metaPkgPath, 'utf8');
    try {
      const result = spawnSync(process.execPath, [setVersionScript, '9.8.7'], {
        encoding: 'utf8',
      });
      assert.equal(result.status, 0, result.stderr);
      const pkg = JSON.parse(fs.readFileSync(metaPkgPath, 'utf8'));
      assert.equal(pkg.version, '9.8.7');
      for (const [name, ver] of Object.entries(pkg.optionalDependencies)) {
        assert.equal(ver, '9.8.7', name);
      }
    } finally {
      fs.writeFileSync(metaPkgPath, original);
    }
  });
});

describe('bump-cargo-version', () => {
  const bumpScript = path.join(scripts, 'bump-cargo-version.cjs');

  it('updates only the [package] version line', () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'minesweeper-cargo-'));
    const cargoPath = path.join(dir, 'Cargo.toml');
    fs.writeFileSync(
      cargoPath,
      `[package]\nname = "demo"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\nserde = "1.0.0"\n`,
    );
    try {
      const result = spawnSync(
        process.execPath,
        [bumpScript, '2.3.4', cargoPath],
        { encoding: 'utf8' },
      );
      assert.equal(result.status, 0, result.stderr);
      const text = fs.readFileSync(cargoPath, 'utf8');
      assert.match(text, /^version = "2\.3\.4"$/m);
      assert.match(text, /serde = "1\.0\.0"/);
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  it('rejects non-semver', () => {
    const result = spawnSync(process.execPath, [bumpScript, 'v1.2.3'], {
      encoding: 'utf8',
    });
    assert.equal(result.status, 2);
  });
});
