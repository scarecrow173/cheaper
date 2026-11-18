#!/usr/bin/env node
// build-api-inventory.mjs
//
// Scans docs/*.md for endpoint headings of the form
//   ### <HTTP-METHOD> <path>
// and writes reports/api-inventory.md — every endpoint grouped by
// service domain (the filename prefix before the first "-").
//
// Re-run after docs change:
//   npm run api-inventory
// or
//   node scripts/build-api-inventory.mjs
//
// Output is deterministic (no timestamps) so diffs stay clean.

import { readdirSync, readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const docsDir = join(root, "docs");
const outPath = join(root, "reports", "api-inventory.md");

// Preferred domain ordering; unknown prefixes sort alphabetically after these.
const DOMAIN_ORDER = ["billing", "auth", "notify", "ledger", "catalog", "shipping"];

const HEADING_RE = /^###\s+(GET|POST|PUT|PATCH|DELETE|HEAD|OPTIONS)\s+(\S+)\s*$/;
const METHOD_ORDER = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

const files = readdirSync(docsDir)
  .filter((f) => f.endsWith(".md"))
  .sort();

/** domain -> [{ method, path, description, file }] */
const domains = new Map();

for (const file of files) {
  const domain = file.includes("-") ? file.slice(0, file.indexOf("-")) : file.replace(/\.md$/, "");
  const lines = readFileSync(join(docsDir, file), "utf8").split(/\r?\n/);

  for (let i = 0; i < lines.length; i++) {
    const m = lines[i].match(HEADING_RE);
    if (!m) continue;
    const [, method, path] = m;

    // First non-blank line after the heading is the one-line summary,
    // unless it is already the param table or another heading.
    let description = "";
    for (let j = i + 1; j < lines.length; j++) {
      const s = lines[j].trim();
      if (!s) continue;
      if (!s.startsWith("|") && !s.startsWith("#")) description = s;
      break;
    }

    if (!domains.has(domain)) domains.set(domain, []);
    domains.get(domain).push({ method, path, description, file });
  }
}

const domainNames = [...domains.keys()].sort((a, b) => {
  const ia = DOMAIN_ORDER.indexOf(a);
  const ib = DOMAIN_ORDER.indexOf(b);
  if (ia !== -1 || ib !== -1) return (ia === -1 ? 99 : ia) - (ib === -1 ? 99 : ib);
  return a.localeCompare(b);
});

const esc = (s) => s.replace(/\|/g, "\\|");
const methodRank = (m) => {
  const i = METHOD_ORDER.indexOf(m);
  return i === -1 ? METHOD_ORDER.length : i;
};

const out = [];
out.push("# API Endpoint Inventory");
out.push("");
out.push("Generated from `docs/*.md` by `scripts/build-api-inventory.mjs`.");
out.push("Regenerate with `npm run api-inventory`.");

let total = 0;
for (const d of domainNames) total += domains.get(d).length;
out.push("");
out.push(`**${total} endpoints** across **${domainNames.length} domains** (${files.length} doc files scanned).`);

for (const domain of domainNames) {
  const eps = domains
    .get(domain)
    .sort((a, b) => a.path.localeCompare(b.path) || methodRank(a.method) - methodRank(b.method));
  out.push("");
  out.push(`## ${domain} (${eps.length})`);
  out.push("");
  out.push("| Method | Path | Description | Source |");
  out.push("|---|---|---|---|");
  for (const e of eps) {
    out.push(`| ${e.method} | \`${e.path}\` | ${esc(e.description)} | \`docs/${e.file}\` |`);
  }
}

out.push("");

mkdirSync(dirname(outPath), { recursive: true });
writeFileSync(outPath, out.join("\n"), "utf8");
console.log(`wrote ${outPath} (${total} endpoints, ${domainNames.length} domains, ${files.length} files)`);
