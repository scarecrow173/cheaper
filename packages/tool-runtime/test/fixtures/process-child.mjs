import { spawn } from "node:child_process";
import { access, mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";

const [mode, target, ...args] = process.argv.slice(2);
if (mode === "inspect") {
  let stdin = "";
  for await (const chunk of process.stdin) stdin += chunk;
  console.log(
    JSON.stringify({
      cwd: process.cwd(),
      args: [target, ...args],
      explicit: process.env.ONLY_THIS,
      secret: process.env.CHEAPER_PROCESS_TEST_SECRET ?? null,
      path: process.env.PATH ?? null,
      stdin,
    }),
  );
} else if (mode === "output") {
  process.stdout.write(Buffer.alloc(131072, 0xff));
  process.stderr.write(Buffer.alloc(131072, 0x51));
} else if (mode === "fail") {
  console.log("before failure");
  console.error("failure detail");
  process.exitCode = 7;
} else if (mode === "hold") {
  while (true) {
    try {
      await access(target);
      break;
    } catch {
      await delay(20);
    }
  }
  console.log("released");
} else if (mode === "touch") {
  await mkdir(path.dirname(target), { recursive: true });
  await writeFile(target, "generated");
  console.log("generated");
} else if (mode === "child") {
  if (target === "stubborn") process.on("SIGTERM", () => {});
  console.log("ready");
  setInterval(() => {}, 1000);
} else if (mode === "graceful" || mode === "stubborn") {
  process.on("SIGTERM", () => {
    if (mode === "graceful") {
      writeFile(`${target}.terminated`, "graceful").then(() => process.exit(0));
    }
  });
  await writeFile(target, String(process.pid));
  setInterval(() => {}, 1000);
} else if (mode === "branch") {
  if (target === "stubborn") process.on("SIGTERM", () => {});
  const leaf = spawn(process.execPath, [fileURLToPath(import.meta.url), "child", target ?? ""], {
    env: process.env,
    stdio: ["ignore", "pipe", "inherit"],
    windowsHide: true,
  });
  await new Promise((resolve, reject) => {
    leaf.stdout.once("data", resolve);
    leaf.once("error", reject);
  });
  console.log(JSON.stringify({ child: process.pid, grandchild: leaf.pid }));
  setInterval(() => {}, 1000);
} else if (
  mode === "exit-tree" ||
  mode === "fail-tree" ||
  mode === "deep-tree" ||
  mode === "deep-stubborn"
) {
  if (mode === "deep-stubborn") process.on("SIGTERM", () => {});
  const branch = spawn(
    process.execPath,
    [fileURLToPath(import.meta.url), "branch", mode === "deep-stubborn" ? "stubborn" : ""],
    {
      env: process.env,
      stdio: ["ignore", "pipe", "inherit"],
      windowsHide: true,
    },
  );
  const ids = await new Promise((resolve, reject) => {
    branch.stdout.once("data", (chunk) => resolve(JSON.parse(chunk.toString())));
    branch.once("error", reject);
  });
  await writeFile(target, JSON.stringify({ parent: process.pid, ...ids }));
  if (mode === "deep-tree" || mode === "deep-stubborn") setInterval(() => {}, 1000);
  else process.exit(mode === "fail-tree" ? 7 : 0);
} else if (mode === "tree") {
  const child = spawn(process.execPath, [fileURLToPath(import.meta.url), "child"], {
    env: process.env,
    stdio: ["ignore", "pipe", "ignore"],
    windowsHide: true,
  });
  await new Promise((resolve, reject) => {
    child.stdout.once("data", resolve);
    child.once("error", reject);
  });
  await writeFile(target, JSON.stringify({ parent: process.pid, child: child.pid }));
  setInterval(() => {}, 1000);
}
