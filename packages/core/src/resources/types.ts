export type ResourceScope = "exact" | "subtree";
export type ResourceClaim = {
  resource: { kind: "workspace"; path: string; scope: ResourceScope }; // path = canonical 絶対 real path
  mode: "shared" | "exclusive";
};
export type ResourceManifest = {
  reads: ResourceClaim[];
  writes: ResourceClaim[];
  produces: ResourceClaim[];
};
