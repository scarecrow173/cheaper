// Envelope byte-parity vectors — emits the TypeScript reference
// outputs (`packages/cli/dist/envelope.js` の `envelopeJson`)を stdout
// に出し、`crates/cheaper-cli/tests/envelope.rs` の literal と diff で
// byte 一致を確認するための generator。
//
// Usage:
//   node crates/cheaper-cli/scripts/ts-envelope-vectors.mjs
//
// 各行 = 1 envelope(JSON.stringify + "\n" の末尾改行を strip せずに
// 連結)。tests/envelope.rs の各 byte vector と順に対応する。
//
// T8 differential harness が TS envelope generator を必要とする場合は
// この script を再利用する。

import { CheaperError } from "../../../packages/core/dist/index.js";
import {
  envelopeJson,
  errorEnvelope,
  utilitySuccessEnvelope,
} from "../../../packages/cli/dist/envelope.js";

// 1. utility success
process.stdout.write(
  envelopeJson(
    utilitySuccessEnvelope({
      result: { path: "a.txt", tokens: 3 },
      durationMs: 12,
    }),
  ),
);

// 2. error: minimal INVALID_REQUEST
process.stdout.write(
  envelopeJson(
    errorEnvelope(new CheaperError("INVALID_REQUEST", "command required: run"), {
      worker: null,
      durationMs: 5,
    }),
  ),
);

// 3. error: worker + details + CheaperError cause
process.stdout.write(
  envelopeJson(
    errorEnvelope(
      new CheaperError("WORKER_NOT_FOUND", "worker not found: bad-name", {
        details: { name: "bad-name" },
        cause: new CheaperError("TIMEOUT", "timed out"),
      }),
      { worker: "bad-name", durationMs: 7 },
    ),
  ),
);

// 4. error: plain Error → INTERNAL_ERROR(70)
process.stdout.write(
  envelopeJson(
    errorEnvelope(new Error("disk broke"), { worker: "w", durationMs: 0 }),
  ),
);

// 5. workflow envelope(ok / error)
process.stdout.write(
  envelopeJson({
    schemaVersion: 1,
    workflow: true,
    ok: true,
    worker: null,
    result: { out: 1 },
    jobs: [{ id: "j1" }],
    meta: { durationMs: 8 },
  }),
);
process.stdout.write(
  envelopeJson({
    schemaVersion: 1,
    workflow: true,
    ok: false,
    worker: null,
    result: {},
    jobs: [],
    meta: { durationMs: 0 },
    error: { code: "TIMEOUT", message: "t", exitCode: 7, retryable: true },
  }),
);
