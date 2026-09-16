// pi-ai resolves per-provider OAuth flows through a variable dynamic import
// that bundlers cannot follow; registering statically bundled loaders lets
// the esbuild single-file bundle run OAuth login/derivation.
import { registerBundledOAuthFlows } from "@cheaper/runtime-pi";
import { runCli } from "./cli.js";

registerBundledOAuthFlows();

process.exitCode = await runCli(process.argv.slice(2), {
  stdin: process.stdin,
  stdout: process.stdout,
  stderr: process.stderr,
  env: process.env,
  cwd: process.cwd(),
});
