import type { Api, Context, Model } from "@earendil-works/pi-ai";
import { afterEach, describe, expect, it, vi } from "vitest";
import { streamDevin } from "../../../src/providers/devin/stream.js";
import { encodeString } from "../../../src/providers/devin/wire.js";

const JWT = "eyJhbGciOiJIUzI1NiJ9.eyJleHAiOjk5OTk5OTk5OTl9.signature";

function toArrayBuffer(buf: Buffer): ArrayBuffer {
  return buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength) as ArrayBuffer;
}

/** socket が応答途中で死に、cancel() も reject する stream。 */
function erroringBody(): ReadableStream<Uint8Array> {
  return new ReadableStream<Uint8Array>({
    start(controller) {
      controller.enqueue(new Uint8Array([0]));
      controller.error(new Error("socket reset"));
    },
    cancel: () => Promise.reject(new Error("cancel failed")),
  });
}

function stubFetch(): void {
  vi.stubGlobal("fetch", async (input: RequestInfo | URL) => {
    const url = String(input);
    if (url.includes("GetUserJwt")) {
      return {
        ok: true,
        status: 200,
        arrayBuffer: () => Promise.resolve(toArrayBuffer(encodeString(1, JWT))),
      } as unknown as Response;
    }
    return {
      ok: true,
      status: 200,
      text: () => Promise.resolve(""),
      body: erroringBody(),
    } as unknown as Response;
  });
}

const MODEL = {
  id: "test-model",
  api: "devin-local",
  provider: "devin",
} as unknown as Model<Api>;

const CONTEXT = {
  messages: [{ role: "user", content: "hi" }],
} as unknown as Context;

describe("streamDevin error handling", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("emits an error event without leaking unhandled rejections on a socket error", async () => {
    stubFetch();
    const unhandled: unknown[] = [];
    const onUnhandled = (reason: unknown) => {
      unhandled.push(reason);
    };
    process.on("unhandledRejection", onUnhandled);
    try {
      const stream = streamDevin(MODEL, CONTEXT, {
        apiKey: "test-api-key",
        env: { DEVIN_API_SERVER_URL: "https://devin.test" },
      });
      const events: string[] = [];
      for await (const event of stream) {
        events.push(event.type);
      }
      // unhandledRejection は task 境界で発火するため待機してから assert する。
      // reader.closed と body.cancel() の rejection が両方 consume されて
      // いれば process には 1 件も届かない。
      await new Promise((resolve) => setTimeout(resolve, 50));
      expect(events.at(-1)).toBe("error");
      expect(unhandled).toEqual([]);
    } finally {
      process.off("unhandledRejection", onUnhandled);
    }
  });
});
