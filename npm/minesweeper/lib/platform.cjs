'use strict';

const fs = require('node:fs');
const path = require('node:path');

const PLATFORMS = JSON.parse(
  fs.readFileSync(path.join(__dirname, '..', 'platforms.json'), 'utf8'),
);

const SUPPORTED = Object.keys(PLATFORMS);

/**
 * Map Node's process.platform + process.arch to a supported package key.
 * @param {string} platform
 * @param {string} arch
 * @returns {string | null}
 */
function platformKey(platform, arch) {
  const key = `${platform}-${arch}`;
  return Object.hasOwn(PLATFORMS, key) ? key : null;
}

/**
 * @param {string} key platform key from platformKey()
 * @returns {string}
 */
function binaryName(key) {
  return PLATFORMS[key].binary;
}

/**
 * @param {string} key platform key from platformKey()
 * @returns {string}
 */
function packageName(key) {
  return `@rassul/minesweeper-${key}`;
}

/**
 * @param {string} key
 * @returns {{ os: string[], cpu: string[], binary: string }}
 */
function platformMeta(key) {
  return PLATFORMS[key];
}

module.exports = {
  SUPPORTED,
  PLATFORMS,
  platformKey,
  binaryName,
  packageName,
  platformMeta,
};
