'use strict';

const { describe, it } = require('node:test');
const assert = require('node:assert/strict');
const {
  SUPPORTED,
  platformKey,
  binaryName,
  packageName,
} = require('../lib/platform.cjs');

describe('platform package mapping', () => {
  it('maps each supported Node platform/arch to a scoped package', () => {
    assert.equal(platformKey('linux', 'x64'), 'linux-x64');
    assert.equal(packageName('linux-x64'), '@rassul/minesweeper-linux-x64');
    assert.equal(platformKey('linux', 'arm64'), 'linux-arm64');
    assert.equal(platformKey('darwin', 'x64'), 'darwin-x64');
    assert.equal(platformKey('darwin', 'arm64'), 'darwin-arm64');
    assert.equal(platformKey('win32', 'x64'), 'win32-x64');
    assert.equal(packageName('win32-x64'), '@rassul/minesweeper-win32-x64');
  });

  it('returns null for unsupported platforms (fail-fast input)', () => {
    assert.equal(platformKey('freebsd', 'x64'), null);
    assert.equal(platformKey('linux', 'ia32'), null);
    assert.equal(platformKey('win32', 'arm64'), null);
  });

  it('lists exactly the day-one supported matrix', () => {
    assert.deepEqual(SUPPORTED, [
      'linux-x64',
      'linux-arm64',
      'darwin-x64',
      'darwin-arm64',
      'win32-x64',
    ]);
  });

  it('uses .exe only on win32-x64', () => {
    assert.equal(binaryName('linux-x64'), 'tui-minesweeper');
    assert.equal(binaryName('darwin-arm64'), 'tui-minesweeper');
    assert.equal(binaryName('win32-x64'), 'tui-minesweeper.exe');
  });
});
