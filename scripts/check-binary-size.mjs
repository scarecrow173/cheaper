// Asserts that a release binary stays within the size budget
// (<=20MiB contract, Phase 5 plan / ISS-21). Wired as a CI gate in
// build-executables.yml between `cargo build --release` and artifact
// upload. Usage:
//
//   node scripts/check-binary-size.mjs --file <path> [--max-mb <n>]
//
// Exits 0 when --file is a regular file of at most --max-mb mebibytes
// (default 20); exits 1 on missing/non-regular/oversize file or bad args.

import { lstatSync } from "node:fs";
import path from "node:path";
import { parseArgs } from "./lib/release-artifacts.mjs";

try {
  const args = parseArgs(process.argv.slice(2), ["file", "max-mb"]);
  if (!args.file) throw new Error("--file is required");
  const maxMb = args["max-mb"] === undefined ? 20 : Number(args["max-mb"]);
  if (!Number.isFinite(maxMb) || maxMb <= 0) {
    throw new Error(`--max-mb must be a positive number, got ${args["max-mb"]}`);
  }
  const file = path.resolve(args.file);
  let stat;
  try {
    stat = lstatSync(file);
  } catch (error) {
    throw new Error(`binary is missing or unreadable: ${file} (${error.message})`);
  }
  if (!stat.isFile()) throw new Error(`expected a regular file: ${file}`);
  const limit = maxMb * 1024 * 1024;
  if (stat.size > limit) {
    throw new Error(`binary exceeds size limit: ${file} is ${stat.size} bytes (limit ${limit} bytes = ${maxMb} MiB)`);
  }
  console.log(`binary size OK: ${file} is ${stat.size} bytes (limit ${limit} bytes = ${maxMb} MiB)`);
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
