import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const hooksPath = path.join(repoRoot, "packages/apm/.apm/hooks/hooks.json");
const hooksJson = JSON.parse(readFileSync(hooksPath, "utf8"));

const pretooluseGroups = (hooksJson.hooks?.PreToolUse ?? []).filter((group) =>
  (group.hooks ?? []).some((h) => String(h.command).includes("--target pretooluse")),
);
const matcher = new RegExp(pretooluseGroups[0]?.matcher ?? "$^");

describe("source hooks.json PreToolUse matcher", () => {
  it("matches read/grep/bash/powershell in every ASCII case variant", () => {
    for (const name of [
      "Read",
      "READ",
      "read",
      "rEaD",
      "Grep",
      "GREP",
      "grep",
      "gReP",
      "Bash",
      "BASH",
      "bash",
      "PowerShell",
      "POWERSHELL",
      "powershell",
      "pOwErShElL",
    ]) {
      assert.ok(matcher.test(name), `expected match: ${name}`);
    }
  });

  it("rejects prefix/suffix variants and unknown tools", () => {
    for (const name of [
      "reads",
      "xread",
      "readx",
      "grepx",
      "xgrep",
      "bashx",
      "xpowershell",
      "Write",
      "view",
      "shell",
      "",
      " read",
      "read ",
      "read,grep",
    ]) {
      assert.ok(!matcher.test(name), `expected non-match: ${name}`);
    }
  });
});
