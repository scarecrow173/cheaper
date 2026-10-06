import { randomUUID } from "node:crypto";
import { CheaperError } from "@cheaper/core";
import type {
  ArtifactMetadata,
  ArtifactPut,
  ArtifactReader,
  ArtifactReadRequest,
  ArtifactReadResult,
  ArtifactReference,
  ArtifactSource,
} from "@cheaper/tool-runtime";

export type {
  ArtifactMetadata,
  ArtifactPorts,
  ArtifactProducer,
  ArtifactPublisher,
  ArtifactPut,
  ArtifactReader,
  ArtifactReadRequest,
  ArtifactReadResult,
  ArtifactReference,
  ArtifactSource,
  ReadSource,
  WorkspaceSource,
} from "@cheaper/tool-runtime";

/** cap の供給元は owning invocation。RuntimePolicy schema 配線は engine slice が所有。 */
export interface ArtifactStoreOptions {
  maxArtifactBytes: number;
  maxTotalBytes: number;
  maxPreviewBytes: number;
  maxReadBytes: number;
  signal?: AbortSignal;
}

export interface ArtifactProjectionOptions {
  includePreview?: boolean;
}

interface Entry {
  reference: ArtifactReference;
  data: Buffer;
}

const UUID = "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}";
const ARTIFACT_URI = new RegExp(`^artifact://(${UUID})/(${UUID})$`);

function project(
  ref: ArtifactReference,
  availability: ArtifactMetadata["availability"],
  options: ArtifactProjectionOptions,
): ArtifactMetadata {
  return Object.freeze({
    artifact: ref.artifact,
    runId: ref.runId,
    producer: Object.freeze({ jobId: ref.producer.jobId, name: ref.producer.name }),
    bytes: ref.bytes,
    truncated: ref.truncated,
    availability,
    ...(options.includePreview !== false ? { preview: ref.preview } : {}),
  });
}

/** final envelope 向け。cleanup 後の reference は metadata であり取得可能性を宣言しない。 */
export function projectArtifactMetadata(
  reference: ArtifactReference,
  options: ArtifactProjectionOptions = {},
): ArtifactMetadata {
  return project(reference, "unavailable", options);
}

function boundedPreview(data: Buffer, cap: number): { preview: string; truncated: boolean } {
  // decoded text 全文を作らず、最大 cap source bytes だけを decode。
  // stream は cap で切れた multibyte tail を flush しない。真の EOF は replacement を使う。
  const decoder = new TextDecoder("utf-8", { ignoreBOM: true });
  const text = decoder.decode(data.subarray(0, cap), { stream: data.length > cap });
  let preview = "";
  let bytes = 0;
  for (const char of text) {
    const size = Buffer.byteLength(char);
    if (bytes + size > cap) break;
    preview += char;
    bytes += size;
  }
  return { preview, truncated: data.length > cap || preview.length < text.length };
}

/** D4/D7: filesystem を持たず、1 invocation が所有する in-memory bounded store。 */
export class RunArtifactStore implements ArtifactReader {
  readonly runId = randomUUID();
  readonly reader: ArtifactReader;
  #limits: Readonly<Omit<ArtifactStoreOptions, "signal">>;
  #entries = new Map<string, Entry>();
  #producers = new Set<string>();
  #bytes = 0;
  #closed = false;
  #aborted = false;
  #signal: AbortSignal | undefined;
  #onAbort = () => {
    this.#aborted = true;
    this.dispose();
  };

  constructor(options: ArtifactStoreOptions) {
    const { maxArtifactBytes, maxTotalBytes, maxPreviewBytes, maxReadBytes } = options;
    const caps = { maxArtifactBytes, maxTotalBytes, maxPreviewBytes, maxReadBytes };
    for (const [field, cap] of Object.entries(caps)) {
      if (!Number.isSafeInteger(cap) || cap < 0 || (field === "maxReadBytes" && cap === 0)) {
        throw new CheaperError("INVALID_REQUEST", `invalid artifact cap: ${field}`);
      }
    }
    this.#limits = Object.freeze(caps);
    this.#signal = options.signal;
    if (this.#signal?.aborted) throw new CheaperError("ABORTED", "artifact invocation aborted");
    this.#signal?.addEventListener("abort", this.#onAbort, { once: true });
    this.reader = Object.freeze({
      read: (source: ArtifactSource, request: ArtifactReadRequest) => this.read(source, request),
    });
  }

  get stats(): { bytes: number; artifacts: number; closed: boolean } {
    return { bytes: this.#bytes, artifacts: this.#entries.size, closed: this.#closed };
  }

  get maxArtifactBytes(): number {
    return this.#limits.maxArtifactBytes;
  }

  #assertOpen(): void {
    if (this.#closed) {
      throw new CheaperError(
        this.#aborted ? "ABORTED" : "INVALID_REQUEST",
        "artifact store is closed",
      );
    }
  }

  put({ producer, data }: ArtifactPut): ArtifactReference {
    this.#assertOpen();
    if (
      !producer ||
      typeof producer.jobId !== "string" ||
      producer.jobId.length === 0 ||
      typeof producer.name !== "string" ||
      producer.name.length === 0 ||
      (typeof data !== "string" && !(data instanceof Uint8Array))
    ) {
      throw new CheaperError("INVALID_INPUT", "invalid artifact value or producer");
    }
    const producerKey = JSON.stringify([producer.jobId, producer.name]);
    if (this.#producers.has(producerKey)) {
      throw new CheaperError("INVALID_INPUT", "duplicate artifact producer/name");
    }
    const bytes = typeof data === "string" ? Buffer.byteLength(data, "utf8") : data.byteLength;
    if (bytes > this.#limits.maxArtifactBytes || bytes > this.#limits.maxTotalBytes - this.#bytes) {
      throw new CheaperError("POLICY_VIOLATION", "artifact storage limit exceeded", {
        details: { bytes, storedBytes: this.#bytes },
      });
    }
    const copy = Buffer.from(data);
    const reference = Object.freeze({
      artifact: `artifact://${this.runId}/${randomUUID()}`,
      runId: this.runId,
      producer: Object.freeze({ jobId: producer.jobId, name: producer.name }),
      bytes,
      ...boundedPreview(copy, this.#limits.maxPreviewBytes),
    });
    this.#entries.set(reference.artifact, { reference, data: copy });
    this.#producers.add(producerKey);
    this.#bytes += bytes;
    return reference;
  }

  read(source: ArtifactSource, request: ArtifactReadRequest): ArtifactReadResult {
    this.#assertOpen();
    if (source?.kind !== "artifact" || typeof source.reference?.artifact !== "string") {
      throw new CheaperError("INVALID_INPUT", "artifact source required");
    }
    const ref = source.reference;
    const uri = ARTIFACT_URI.exec(ref.artifact);
    if (!uri || uri[0] !== ref.artifact || uri[1] !== this.runId || ref.runId !== this.runId) {
      throw new CheaperError("INVALID_INPUT", "invalid or foreign artifact identity");
    }
    const entry = this.#entries.get(ref.artifact);
    if (
      !entry ||
      ref.producer?.jobId !== entry.reference.producer.jobId ||
      ref.producer?.name !== entry.reference.producer.name ||
      ref.bytes !== entry.reference.bytes ||
      ref.preview !== entry.reference.preview ||
      ref.truncated !== entry.reference.truncated
    ) {
      throw new CheaperError("INVALID_INPUT", "unknown or inconsistent artifact reference");
    }
    const { offset, maxBytes } = request;
    if (
      !Number.isSafeInteger(offset) ||
      offset < 0 ||
      offset > entry.data.length ||
      !Number.isSafeInteger(maxBytes) ||
      maxBytes <= 0
    ) {
      throw new CheaperError("INVALID_INPUT", "invalid artifact read range");
    }
    if (maxBytes > this.#limits.maxReadBytes) {
      throw new CheaperError("POLICY_VIOLATION", "artifact read limit exceeded");
    }
    const nextOffset = offset + Math.min(maxBytes, entry.data.length - offset);
    return {
      data: new Uint8Array(entry.data.subarray(offset, nextOffset)),
      offset,
      nextOffset,
      eof: nextOffset === entry.data.length,
    };
  }

  metadata(options: ArtifactProjectionOptions = {}): ArtifactMetadata[] {
    return [...this.#entries.values()].map(({ reference }) =>
      project(reference, "available", options),
    );
  }

  toJSON() {
    return {
      runId: this.runId,
      closed: this.#closed,
      bytes: this.#bytes,
      artifacts: this.metadata(),
    };
  }

  /** abort clear は tool drain と独立。consumer の次 read は直ちに失敗する。 */
  dispose(): void {
    if (this.#closed) return;
    this.#closed = true;
    this.#signal?.removeEventListener("abort", this.#onAbort);
    this.#signal = undefined;
    this.#entries.clear();
    this.#producers.clear();
    this.#bytes = 0;
  }
}

/** engine 向け所有 seam。borrowed runtime/consumer 自体の drain は行わない。 */
export async function withArtifactStore<T>(
  options: ArtifactStoreOptions,
  run: (store: RunArtifactStore) => T | Promise<T>,
): Promise<T> {
  const signal = options.signal;
  const store = new RunArtifactStore(options);
  try {
    const result = await run(store);
    if (signal?.aborted) throw new CheaperError("ABORTED", "artifact invocation aborted");
    return result;
  } finally {
    store.dispose();
  }
}
