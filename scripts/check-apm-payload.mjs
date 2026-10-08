// Source assets are verified here; native builds and release smokes verify executables.
import { lstatSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { cargoWorkspaceVersion } from "./lib/release-artifacts.mjs";

const workers = ["bulk-reader", "code-writer", "doc-writer"];
const assets = [
  "cheaper",
  "cheaper.ps1",
  "hooks.json",
  "cli/version.json",
  "cli/policy.yaml",
  ...workers.flatMap((name) => [
    `cli/workers/${name}.md`,
    `cli/workers/schemas/${name}.input.schema.json`,
    `cli/workers/schemas/${name}.output.schema.json`,
  ]),
];

export function checkApmPayload(repoRoot) {
  const errors = [];
  const contents = new Map();
  for (const asset of assets) {
    const relative = `packages/apm/.apm/hooks/${asset}`;
    try {
      const file = path.join(repoRoot, relative);
      const stat = lstatSync(file);
      if (!stat.isFile() || stat.size === 0) {
        errors.push(`asset must be a nonempty regular file: ${relative}`);
        continue;
      }
      const text = readFileSync(file, "utf8");
      if (text.startsWith("version https://git-lfs")) {
        errors.push(`asset is an LFS pointer: ${relative}`);
        continue;
      }
      contents.set(asset, text);
      if (asset.endsWith(".json")) {
        try {
          JSON.parse(text);
        } catch {
          errors.push(`invalid JSON: ${relative}`);
        }
      }
    } catch (error) {
      errors.push(`asset missing or unreadable: ${relative} (${error.message})`);
    }
  }
  try {
    const canonical = cargoWorkspaceVersion(repoRoot);
    const metadata = contents.get("cli/version.json");
    const match = metadata?.match(/^\s*\{\s*"version"\s*:\s*"([^"\s]+)"\s*\}\s*$/);
    if (!match || match[1] !== canonical)
      errors.push(`version metadata must contain only canonical version ${canonical}`);
  } catch (error) {
    errors.push(`invalid canonical version JSON: ${error.message}`);
  }
  return errors;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const errors = checkApmPayload(fileURLToPath(new URL("..", import.meta.url)));
  if (errors.length) {
    console.error(errors.join("\n"));
    process.exitCode = 1;
  } else console.log("APM source assets OK (platform executables are acquired or locally built)");
}
