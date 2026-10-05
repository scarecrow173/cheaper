/** Artifact identity/provenance は plain JSON data。workspace path の意味を持たない。 */
export type ArtifactProducer = Readonly<{ jobId: string; name: string }>;

export type ArtifactReference = Readonly<{
  artifact: string;
  runId: string;
  producer: ArtifactProducer;
  /** 保存した source byte 数。preview の byte 数ではない。 */
  bytes: number;
  preview: string;
  truncated: boolean;
}>;

export type ArtifactSource = Readonly<{ kind: "artifact"; reference: ArtifactReference }>;
export type WorkspaceSource = Readonly<{ kind: "workspace"; path: string }>;
/** consumer が明示 dispatch する。artifact URI を workspace resolve してはならない。 */
export type ReadSource = ArtifactSource | WorkspaceSource;

export interface ArtifactReadRequest {
  /** source byte 基準。0-indexed、EOF と同値まで許可。 */
  offset: number;
  /** 正数。invocation の maxReadBytes を超える要求は拒否する。 */
  maxBytes: number;
}

export interface ArtifactReadResult {
  /** 明示 bounded materialization の copy。返却後は caller 所有。 */
  data: Uint8Array;
  offset: number;
  nextOffset: number;
  eof: boolean;
}

/** invocation runtime が借用する port。store/cache/lifetime の所有権は持たない。 */
export interface ArtifactReader {
  read(source: ArtifactSource, request: ArtifactReadRequest): ArtifactReadResult;
}

export interface ArtifactPut {
  producer: ArtifactProducer;
  data: string | Uint8Array;
}

/** scope は借用する。producer.jobId は runtime owner が固定する。 */
export interface ArtifactPublisher {
  readonly maxArtifactBytes: number;
  put(value: ArtifactPut): ArtifactReference;
}

export interface ArtifactPorts {
  reader?: ArtifactReader | undefined;
  publisher?: ArtifactPublisher | undefined;
}

export type ArtifactMetadata = Readonly<
  Omit<ArtifactReference, "preview"> & {
    preview?: string;
    availability: "available" | "unavailable";
  }
>;
