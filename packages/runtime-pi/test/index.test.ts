import { describe, expect, it } from "vitest";
import * as api from "../src/index.js";

describe("public API", () => {
  it("exports runWorker and the model adapter", () => {
    expect(typeof api.runWorker).toBe("function");
    expect(typeof api.resolvePiModel).toBe("function");
    expect(typeof api.toModelInfo).toBe("function");
    expect(typeof api.MutationJournal).toBe("function");
    expect(typeof api.UsageCollector).toBe("function");
  });
});
