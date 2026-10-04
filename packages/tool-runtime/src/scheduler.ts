import { type ConcurrencyPolicy, resolveConcurrencyPolicy } from "@cheaper/core";
import type { CreateOperationOptions, OperationManager } from "./operation.js";

/** 既知 tool の access は registry が決める。process 等はこの宣言 seam を使う。 */
export interface OperationDeclaration {
  access: "shared" | "exclusive";
}

interface Waiter {
  id: string;
  signal: AbortSignal;
  access: OperationDeclaration["access"];
  admit: () => void;
  reject: (reason: unknown) => void;
  cleanup: () => void;
}

/** invocation 全 owner に対する FIFO admission。capacity は実際の settle まで保持する。 */
export class ToolScheduler {
  private readonly policy: ConcurrencyPolicy;
  private readonly queue: Waiter[] = [];
  private running = 0;
  private exclusive = false;
  private pumpDeferred = false;

  constructor(
    private readonly manager: OperationManager,
    concurrency?: Partial<ConcurrencyPolicy>,
  ) {
    this.policy = resolveConcurrencyPolicy(concurrency);
  }

  async run<T>(
    options: CreateOperationOptions,
    declaration: OperationDeclaration,
    execute: (signal: AbortSignal) => Promise<T>,
  ): Promise<T> {
    const operation = this.manager.create(options);
    if (operation.signal.aborted || this.manager.get(operation.id).state !== "queued")
      throw operation.signal.reason;
    return new Promise<T>((resolve, reject) => {
      const waiter: Waiter = {
        id: operation.id,
        signal: operation.signal,
        access: declaration.access,
        reject,
        cleanup: () => operation.signal.removeEventListener("abort", onAbort),
        admit: () => {
          void (async () => {
            try {
              const result = await execute(operation.signal);
              this.manager.complete(operation.id, result);
              resolve(result);
            } catch (error) {
              this.manager.fail(operation.id, error);
              reject(error);
            } finally {
              this.running--;
              if (waiter.access === "exclusive") this.exclusive = false;
              this.pump();
            }
          })();
        },
      };
      const onAbort = () => {
        const index = this.queue.indexOf(waiter);
        if (index < 0) return;
        this.queue.splice(index, 1);
        waiter.cleanup();
        reject(operation.signal.reason);
        // root/owner cancellation の同期伝播が完了してから次の admission を行う。
        this.pumpDeferred = true;
        queueMicrotask(() => {
          this.pumpDeferred = false;
          this.pump();
        });
      };
      operation.signal.addEventListener("abort", onAbort, { once: true });
      this.queue.push(waiter);
      this.pump();
    });
  }

  private pump(): void {
    while (this.queue.length > 0) {
      if (this.pumpDeferred) return;
      const waiter = this.queue[0];
      if (!waiter) return;
      if (waiter.signal.aborted || this.manager.get(waiter.id).state !== "queued") {
        this.queue.shift();
        waiter.cleanup();
        waiter.reject(waiter.signal.reason);
        continue;
      }
      if (this.exclusive || this.running >= this.policy.maxOperations) return;
      // per-resource も #61 の lease 導入までは conservatively exclusive。
      if (waiter.access === "exclusive" && this.running > 0) return;
      this.queue.shift();
      waiter.cleanup();
      this.manager.start(waiter.id);
      this.running++;
      this.exclusive = waiter.access === "exclusive";
      waiter.admit();
    }
  }
}
