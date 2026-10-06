import { realpathSync } from "node:fs";
import {
  CheaperError,
  claimContains,
  claimsConflict,
  type ResourceClaim,
  resourceKey,
  type SymlinkPolicy,
} from "@cheaper/core";
import type { ReadStrictness } from "./access.js";

declare const leaseIdentity: unique symbol;
/** manager が発行した identity のみ有効。公開値に claim/owner の可変状態を載せない。 */
export interface ResourceLease {
  readonly [leaseIdentity]: true;
}
export interface LeasePreparation {
  claims: readonly ResourceClaim[];
  readStrictness: ReadStrictness;
}
interface LeaseEntry extends LeasePreparation {
  ownerId: string;
  kind: "job" | "operation";
  parent?: ResourceLease;
  children: number;
  closing: boolean;
}
export interface ResourceRequest<T> {
  ownerId: string;
  kind: "job" | "operation";
  signal?: AbortSignal | undefined;
  parentLease?: ResourceLease | (() => ResourceLease | undefined) | undefined;
  /** serialized admission の全域 gate。job は operation capacity を消費しない。 */
  access?: "shared" | "exclusive" | undefined;
  prepare: () => LeasePreparation | Promise<LeasePreparation>;
  canStart?: () => boolean;
  start: (lease: ResourceLease) => T;
}
interface Waiter {
  options: ResourceRequest<unknown>;
  prepared?: LeasePreparation;
  resolve: (value: unknown) => void;
  reject: (reason: unknown) => void;
  cleanup: () => void;
}
export function invalidResource(message: string): CheaperError {
  return new CheaperError("INVALID_REQUEST", `invalid resource lease: ${message}`);
}
const conflict = (a: readonly ResourceClaim[], b: readonly ResourceClaim[]) =>
  a.some((x) => b.some((y) => claimsConflict(x, y)));
const covers = (a: readonly ResourceClaim[], b: readonly ResourceClaim[]) =>
  b.every((x) => a.some((y) => claimContains(y, x)));

/** invocation 内の job/operation を一つの提出順で atomic に調停する。 */
export class ResourceManager {
  private readonly leases = new Map<ResourceLease, LeaseEntry>();
  private readonly queue: Waiter[] = [];
  private pumping = false;
  private deferred = false;
  readonly root: string;
  constructor(
    root: string,
    readonly symlinkPolicy: SymlinkPolicy,
  ) {
    // 実在 root の identity を同期に確定。injected fake-root の従来 admission は維持する。
    try {
      this.root = realpathSync(root);
    } catch {
      this.root = root;
    }
  }

  parent(lease: ResourceLease, ownerId: string): LeasePreparation {
    const entry = this.leases.get(lease);
    if (entry?.kind !== "job" || entry.closing)
      throw invalidResource("foreign, released or closing parent");
    if (entry.ownerId !== ownerId) throw invalidResource("parent owner mismatch");
    return { claims: copyClaims(entry.claims), readStrictness: entry.readStrictness };
  }

  request<T>(options: ResourceRequest<T>): Promise<T> {
    if (options.signal?.aborted) return Promise.reject(options.signal.reason);
    return new Promise<T>((resolve, reject) => {
      const waiter: Waiter = {
        options,
        resolve: (value) => resolve(value as T),
        reject,
        cleanup: () => options.signal?.removeEventListener("abort", onAbort),
      };
      const onAbort = () => {
        if (!this.queue.includes(waiter)) return;
        this.remove(waiter);
        reject(options.signal?.reason);
        this.deferred = true;
        queueMicrotask(() => {
          this.deferred = false;
          this.pump();
        });
      };
      this.queue.push(waiter); // normalization より前に同期提出順を確定する。
      options.signal?.addEventListener("abort", onAbort, { once: true });
      const prepared = (value: LeasePreparation) => {
        if (!this.queue.includes(waiter)) return;
        waiter.prepared = {
          claims: copyClaims(value.claims),
          readStrictness: value.readStrictness,
        };
        this.pump();
        queueMicrotask(() => this.pump());
      };
      const failed = (error: unknown) => {
        if (this.queue.includes(waiter)) {
          this.remove(waiter);
          reject(error);
          this.pump();
          queueMicrotask(() => this.pump());
        }
      };
      try {
        const value = options.prepare();
        if (value instanceof Promise) void value.then(prepared, failed);
        else prepared(value);
      } catch (error) {
        failed(error);
      }
    });
  }

  release(lease: ResourceLease): void {
    const entry = this.leases.get(lease);
    if (!entry) return;
    entry.closing = true;
    if (entry.children > 0) return;
    this.leases.delete(lease);
    if (entry.parent) {
      const parent = this.leases.get(entry.parent);
      if (parent) {
        parent.children--;
        if (parent.closing) this.release(entry.parent);
      }
    }
    this.pump();
  }

  /** capacity の解放も同じ仲裁点へ通知する。 */
  notify(): void {
    this.pump();
  }

  releaseJobs(): void {
    for (const [lease, entry] of this.leases) if (entry.kind === "job") this.release(lease);
  }

  private remove(waiter: Waiter): void {
    const index = this.queue.indexOf(waiter);
    if (index >= 0) this.queue.splice(index, 1);
    waiter.cleanup();
  }
  private pump(): void {
    if (this.pumping || this.deferred) return;
    this.pumping = true;
    try {
      for (const waiter of [...this.queue]) {
        if (!this.queue.includes(waiter) || !waiter.prepared) continue;
        const { options, prepared } = waiter;
        const parentLease =
          typeof options.parentLease === "function" ? options.parentLease() : options.parentLease;
        if (options.signal?.aborted) {
          this.remove(waiter);
          waiter.reject(options.signal.reason);
          continue;
        }
        let parent: LeasePreparation | undefined;
        try {
          if (parentLease) {
            parent = this.parent(parentLease, options.ownerId);
            if (!covers(parent.claims, prepared.claims))
              throw invalidResource("child claims are outside parent");
            if (parent.readStrictness !== prepared.readStrictness)
              throw invalidResource("parent read strictness mismatch");
          }
          const active = [...this.leases.entries()].filter(([token]) => token !== parentLease);
          if (
            parent &&
            active.some(
              ([, entry]) => entry.kind === "job" && conflict(entry.claims, prepared.claims),
            )
          )
            throw invalidResource("borrow conflicts with foreign job token");
          if (active.some(([, entry]) => conflict(entry.claims, prepared.claims))) continue;
          const earlier = this.queue.slice(0, this.queue.indexOf(waiter));
          if (
            earlier.some((previous) => {
              // 未正規化の先行要求は scope 不明なので先行する。
              if (!previous.prepared) return true;
              const previousParent =
                typeof previous.options.parentLease === "function"
                  ? previous.options.parentLease()
                  : previous.options.parentLease;
              const blockedByParent =
                parent &&
                previousParent !== parentLease &&
                conflict(parent.claims, previous.prepared.claims);
              if (blockedByParent) return false;
              const globalConflict =
                options.access &&
                previous.options.access &&
                (options.access === "exclusive" || previous.options.access === "exclusive");
              return globalConflict || conflict(previous.prepared.claims, prepared.claims);
            })
          )
            continue;
          if (options.canStart && !options.canStart()) continue;
          const token = Object.freeze({}) as ResourceLease;
          this.leases.set(token, {
            ...prepared,
            ownerId: options.ownerId,
            kind: options.kind,
            children: 0,
            closing: false,
            ...(parentLease ? { parent: parentLease } : {}),
          });
          if (parentLease) {
            const entry = this.leases.get(parentLease);
            if (entry) entry.children++;
          }
          this.remove(waiter);
          try {
            waiter.resolve(options.start(token));
          } catch (error) {
            this.release(token);
            waiter.reject(error);
          }
        } catch (error) {
          this.remove(waiter);
          waiter.reject(error);
        }
      }
    } finally {
      this.pumping = false;
    }
  }
}

/** 並べ替え/重複除去と immutable 内部 snapshot を同じ境界で行う。 */
function copyClaims(claims: readonly ResourceClaim[]): ResourceClaim[] {
  const unique = new Map<string, ResourceClaim>();
  for (const claim of claims) {
    const key = JSON.stringify([resourceKey(claim.resource), claim.mode]);
    unique.set(key, Object.freeze({ ...claim, resource: Object.freeze({ ...claim.resource }) }));
  }
  return [...unique.entries()]
    .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
    .map(([, claim]) => claim);
}
