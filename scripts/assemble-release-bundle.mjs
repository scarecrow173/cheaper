import { assembleBundle, parseArgs, repoRoot } from "./lib/release-artifacts.mjs";

try {
  const args = parseArgs(process.argv.slice(2), ["asset-dir", "out-dir", "tag"], ["dry-run"]);
  if (!args["asset-dir"] || !args["out-dir"] || !args.tag)
    throw new Error("--asset-dir, --out-dir and --tag are required");
  console.log(
    JSON.stringify(
      assembleBundle({
        root: repoRoot,
        assetDir: args["asset-dir"],
        outDir: args["out-dir"],
        tag: args.tag,
        dryRun: args["dry-run"] === true,
      }),
    ),
  );
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
