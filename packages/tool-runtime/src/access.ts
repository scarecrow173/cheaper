import {
  canonicalizeWorkspaceRoot,
  claimContains,
  isMutatingTool,
  normalizeResourceManifest,
  type PathDenylist,
  type ProcessPolicy,
  type ResourceClaim,
  type ResourceManifest,
  type ResourceManifestInput,
  type ResourceScope,
  resolveWorkspacePath,
  type SymlinkPolicy,
  type ToolName,
} from "@cheaper/core";
import { createToolCatalog, type JsonValue } from "./catalog.js";
import { invalidResource, type LeasePreparation } from "./resources.js";
import { POLICY_DENIED_MESSAGE, rethrowAsDenied, SCOPE_DENIED_MESSAGE } from "./walker.js";

export type ReadStrictness = "record" | "reject";
export interface ResourceScopeOptions {
  resourceManifest?: ResourceManifestInput | undefined;
  writeScope?: ReadonlySet<string> | undefined;
  readStrictness?: ReadStrictness | undefined;
}
export type AccessObserver = (
  path: string,
  mode: ResourceClaim["mode"],
  scope?: ResourceScope,
  support?: boolean,
) => Promise<void>;
export interface AccessObservation {
  readonly operationId?: string;
  readonly path: string;
  readonly claim: ResourceClaim;
  readonly declared: boolean;
  readonly allowed: boolean;
  readonly support: boolean;
}
export interface AccessSnapshot {
  readonly observations: readonly AccessObservation[];
  readonly total: number;
  readonly dropped: number;
}
export interface PreparedResourceScope extends LeasePreparation {
  root: string;
  manifest: ResourceManifest;
  writeClaims: readonly ResourceClaim[] | undefined;
}
export interface ScopeAccessOptions extends ResourceScopeOptions {
  root: string;
  symlinkPolicy: SymlinkPolicy;
  names: readonly ToolName[];
  denylist?: PathDenylist | undefined;
  process?: Partial<ProcessPolicy> | undefined;
}
export interface ActiveResourceAccess {
  id: string;
  claims: readonly ResourceClaim[];
}
const contains = (claims: readonly ResourceClaim[], claim: ResourceClaim) =>
  claims.some((parent) => claimContains(parent, claim));
const allClaims = (manifest: ResourceManifest) => [
  ...manifest.reads,
  ...manifest.writes,
  ...manifest.produces,
];
export const MAX_ACCESS_OBSERVATIONS = 1024;

/** writeScope の canonical identity と display、manifest/scope 認可の唯一の実装。 */
export class ResourceAccess {
  private readonly options: ScopeAccessOptions;
  private preparation: Promise<PreparedResourceScope> | undefined;
  private readonly rows: AccessObservation[] = [];
  private total = 0;
  readonly readStrictness: ReadStrictness;
  readonly explicit: boolean;
  constructor(options: ScopeAccessOptions, inherited?: ReadStrictness) {
    if (
      options.readStrictness !== undefined &&
      options.readStrictness !== "record" &&
      options.readStrictness !== "reject"
    )
      throw invalidResource("invalid read strictness");
    if (inherited && options.readStrictness && inherited !== options.readStrictness)
      throw invalidResource("parent read strictness mismatch");
    this.readStrictness = inherited ?? options.readStrictness ?? "record";
    this.options = {
      ...options,
      names: [...options.names],
      resourceManifest:
        options.resourceManifest === undefined
          ? undefined
          : structuredClone(options.resourceManifest),
      writeScope: options.writeScope === undefined ? undefined : new Set(options.writeScope),
    };
    this.explicit = options.resourceManifest !== undefined || options.writeScope !== undefined;
  }
  prepare(): Promise<PreparedResourceScope> {
    this.preparation ??= this.prepareScope();
    return this.preparation;
  }
  /** resourceManifest 無しで準備した literal writeClaims を safety の各 bucket に narrow。 */
  async prepareWorkerManifest(safetyManifest?: ResourceManifestInput): Promise<ResourceManifest> {
    const prepared = await this.prepare();
    const safety =
      safetyManifest === undefined
        ? prepared.manifest
        : await normalizeResourceManifest(
            prepared.root,
            safetyManifest,
            this.options.symlinkPolicy,
          );
    if (prepared.writeClaims === undefined) return safety;
    return {
      reads: safety.reads,
      writes: prepared.writeClaims.filter((claim) => contains(safety.writes, claim)),
      produces: prepared.writeClaims.filter((claim) => contains(safety.produces, claim)),
    };
  }
  private async prepareScope(): Promise<PreparedResourceScope> {
    const opts = this.options,
      root = await canonicalizeWorkspaceRoot(opts.root);
    const writeClaims: ResourceClaim[] | undefined = opts.writeScope === undefined ? undefined : [];
    if (writeClaims) {
      for (const target of opts.writeScope ?? []) {
        const resolved = await resolveWorkspacePath(root, target, opts.symlinkPolicy);
        writeClaims.push({
          resource: { kind: "workspace", path: resolved.real, scope: "exact" },
          mode: "exclusive",
        });
      }
    }
    const rootClaim = (mode: ResourceClaim["mode"]): ResourceClaim => ({
      resource: { kind: "workspace", path: root, scope: "subtree" },
      mode,
    });
    const manifest =
      opts.resourceManifest !== undefined
        ? await normalizeResourceManifest(root, opts.resourceManifest, opts.symlinkPolicy)
        : {
            reads: opts.names.length ? [rootClaim("shared")] : [],
            writes: opts.names.some(isMutatingTool)
              ? (writeClaims ?? [rootClaim("exclusive")])
              : [],
            produces: [],
          };
    if (
      writeClaims &&
      [...manifest.writes, ...manifest.produces].some((claim) => !contains(writeClaims, claim))
    )
      throw invalidResource("manifest writes exceed writeScope");
    const reads =
      this.readStrictness === "record" && opts.names.length
        ? [rootClaim("shared")]
        : manifest.reads;
    return {
      root,
      manifest,
      writeClaims,
      claims: [...reads, ...manifest.writes, ...manifest.produces],
      readStrictness: this.readStrictness,
    };
  }
  async operation(name: ToolName, input: JsonValue): Promise<LeasePreparation> {
    try {
      const scope = await this.prepare();
      let args = input;
      if (
        input &&
        typeof input === "object" &&
        !Array.isArray(input) &&
        typeof input.path === "string"
      ) {
        const resolved = await resolveWorkspacePath(
          scope.root,
          input.path,
          this.options.symlinkPolicy,
        );
        args = { ...input, path: resolved.display };
        if (this.options.denylist?.isDenied(resolved.display))
          throw new Error(POLICY_DENIED_MESSAGE);
      }
      const declaration = createToolCatalog({ process: this.options.process })
        .get(name)
        ?.resources?.(args);
      const manifest = await normalizeResourceManifest(
        scope.root,
        declaration,
        this.options.symlinkPolicy,
      );
      // subtree search が symlink を辿る時は start directory 外へ読み得る。
      if (this.options.symlinkPolicy === "within-workspace" && (name === "find" || name === "grep"))
        manifest.reads = [
          { resource: { kind: "workspace", path: scope.root, scope: "subtree" }, mode: "shared" },
        ];
      if (
        [...manifest.writes, ...manifest.produces].some(
          (claim) =>
            !contains([...scope.manifest.writes, ...scope.manifest.produces], claim) ||
            (scope.writeClaims && !contains(scope.writeClaims, claim)),
        )
      )
        throw new Error(SCOPE_DENIED_MESSAGE);
      return { claims: allClaims(manifest), readStrictness: this.readStrictness };
    } catch (error) {
      rethrowAsDenied(error);
    }
  }
  async observe(
    inputPath: string,
    mode: ResourceClaim["mode"],
    resourceScope: ResourceScope = "exact",
    active?: ActiveResourceAccess,
    support = false,
  ): Promise<void> {
    const prepared = await this.prepare();
    const resolved = await resolveWorkspacePath(
      prepared.root,
      inputPath,
      this.options.symlinkPolicy,
    ).catch(rethrowAsDenied);
    if (this.options.denylist?.isDenied(resolved.display)) throw new Error(POLICY_DENIED_MESSAGE);
    const claim: ResourceClaim = {
      resource: { kind: "workspace", path: resolved.real, scope: resourceScope },
      mode,
    };
    const declared = contains(
      mode === "exclusive"
        ? [...prepared.manifest.writes, ...prepared.manifest.produces]
        : prepared.manifest.reads,
      claim,
    );
    const supportIn = (claims: readonly ResourceClaim[]) =>
      isSupportDirectory(prepared.root, claim, claims);
    const authorizedWrites = [...prepared.manifest.writes, ...prepared.manifest.produces];
    const jobAllowed =
      mode === "shared"
        ? declared || this.readStrictness === "record"
        : (declared && (!prepared.writeClaims || contains(prepared.writeClaims, claim))) ||
          (support &&
            supportIn(authorizedWrites) &&
            (!prepared.writeClaims || supportIn(prepared.writeClaims)));
    const operationAllowed =
      !active || contains(active.claims, claim) || (support && supportIn(active.claims));
    const allowed = jobAllowed && operationAllowed;
    this.total++;
    if (this.rows.length < MAX_ACCESS_OBSERVATIONS)
      this.rows.push({
        ...(active ? { operationId: active.id } : {}),
        path: resolved.display,
        claim,
        declared,
        allowed,
        support,
      });
    if (!allowed) throw new Error(SCOPE_DENIED_MESSAGE);
  }
  snapshot(): AccessSnapshot {
    return {
      total: this.total,
      dropped: this.total - this.rows.length,
      observations: this.rows.map((row) => ({
        ...row,
        claim: { ...row.claim, resource: { ...row.claim.resource } },
      })),
    };
  }
}

/** mkdir は認可済み target の canonical 祖先と既存 root だけを support する。 */
export function isSupportDirectory(
  root: string,
  directory: ResourceClaim,
  claims: readonly ResourceClaim[],
): boolean {
  if (directory.resource.path === root) return true;
  const subtree = { ...directory.resource, scope: "subtree" as const };
  return claims.some(
    (claim) =>
      claim.mode === "exclusive" && claimContains({ resource: subtree, mode: "exclusive" }, claim),
  );
}
