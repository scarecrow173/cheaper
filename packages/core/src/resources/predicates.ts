import path from "node:path";
import type { ResourceClaim, ResourceManifest } from "./types.js";

type Resource = ResourceClaim["resource"];

function identityKey(identity: string): string {
  return process.platform === "win32" ? identity.toLowerCase() : identity;
}

/** 正規化済み resource の比較 key。glob / display path は受け取らない。 */
export function resourceKey(resource: Resource): string {
  return JSON.stringify([resource.kind, identityKey(resource.path), resource.scope]);
}

/** subtree は本体と descendants、exact は同じ exact identity のみを覆う。 */
export function resourceContains(container: Resource, contained: Resource): boolean {
  const parent = identityKey(container.path);
  const child = identityKey(contained.path);
  if (container.scope === "exact") {
    return contained.scope === "exact" && parent === child;
  }
  const descendantPrefix = parent.endsWith(path.sep) ? parent : parent + path.sep;
  return parent === child || child.startsWith(descendantPrefix);
}

export function resourcesOverlap(left: Resource, right: Resource): boolean {
  return resourceContains(left, right) || resourceContains(right, left);
}

/** exclusive は shared/exclusive を覆い、shared は shared のみを覆う。 */
export function claimContains(container: ResourceClaim, contained: ResourceClaim): boolean {
  return (
    (container.mode === "exclusive" || contained.mode === "shared") &&
    resourceContains(container.resource, contained.resource)
  );
}

export function claimsConflict(left: ResourceClaim, right: ResourceClaim): boolean {
  return (
    (left.mode === "exclusive" || right.mode === "exclusive") &&
    resourcesOverlap(left.resource, right.resource)
  );
}

export function manifestsConflict(left: ResourceManifest, right: ResourceManifest): boolean {
  const leftClaims = [...left.reads, ...left.writes, ...left.produces];
  const rightClaims = [...right.reads, ...right.writes, ...right.produces];
  return leftClaims.some((a) => rightClaims.some((b) => claimsConflict(a, b)));
}
