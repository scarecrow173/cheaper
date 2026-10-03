// SEA standalone executable 用 entrypoint（issue #71 / DESIGN-034z1EfqYaEBfyNvqZyFSD）。
// useVfs では injected main script が VFS root で実行されるため、
// `import.meta.dirname` が built-in resources（workers/・policy.yaml・
// version.json）を含む VFS root を指す。これを builtinDir として渡し、
// 通常 Node 実行の defaultBuiltinDir() とは分岐させない。
import { registerBundledOAuthFlows } from "@cheaper/runtime-pi";
import { runCli } from "./cli.js";

registerBundledOAuthFlows();

process.exitCode = await runCli(
  process.argv.slice(2),
  {
    stdin: process.stdin,
    stdout: process.stdout,
    stderr: process.stderr,
    env: process.env,
    cwd: process.cwd(),
  },
  { builtinDir: import.meta.dirname },
);
