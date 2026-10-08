import { writeFileSync } from "node:fs";
import path from "node:path";
import {
  generateManifest,
  parseArgs,
  repoRoot,
  verifyAssets,
  verifyReleaseVersion,
} from "./lib/release-artifacts.mjs";

try {
  const args = parseArgs(process.argv.slice(2), ["asset-dir", "tag"], ["verify"]);
  if (!args["asset-dir"] || !args.tag) throw new Error("--asset-dir and --tag are required");
  verifyReleaseVersion(repoRoot, args.tag);
  const dir = path.resolve(args["asset-dir"]);
  if (args.verify) verifyAssets(dir);
  else writeFileSync(path.join(dir, "SHA256SUMS"), generateManifest(dir));
  console.log(args.verify ? "Release checksums verified" : "Release SHA256SUMS generated");
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
