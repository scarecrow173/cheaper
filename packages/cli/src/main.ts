import { runCli } from "./cli.js";

process.exitCode = await runCli(process.argv.slice(2), {
  stdin: process.stdin,
  stdout: process.stdout,
  stderr: process.stderr,
  env: process.env,
  cwd: process.cwd(),
});
