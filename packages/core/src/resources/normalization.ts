import path from "node:path";
import { CheaperError } from "../errors.js";
import {
  canonicalizeWorkspaceRoot,
  resolveWorkspacePath,
  type SymlinkPolicy,
} from "../workspace/paths.js";
import { resourceKey } from "./predicates.js";
import type { ResourceClaim, ResourceManifest, ResourceScope } from "./types.js";

/** D5 JSON の workspace-relative 宣言。mode は bucket から導出する。 */
export type ScopeDeclaration = { path: string; scope: ResourceScope };
export type ResourceManifestDeclaration = {
  reads?: readonly ScopeDeclaration[] | null;
  writes?: readonly ScopeDeclaration[] | null;
  produces?: readonly ScopeDeclaration[];
};

/** core API は canonical claim も受理する。同一 bucket の表現混在は拒否。 */
export type ResourceManifestInput = {
  reads?: readonly ScopeDeclaration[] | readonly ResourceClaim[] | null;
  writes?: readonly ScopeDeclaration[] | readonly ResourceClaim[] | null;
  produces?: readonly ScopeDeclaration[] | readonly ResourceClaim[];
};

function invalid(message: string): never {
  throw new CheaperError("INVALID_REQUEST", `invalid resource declaration: ${message}`);
}

function objectWithKeys(
  value: unknown,
  allowed: readonly string[],
  required: readonly string[] = allowed,
): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    invalid("expected an object");
  }
  const record = value as Record<string, unknown>;
  if (
    Object.keys(record).some((key) => !allowed.includes(key)) ||
    required.some((key) => !Object.hasOwn(record, key))
  ) {
    invalid("missing or unknown field");
  }
  return record;
}

function validateScope(value: unknown): asserts value is ResourceScope {
  if (value !== "exact" && value !== "subtree") invalid("scope must be exact or subtree");
}

function validateMode(value: unknown): asserts value is ResourceClaim["mode"] {
  if (value !== "shared" && value !== "exclusive") invalid("mode must be shared or exclusive");
}

function validatePolicy(value: unknown): asserts value is SymlinkPolicy {
  if (value !== "deny" && value !== "within-workspace") invalid("unknown symlink policy");
}

function validatePath(value: unknown): asserts value is string {
  if (typeof value !== "string" || value.length === 0 || value.includes("\0")) {
    invalid("path must be a nonempty string without NUL");
  }
}

function declaration(value: unknown): ScopeDeclaration {
  const record = objectWithKeys(value, ["path", "scope"]);
  validatePath(record.path);
  validateScope(record.scope);
  if (path.isAbsolute(record.path) || /^[a-z]:/i.test(record.path)) {
    invalid("declaration path must be workspace-relative");
  }
  return { path: record.path, scope: record.scope };
}

function canonicalClaim(value: unknown, mode: ResourceClaim["mode"]): ResourceClaim {
  const record = objectWithKeys(value, ["resource", "mode"]);
  validateMode(record.mode);
  if (record.mode !== mode) invalid("bucket and claim mode disagree");
  const resource = objectWithKeys(record.resource, ["kind", "path", "scope"]);
  if (resource.kind !== "workspace") invalid("unknown resource kind");
  validatePath(resource.path);
  validateScope(resource.scope);
  if (!path.isAbsolute(resource.path)) invalid("canonical claim path must be absolute");
  return {
    resource: { kind: "workspace", path: resource.path, scope: resource.scope },
    mode,
  };
}

/** POSIX の backslash は escape meta、Windows では separator。 */
function components(input: string): string[] {
  return process.platform === "win32" ? input.split(/[\\/]/) : input.split("/");
}

function isGlobComponent(component: string): boolean {
  return (
    /[*?[\]{}()]/.test(component) || (process.platform !== "win32" && component.includes("\\"))
  );
}

function declaredScope(input: ScopeDeclaration, ownership: boolean) {
  const parts = components(input.path);
  const negated = input.path.startsWith("!");
  const firstMeta = negated ? 0 : parts.findIndex(isGlobComponent);
  if (firstMeta < 0) return { path: input.path, scope: input.scope, glob: false };
  if (parts.slice(firstMeta).includes("..")) invalid("traversal after glob meta");

  if (ownership) {
    if (
      negated ||
      input.scope !== "subtree" ||
      firstMeta !== parts.length - 1 ||
      parts[firstMeta] !== "**"
    ) {
      invalid("partial glob producer requires enumeration or directory ownership");
    }
  }
  return {
    path: parts.slice(0, firstMeta).join(path.sep) || ".",
    scope: "subtree" as const,
    glob: true,
  };
}

async function resolveDeclaration(
  root: string,
  input: ScopeDeclaration,
  mode: ResourceClaim["mode"],
  policy: SymlinkPolicy,
  ownership: boolean,
): Promise<ResourceClaim> {
  const scope = declaredScope(input, ownership);
  // 拡大前にも literal ancestor の boundary を検査。ownership は拡大しない。
  const resolved = await resolveWorkspacePath(root, scope.path, policy);
  const identity = scope.glob && !ownership && policy === "within-workspace" ? root : resolved.real;
  return { resource: { kind: "workspace", path: identity, scope: scope.scope }, mode };
}

export async function normalizeResourceClaim(
  workspaceRoot: string,
  input: ScopeDeclaration,
  mode: ResourceClaim["mode"],
  symlinkPolicy: SymlinkPolicy,
): Promise<ResourceClaim> {
  validateMode(mode);
  validatePolicy(symlinkPolicy);
  const parsed = declaration(input);
  const root = await canonicalizeWorkspaceRoot(workspaceRoot);
  return resolveDeclaration(root, parsed, mode, symlinkPolicy, false);
}

function compare(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

function deduplicate(claims: ResourceClaim[]): ResourceClaim[] {
  const sorted = claims.sort(
    (a, b) =>
      compare(resourceKey(a.resource), resourceKey(b.resource)) ||
      compare(a.mode, b.mode) ||
      compare(a.resource.path, b.resource.path),
  );
  const seen = new Set<string>();
  return sorted.filter((claim) => {
    const key = JSON.stringify([resourceKey(claim.resource), claim.mode]);
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

async function normalizeBucket(
  root: string,
  input: unknown,
  mode: ResourceClaim["mode"],
  policy: SymlinkPolicy,
  ownership: boolean,
): Promise<ResourceClaim[]> {
  if (input === undefined || input === null) {
    if (ownership) {
      if (input === null) invalid("unknown producer ownership");
      return [];
    }
    return [{ resource: { kind: "workspace", path: root, scope: "subtree" }, mode }];
  }
  if (!Array.isArray(input)) invalid("bucket must be an array");
  let representation: "claim" | "declaration" | undefined;
  const claims: ResourceClaim[] = [];
  for (const item of input) {
    const record = objectWithKeys(item, ["resource", "mode", "path", "scope"], []);
    const current = Object.hasOwn(record, "resource") ? "claim" : "declaration";
    if (representation !== undefined && representation !== current)
      invalid("mixed bucket representations");
    representation = current;
    if (current === "claim") {
      const claim = canonicalClaim(item, mode);
      // canonical path は literal identity。glob として再解釈しない。
      const resolved = await resolveWorkspacePath(root, claim.resource.path, policy);
      claims.push({ resource: { ...claim.resource, path: resolved.real }, mode });
    } else {
      claims.push(await resolveDeclaration(root, declaration(item), mode, policy, ownership));
    }
  }
  return deduplicate(claims);
}

/** WorkspaceEscapeError は既存 boundary のまま伝播する。 */
export async function normalizeResourceManifest(
  workspaceRoot: string,
  input: ResourceManifestInput | undefined,
  symlinkPolicy: SymlinkPolicy,
): Promise<ResourceManifest> {
  validatePolicy(symlinkPolicy);
  const record =
    input === undefined ? {} : objectWithKeys(input, ["reads", "writes", "produces"], []);
  const root = await canonicalizeWorkspaceRoot(workspaceRoot);
  return {
    reads: await normalizeBucket(root, record.reads, "shared", symlinkPolicy, false),
    writes: await normalizeBucket(root, record.writes, "exclusive", symlinkPolicy, false),
    produces: await normalizeBucket(root, record.produces, "exclusive", symlinkPolicy, true),
  };
}
