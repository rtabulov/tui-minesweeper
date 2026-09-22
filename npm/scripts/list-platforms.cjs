#!/usr/bin/env node
'use strict';

/** Print "platform<TAB>binary" rows from platforms.json for CI shells. */

const { PLATFORMS } = require('../minesweeper/lib/platform.cjs');

for (const [platform, meta] of Object.entries(PLATFORMS)) {
  process.stdout.write(`${platform}\t${meta.binary}\n`);
}
