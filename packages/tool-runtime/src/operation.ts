import { CheaperError, type ResourceClaim } from "@cheaper/core";
import { collectExecutionMetrics, type ExecutionMetrics } from "./execution-metrics.js";

export type OperationState =
  | "queued"
  | "running"
  | "completed"
  | "failed"
  | "cancelled"
  | "timed_out";

const transitions: Record<OperationState, readonly OperationState[]> = {
  queued: ["running", "cancelled", "timed_out"],
  running: ["completed", "failed", "cancelled", "timed_out"],
  completed: [],
  failed: [],
  cancelled: [],
  timed_out: [],
};

export function assertOperationTransition(from: OperationState, to: OperationState): void {
  if (!transitions[from].includes(to)) {
    throw new CheaperError("INVALID_REQUEST", `invalid operation transition: ${from} -> ${to}`);
  }
}

export interface OperationError {
  readonly name: string;
  readonly message: string;
  readonly code?: string;
}

export interface OperationRecord {
  readonly id: string;
  readonly ownerId: string;
  readonly kind: string;
  readonly toolName?: string;
  readonly state: OperationState;
  readonly createdAt: number;
  readonly startedAt?: number;
  readonly finishedAt?: number;
  readonly signal: AbortSignal;
  readonly resourceClaims?: readonly ResourceClaim[];
  readonly cancellation?: {
    readonly requestedAt: number;
    readonly state: "cancelled" | "timed_out";
    readonly reason: OperationError;
  };
  /** unsettled は grace 超過時の明示分類。実停止や terminal の意味を持たない。 */
  readonly settlement: "pending" | "settled" | "unsettled";
  readonly owned: boolean;
  readonly detachedAt?: number;
  readonly result?: unknown;
  readonly error?: OperationError;
}

export interface CreateOperationOptions {
  ownerId: string;
  kind: string;
  toolName?: string;
  resourceClaims?: readonly ResourceClaim[];
  /** owner scope と call-context の composite signal を runtime から渡す。 */
  signal?: AbortSignal;
}

export interface OperationManagerOptions {
  signal?: AbortSignal;
  abortGraceMs?: number;
  /** timestamp は monotonic。同じ clock を invocation の全 owner が共有する。 */
  clock?: () => number;
}

type MutableRecord = { -readonly [K in keyof OperationRecord]: OperationRecord[K] };
interface Entry {
  record: MutableRecord;
  controller: AbortController;
  settled: Promise<void>;
  resolveSettled: () => void;
  cleanup: () => void;
}

/** ADR-034zT6bqTxOvK1nP0ELSyR / 共有 ADR D1/D3/D7。scheduler の責務は持たない。 */
export class OperationManager {
  private readonly entries = new Map<string, Entry>();
  private readonly closedOwners = new Set<string>();
  private readonly clock: () => number;
  private readonly graceMs: number;
  private readonly rootSignal: AbortSignal | undefined;
  private readonly onRootAbort: () => void;
  private closed = false;
  private nextId = 0;
  private disposal: Promise<void> | undefined;
  private running = 0;
  private maxConcurrency = 0;
  private readonly ownerConcurrency = new Map<string, { running: number; maximum: number }>();

  constructor(options: OperationManagerOptions = {}) {
    this.graceMs = validateGrace(options.abortGraceMs ?? 5000);
    this.clock = options.clock ?? (() => performance.now());
    this.rootSignal = options.signal;
    this.onRootAbort = () => this.cancelAll(this.rootSignal?.reason);
    this.rootSignal?.addEventListener("abort", this.onRootAbort, { once: true });
    if (this.rootSignal?.aborted) this.onRootAbort();
  }

  create(options: CreateOperationOptions): OperationRecord {
    if (this.closed || this.closedOwners.has(options.ownerId)) {
      throw new CheaperError("INVALID_REQUEST", "Operation aborted: admission closed");
    }
    if (!options.ownerId || !options.kind) {
      throw new CheaperError("INVALID_REQUEST", "operation owner and kind must be nonempty");
    }
    const controller = new AbortController();
    const signal = options.signal
      ? AbortSignal.any([controller.signal, options.signal])
      : controller.signal;
    let resolveSettled!: () => void;
    const record: MutableRecord = {
      id: `op-${++this.nextId}`,
      ownerId: options.ownerId,
      kind: options.kind,
      ...(options.toolName !== undefined ? { toolName: options.toolName } : {}),
      ...(options.resourceClaims !== undefined
        ? { resourceClaims: copyClaims(options.resourceClaims) }
        : {}),
      state: "queued",
      createdAt: this.clock(),
      signal,
      owned: true,
      settlement: "pending",
    };
    const onAbort = () => {
      if (!isTerminal(record.state)) this.requestCancellation(entry, signal.reason);
    };
    const entry: Entry = {
      record,
      controller,
      settled: new Promise<void>((resolve) => {
        resolveSettled = resolve;
      }),
      resolveSettled: () => resolveSettled(),
      cleanup: () => signal.removeEventListener("abort", onAbort),
    };
    this.entries.set(record.id, entry);
    signal.addEventListener("abort", onAbort, { once: true });
    if (signal.aborted) onAbort();
    return this.get(record.id);
  }

  queue(options: CreateOperationOptions): OperationRecord {
    return this.create(options);
  }

  start(id: string): OperationRecord {
    const entry = this.entry(id);
    assertOperationTransition(entry.record.state, "running");
    entry.record.state = "running";
    entry.record.startedAt = this.clock();
    this.running++;
    this.maxConcurrency = Math.max(this.maxConcurrency, this.running);
    const owner = this.ownerConcurrency.get(entry.record.ownerId) ?? { running: 0, maximum: 0 };
    owner.running++;
    owner.maximum = Math.max(owner.maximum, owner.running);
    this.ownerConcurrency.set(entry.record.ownerId, owner);
    return this.get(id);
  }

  complete(id: string, result: unknown): OperationRecord {
    const entry = this.entry(id);
    assertOperationTransition(entry.record.state, "completed");
    entry.record.result = result;
    this.finish(entry, entry.record.cancellation?.state ?? "completed");
    return this.get(id);
  }

  fail(id: string, error: unknown): OperationRecord {
    const entry = this.entry(id);
    assertOperationTransition(entry.record.state, "failed");
    entry.record.error = describeError(error);
    this.finish(entry, entry.record.cancellation?.state ?? "failed");
    return this.get(id);
  }

  cancel(id: string, reason: unknown = new Error("Operation cancelled")): OperationRecord {
    const entry = this.entry(id);
    assertOperationTransition(entry.record.state, cancellationState(reason));
    this.requestCancellation(entry, reason);
    return this.get(id);
  }

  cancelOwner(ownerId: string, reason: unknown = new Error("Operation cancelled")): void {
    this.closedOwners.add(ownerId);
    for (const entry of this.entries.values()) {
      if (entry.record.ownerId === ownerId && !isTerminal(entry.record.state))
        this.requestCancellation(entry, reason);
    }
  }

  cancelAll(reason: unknown = new Error("Operation cancelled")): void {
    this.closed = true;
    for (const entry of this.entries.values()) {
      if (!isTerminal(entry.record.state)) this.requestCancellation(entry, reason);
    }
  }

  /** lifecycle の低水準 API。admission bound は持たず、tool 実行には ToolScheduler を使う。 */
  async run<T>(
    options: CreateOperationOptions,
    execute: (signal: AbortSignal) => Promise<T>,
  ): Promise<T> {
    const operation = this.create(options);
    if (operation.signal.aborted) throw operation.signal.reason;
    this.start(operation.id);
    try {
      const result = await execute(operation.signal);
      this.complete(operation.id, result);
      return result;
    } catch (error) {
      this.fail(operation.id, error);
      throw error;
    }
  }

  get(id: string): OperationRecord {
    const { record } = this.entry(id);
    return {
      ...record,
      ...(record.resourceClaims ? { resourceClaims: copyClaims(record.resourceClaims) } : {}),
      ...(record.cancellation
        ? { cancellation: { ...record.cancellation, reason: { ...record.cancellation.reason } } }
        : {}),
      ...(record.error ? { error: { ...record.error } } : {}),
    };
  }

  records(ownerId?: string): readonly OperationRecord[] {
    return [...this.entries.values()]
      .filter(({ record }) => ownerId === undefined || record.ownerId === ownerId)
      .map(({ record }) => this.get(record.id));
  }

  /** D3: owner-local または invocation 全体の集合から同じ定義で算出する。 */
  executionMetrics(ownerId?: string): ExecutionMetrics {
    const snapshotAt = this.clock();
    const maximum =
      ownerId === undefined
        ? this.maxConcurrency
        : (this.ownerConcurrency.get(ownerId)?.maximum ?? 0);
    return collectExecutionMetrics(this.records(ownerId), snapshotAt, maximum);
  }

  async drain(ownerId?: string, abortGraceMs: number = this.graceMs): Promise<void> {
    const grace = validateGrace(abortGraceMs);
    if (ownerId === undefined) this.cancelAll();
    else this.cancelOwner(ownerId);
    const pending = [...this.entries.values()].filter(
      ({ record }) => record.owned && (ownerId === undefined || record.ownerId === ownerId),
    );
    if (pending.length === 0) return;
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      await Promise.race([
        Promise.all(pending.map((entry) => entry.settled)),
        new Promise<void>((resolve) => {
          timer = setTimeout(resolve, grace);
        }),
      ]);
    } finally {
      if (timer !== undefined) clearTimeout(timer);
      for (const entry of pending) {
        if (!entry.record.owned) continue;
        entry.record.owned = false;
        entry.record.settlement = "unsettled";
        entry.record.detachedAt = this.clock();
        entry.cleanup();
      }
    }
  }

  dispose(): Promise<void> {
    this.disposal ??= this.drain().finally(() => {
      this.rootSignal?.removeEventListener("abort", this.onRootAbort);
    });
    return this.disposal;
  }

  private entry(id: string): Entry {
    const entry = this.entries.get(id);
    if (!entry) throw new CheaperError("INVALID_REQUEST", `unknown operation: ${id}`);
    return entry;
  }

  private requestCancellation(entry: Entry, reason: unknown): void {
    const { record } = entry;
    if (record.cancellation) return;
    record.cancellation = {
      requestedAt: this.clock(),
      state: cancellationState(reason),
      reason: describeError(reason),
    };
    if (record.state === "queued") this.finish(entry, record.cancellation.state);
    entry.controller.abort(reason);
  }

  private finish(entry: Entry, state: OperationState): void {
    assertOperationTransition(entry.record.state, state);
    if (entry.record.state === "running") {
      this.running--;
      const owner = this.ownerConcurrency.get(entry.record.ownerId);
      if (owner) owner.running--;
    }
    entry.record.state = state;
    entry.record.finishedAt = this.clock();
    entry.record.owned = false;
    entry.record.settlement = "settled";
    entry.cleanup();
    entry.resolveSettled();
  }
}

function isTerminal(state: OperationState): boolean {
  return state !== "queued" && state !== "running";
}

function cancellationState(reason: unknown): "cancelled" | "timed_out" {
  const error = describeError(reason);
  return error.code === "TIMEOUT" || error.code === "TimeoutError" || error.name === "TimeoutError"
    ? "timed_out"
    : "cancelled";
}

function describeError(error: unknown): OperationError {
  const value = error && typeof error === "object" ? (error as Record<string, unknown>) : undefined;
  return {
    name: typeof value?.name === "string" ? value.name : "Error",
    message: typeof value?.message === "string" ? value.message : String(error),
    ...(typeof value?.code === "string" ? { code: value.code } : {}),
  };
}

function copyClaims(claims: readonly ResourceClaim[]): ResourceClaim[] {
  return claims.map((claim) => ({ ...claim, resource: { ...claim.resource } }));
}

function validateGrace(grace: number): number {
  if (!Number.isFinite(grace) || grace < 0)
    throw new CheaperError("INVALID_REQUEST", "abort grace must be a finite nonnegative number");
  return grace;
}
