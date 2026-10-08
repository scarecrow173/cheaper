// Generates packages/apm/.apm/hooks/cli/ — the self-contained cheaper CLI
// payload shipped inside the APM hook bundle (DESIGN-0014). Run via
// `pnpm bundle`. Tree assembly lives in scripts/lib/apm-bundle.mjs; this
// entrypoint only reports the result.

import path from "node:path";

import { buildApmCliTree, repoRoot } from "./lib/apm-bundle.mjs";

const { outDir } = buildApmCliTree();
console.log(`bundled cheaper CLI -> ${path.relative(repoRoot(), outDir)}`);
