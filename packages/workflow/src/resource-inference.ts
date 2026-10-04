import {
  CheaperError,
  claimContains,
  claimsConflict,
  normalizeResourceClaim,
  normalizeResourceManifest,
  type ResourceClaim,
  type ResourceManifest,
  type ResourceManifestDeclaration,
  resourceKey,
  resourcesOverlap,
  type ScopeDeclaration,
  type SymlinkPolicy,
} from "@cheaper/core";
import type { JsonValue, ToolCatalog } from "@cheaper/tool-runtime";
import type {
  DependencyEdge,
  ResourceConflict,
  ResourceInference,
  ResourceInferenceDiagnostic,
  ResourceInferenceJob,
  ResourceInferenceResult,
} from "./types.js";

interface Options {
  workspaceRoot: string;
  symlinkPolicy: SymlinkPolicy;
  tools: ToolCatalog;
}
interface Source {
  declaration: ResourceManifestDeclaration;
  authority?: ResourceClaim[];
  unknownAuthority?: boolean;
  ancestorMkdir?: boolean;
}
const compare = (a: string, b: string) => (a < b ? -1 : a > b ? 1 : 0);
const claimKey = (claim: ResourceClaim) =>
  JSON.stringify([resourceKey(claim.resource), claim.mode]);
const claims = (manifest: ResourceManifest) =>
  [...manifest.reads, ...manifest.writes, ...manifest.produces]
    .sort((a, b) => compare(claimKey(a), claimKey(b)))
    .filter(
      (claim, index, items) => index === 0 || claimKey(claim) !== claimKey(items[index - 1]!),
    );
const object = (value: unknown): value is Record<string, JsonValue> =>
  value !== null && typeof value === "object" && !Array.isArray(value);
function deferred(value: unknown): boolean {
  return object(value) && Object.hasOwn(value, "$from");
}
function invalid(message: string): never {
  throw new CheaperError("INVALID_REQUEST", message);
}

/** safety claim と既知 read を分離し、未知 fallback から順序を発明しない。 */
export class WorkspaceResourceInference implements ResourceInference {
  constructor(private readonly options: Options) {}
  private normalize(declaration?: ResourceManifestDeclaration) {
    return normalizeResourceManifest(
      this.options.workspaceRoot,
      declaration,
      this.options.symlinkPolicy,
    );
  }

  private async source(item: ResourceInferenceJob): Promise<Source> {
    const processJob = item.job.kind === "tool" && item.job.tool === "process";
    if (item.process && !processJob) invalid("process declaration requires a process job");
    if (processJob) {
      const declaration = item.process;
      if (
        !declaration ||
        typeof declaration.task !== "string" ||
        declaration.task.length === 0 ||
        !Array.isArray(declaration.inputs) ||
        !Array.isArray(declaration.outputs)
      )
        invalid("process requires declared task inputs and outputs");
      const manifest = await this.normalize({
        reads: declaration.inputs,
        writes: declaration.outputs,
        produces: declaration.outputs,
      });
      return {
        declaration: {
          reads: declaration.inputs,
          writes: declaration.outputs,
          produces: declaration.outputs,
        },
        authority: manifest.produces,
      };
    }
    if (item.job.kind === "tool") {
      const metadata = this.options.tools.get(item.job.tool);
      return { declaration: metadata?.resources?.(structuredClone(item.job.input)) ?? {} };
    }
    const worker = item.worker;
    if (!worker) invalid("worker resource inference requires worker metadata");
    const mutation = worker.tools.some((tool) => tool === "write" || tool === "edit");
    if (!mutation) return { declaration: { writes: [], produces: [] }, authority: [] };
    const field = worker.cheaper.writeScope;
    if (!field) return { declaration: {}, unknownAuthority: true };
    const value = object(item.job.input) ? item.job.input[field] : undefined;
    if (deferred(value)) return { declaration: {}, unknownAuthority: true };
    if (!Array.isArray(value))
      throw new CheaperError("INVALID_INPUT", "writeScope must contain targets");
    const targets: ScopeDeclaration[] = [];
    let unknown = false;
    for (const target of value) {
      if (deferred(target) || (object(target) && deferred(target.path))) {
        unknown = true;
        continue;
      }
      const targetPath =
        typeof target === "string" ? target : object(target) ? target.path : undefined;
      if (typeof targetPath !== "string" || targetPath.length === 0)
        throw new CheaperError("INVALID_INPUT", "writeScope target must be a path");
      targets.push({ path: targetPath, scope: "exact" });
    }
    // authority は ownership 正規化で exact literal を検査する。安全性 glob 拡大を流用しない。
    const authority = (await this.normalize({ reads: [], writes: [], produces: targets })).produces;
    if (unknown) return { declaration: {}, unknownAuthority: true };
    return { declaration: { writes: targets }, authority, ancestorMkdir: targets.length > 0 };
  }

  async infer(
    jobs: Readonly<Record<string, ResourceInferenceJob>>,
  ): Promise<ResourceInferenceResult> {
    const ids = Object.keys(jobs).sort();
    const manifests: Record<string, ResourceManifest> = Object.create(null);
    const dependencyReads = new Map<string, ResourceClaim[]>();
    const diagnostics: ResourceInferenceDiagnostic[] = [];
    for (const id of ids) {
      const item = jobs[id]!;
      const source = await this.source(item);
      const known = await this.normalize(source.declaration);
      const explicit = item.job.resources;
      // core に渡す前に未 materialize binding を拒否する。
      if (explicit)
        for (const bucket of Object.values(explicit))
          for (const declaration of bucket)
            if (typeof declaration.path !== "string") invalid("resource path must be materialized");
      const declared = explicit as ResourceManifestDeclaration | undefined;
      const omitted = !declared || Object.keys(declared).length === 0;
      const manifest = await this.normalize(
        omitted
          ? item.process
            ? source.declaration
            : undefined
          : { ...source.declaration, ...declared },
      );
      for (const bucket of ["reads", "writes", "produces"] as const) {
        if (
          declared?.[bucket] !== undefined &&
          source.declaration[bucket] != null &&
          known[bucket].some(
            (access) => !manifest[bucket].some((scope) => claimContains(scope, access)),
          )
        )
          invalid(`declared ${bucket} underclaims known access: ${id}`);
      }
      if (source.authority) {
        if (
          declared?.writes !== undefined &&
          manifest.writes.some(
            (access) => !source.authority!.some((scope) => claimContains(scope, access)),
          )
        )
          invalid(`writes exceed authoritative scope: ${id}`);
        if (
          manifest.produces.some(
            (access) => !source.authority!.some((scope) => claimContains(scope, access)),
          )
        )
          invalid(`produces exceed authoritative scope: ${id}`);
      }
      if (
        source.unknownAuthority &&
        ((declared?.writes !== undefined && manifest.writes.length > 0) ||
          manifest.produces.length > 0 ||
          declared?.writes?.length === 0)
      )
        invalid(`unknown writeScope cannot certify declared mutation: ${id}`);
      if (source.ancestorMkdir) {
        const safety = await normalizeResourceClaim(
          this.options.workspaceRoot,
          { path: ".", scope: "subtree" },
          "exclusive",
          this.options.symlinkPolicy,
        );
        manifest.writes = (
          await normalizeResourceManifest(
            this.options.workspaceRoot,
            { ...manifest, writes: [...manifest.writes, safety] },
            this.options.symlinkPolicy,
          )
        ).writes;
        diagnostics.push({ kind: "fallback", job: id, reason: "worker-ancestor-mkdir" });
      }
      manifests[id] = manifest;
      dependencyReads.set(
        id,
        declared?.reads !== undefined
          ? manifest.reads
          : source.declaration.reads != null
            ? known.reads
            : [],
      );
      if (
        (omitted && !item.process) ||
        (source.declaration.reads == null && declared?.reads === undefined) ||
        (source.declaration.writes == null && declared?.writes === undefined)
      )
        diagnostics.push({
          kind: "fallback",
          job: id,
          reason: omitted ? "resources-omitted" : "unknown-access",
        });
    }
    const edges = new Map<string, DependencyEdge>();
    const conflicts: ResourceConflict[] = [];
    for (let i = 0; i < ids.length; i++)
      for (let j = i + 1; j < ids.length; j++) {
        const a = ids[i]!,
          b = ids[j]!;
        for (const left of manifests[a]!.produces)
          for (const right of manifests[b]!.produces) {
            if (resourcesOverlap(left.resource, right.resource))
              throw new CheaperError("INVALID_REQUEST", "duplicate resource producer", {
                details: { kind: "duplicate-producer", jobs: [a, b], claims: [left, right] },
              });
          }
        for (const left of claims(manifests[a]!))
          for (const right of claims(manifests[b]!)) {
            if (claimsConflict(left, right)) {
              const conflict: ResourceConflict = { jobs: [a, b], claims: [left, right] };
              conflicts.push(conflict);
              diagnostics.push({ kind: "conflict", ...conflict });
            }
          }
      }
    for (const from of ids)
      for (const to of ids) {
        if (from === to) continue;
        for (const producer of manifests[from]!.produces)
          for (const read of dependencyReads.get(to)!) {
            if (resourcesOverlap(producer.resource, read.resource)) {
              edges.set(JSON.stringify([from, to]), { from, to });
              diagnostics.push({ kind: "dependency", jobs: [from, to], claims: [producer, read] });
            }
          }
      }
    return { manifests, edges: [...edges.values()], conflicts, diagnostics };
  }
}
