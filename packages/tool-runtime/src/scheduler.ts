import {
  type ConcurrencyPolicy,
  normalizeResourceManifest,
  type ResourceManifestInput,
  resolveConcurrencyPolicy,
} from "@cheaper/core";
import type { CreateOperationOptions, OperationManager } from "./operation.js";
import { type LeasePreparation, type ResourceLease, ResourceManager } from "./resources.js";

/** 既知 tool は registry、process 等は explicit resources 宣言 seam を使う。 */
export interface OperationDeclaration {
  access: "shared" | "exclusive";
  resources?: ResourceManifestInput;
  resourceScoped?: boolean;
  parentLease?: ResourceLease | (() => ResourceLease | undefined) | undefined;
  prepare?: () => LeasePreparation | Promise<LeasePreparation>;
  readiness?: Promise<unknown> | undefined;
}
/** claims 待機に capacity を与えず、実際の settle まで両方を保持する。 */
export class ToolScheduler {
  private readonly policy: ConcurrencyPolicy;
  private running = 0;
  private exclusive = false;
  readonly resources: ResourceManager;
  constructor(
    private readonly manager: OperationManager,
    concurrency?: Partial<ConcurrencyPolicy>,
    resources?: ResourceManager,
  ) {
    this.policy = resolveConcurrencyPolicy(concurrency);
    this.resources = resources ?? new ResourceManager("/workspace", "deny");
  }
  async run<T>(
    options: CreateOperationOptions,
    declaration: OperationDeclaration,
    execute: (signal: AbortSignal) => Promise<T>,
  ): Promise<T> {
    const operation = this.manager.create(options);
    if (operation.signal.aborted || this.manager.get(operation.id).state !== "queued")
      throw operation.signal.reason;
    const snapshot =
      declaration.resources === undefined ? undefined : structuredClone(declaration.resources);
    const access =
      this.policy.mutations === "per-resource" &&
      (declaration.resourceScoped || snapshot !== undefined)
        ? "shared"
        : declaration.access;
    let ready = declaration.readiness === undefined;
    if (declaration.readiness)
      void declaration.readiness.then(
        () => {
          ready = true;
          this.resources.notify();
        },
        (error) => {
          if (this.manager.get(operation.id).state === "queued")
            this.manager.cancel(operation.id, error);
        },
      );
    const prepare = (): LeasePreparation | Promise<LeasePreparation> => {
      const value = declaration.prepare
        ? declaration.prepare()
        : snapshot !== undefined
          ? normalizeResourceManifest(
              this.resources.root,
              snapshot,
              this.resources.symlinkPolicy,
            ).then((manifest) => ({
              claims: [...manifest.reads, ...manifest.writes, ...manifest.produces],
              readStrictness: "record" as const,
            }))
          : {
              claims: [
                {
                  resource: {
                    kind: "workspace" as const,
                    path: this.resources.root,
                    scope: "subtree" as const,
                  },
                  mode: declaration.access,
                },
              ],
              readStrictness: "record" as const,
            };
      const record = (prepared: LeasePreparation) => {
        if (this.manager.get(operation.id).state === "queued")
          this.manager.setResourceClaims(operation.id, prepared.claims);
        return prepared;
      };
      return value instanceof Promise ? value.then(record) : record(value);
    };
    try {
      return await this.resources.request({
        ownerId: options.ownerId,
        kind: "operation",
        signal: operation.signal,
        parentLease: declaration.parentLease,
        access,
        prepare,
        canStart: () =>
          ready &&
          !this.exclusive &&
          this.running < this.policy.maxOperations &&
          (access !== "exclusive" || this.running === 0),
        start: (lease) => {
          this.manager.start(operation.id);
          this.running++;
          if (access === "exclusive") this.exclusive = true;
          return (async () => {
            try {
              const result = await execute(operation.signal);
              this.manager.complete(operation.id, result);
              return result;
            } catch (error) {
              this.manager.fail(operation.id, error);
              throw error;
            } finally {
              this.running--;
              if (access === "exclusive") this.exclusive = false;
              this.resources.release(lease);
              this.resources.notify();
            }
          })();
        },
      });
    } catch (error) {
      // preparation の失敗は queued→cancelled。六状態の契約を拡張しない。
      if (this.manager.get(operation.id).state === "queued")
        this.manager.cancel(operation.id, error);
      throw error;
    }
  }
}
