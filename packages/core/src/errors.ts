export const ERROR_TABLE = {
  INVALID_REQUEST: { exitCode: 2, retryable: false },
  INVALID_INPUT: { exitCode: 2, retryable: false },
  FILE_TOO_LARGE: { exitCode: 2, retryable: false },
  WORKER_NOT_FOUND: { exitCode: 3, retryable: false },
  INVALID_WORKER_DEFINITION: { exitCode: 3, retryable: false },
  INVALID_WORKER_NAME: { exitCode: 3, retryable: false },
  WORKER_NAME_MISMATCH: { exitCode: 3, retryable: false },
  UNKNOWN_TOOL: { exitCode: 3, retryable: false },
  INVALID_RUNTIME_POLICY: { exitCode: 3, retryable: false },
  OUTPUT_SCHEMA_NOT_FOUND: { exitCode: 3, retryable: false },
  INVALID_OUTPUT_SCHEMA: { exitCode: 3, retryable: false },
  INPUT_SCHEMA_NOT_FOUND: { exitCode: 3, retryable: false },
  INVALID_INPUT_SCHEMA: { exitCode: 3, retryable: false },
  INVALID_MODEL_CONFIG: { exitCode: 3, retryable: false },
  POLICY_VIOLATION: { exitCode: 4, retryable: false },
  MODEL_TIER_UNAVAILABLE: { exitCode: 5, retryable: true },
  PROVIDER_FAILURE: { exitCode: 5, retryable: true },
  TOOL_EXECUTION_FAILURE: { exitCode: 6, retryable: false },
  INVALID_OUTPUT: { exitCode: 6, retryable: false },
  MAX_TURNS_EXCEEDED: { exitCode: 6, retryable: false },
  MAX_TOOL_CALLS_EXCEEDED: { exitCode: 6, retryable: false },
  TIMEOUT: { exitCode: 7, retryable: true },
  ABORTED: { exitCode: 8, retryable: false },
  PARTIAL_MUTATION: { exitCode: 9, retryable: false },
  INTERNAL_ERROR: { exitCode: 70, retryable: false },
} as const satisfies Record<string, { exitCode: number; retryable: boolean }>;

export type ErrorCode = keyof typeof ERROR_TABLE;

export class CheaperError extends Error {
  readonly code: ErrorCode;
  readonly exitCode: number;
  readonly retryable: boolean;
  readonly details?: Record<string, unknown>;
  override readonly cause?: unknown;

  constructor(
    code: ErrorCode,
    message: string,
    options?: { details?: Record<string, unknown>; cause?: unknown },
  ) {
    super(message);
    this.name = "CheaperError";
    this.code = code;
    const entry = ERROR_TABLE[code];
    this.exitCode = entry.exitCode;
    this.retryable = entry.retryable;
    if (options?.details !== undefined) this.details = options.details;
    if (options?.cause !== undefined) this.cause = options.cause;
  }
}

export function isCheaperError(e: unknown): e is CheaperError {
  return e instanceof CheaperError;
}
