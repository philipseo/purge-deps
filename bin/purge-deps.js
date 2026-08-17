#!/usr/bin/env node
"use strict";

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const binaryName = process.platform === "win32" ? "purge-deps.exe" : "purge-deps";
const candidates = [
  path.join(__dirname, `${process.platform}-${process.arch}`, binaryName),
  path.join(__dirname, binaryName),
];
const binaryPath = candidates.find((candidate) => fs.existsSync(candidate));

if (!binaryPath) {
  console.error(
    `purge-deps: no binary for ${process.platform}-${process.arch}`
  );
  process.exit(1);
}

const result = spawnSync(binaryPath, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}
process.exit(result.status === null ? 1 : result.status);
