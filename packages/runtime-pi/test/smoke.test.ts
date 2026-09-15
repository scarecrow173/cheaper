import type {
  Api,
  AssistantMessageEventStream,
  Context,
  Model,
  SimpleStreamOptions,
} from "@earendil-works/pi-ai";
import {
  type AgentSession,
  type AgentSessionEvent,
  createAgentSession,
  createEditTool,
  createEditToolDefinition,
  createExtensionRuntime,
  createLsTool,
  createLsToolDefinition,
  createReadTool,
  createReadToolDefinition,
  createWriteTool,
  createWriteToolDefinition,
  defineTool,
  type EditOperations,
  type LsOperations,
  ModelRuntime,
  type ReadOperations,
  type ResourceLoader,
  SessionManager,
  SettingsManager,
  type ToolDefinition,
  type WriteOperations,
} from "@earendil-works/pi-coding-agent";
import { describe, expect, it } from "vitest";

/**
 * PLAN-0002 step 1 smoke test: Pi SDK 0.85.1 の export・イベント名・interface が
 * probe（tmp/pi-probe）で確認した形と一致することを型・実行レベルで検証する。
 * 1 つでも欠けた場合は実装を止めて報告する（plan Risk Register）。
 */

// StreamFn shape（pi-agent-core と同型をローカル定義: 直接依存ではないため）
type StreamFn = (
  model: Model<Api>,
  context: Context,
  options?: SimpleStreamOptions,
) => AssistantMessageEventStream | Promise<AssistantMessageEventStream>;

// イベント名の型レベル確認: union に無い literal との比較は TS error になる
function eventNameCheck(e: AgentSessionEvent): string[] {
  const hit: string[] = [];
  if (e.type === "turn_end") hit.push(e.type);
  if (e.type === "message_end") hit.push(e.type);
  if (e.type === "tool_execution_start") hit.push(e.type);
  if (e.type === "tool_execution_end") hit.push(e.type);
  if (e.type === "auto_retry_start") hit.push(e.type);
  if (e.type === "agent_end") hit.push(e.type);
  return hit;
}

// ResourceLoader の 11 メソッドを満たす実装が型適合すること
class SmokeResourceLoader implements ResourceLoader {
  getExtensions() {
    return { extensions: [], errors: [], runtime: createExtensionRuntime() };
  }
  getSkills() {
    return { skills: [], diagnostics: [] };
  }
  getPrompts() {
    return { prompts: [], diagnostics: [] };
  }
  getThemes() {
    return { themes: [], diagnostics: [] };
  }
  getAgentsFiles() {
    return { agentsFiles: [] };
  }
  getSystemPrompt() {
    return "smoke";
  }
  getSystemPromptSource() {
    return undefined;
  }
  getAppendSystemPrompt() {
    return [] as string[];
  }
  getAppendSystemPromptSources() {
    return [] as Array<{ path: string }>;
  }
  extendResources(): void {}
  async reload(): Promise<void> {}
}

// operations 注入 signature の型確認
const _opsCheck = {
  read: {
    readFile: async (_p: string) => Buffer.alloc(0),
    access: async (_p: string) => {},
  } satisfies ReadOperations,
  write: {
    writeFile: async (_p: string, _c: string) => {},
    mkdir: async (_d: string) => {},
  } satisfies WriteOperations,
  edit: {
    readFile: async (_p: string) => Buffer.alloc(0),
    writeFile: async (_p: string, _c: string) => {},
    access: async (_p: string) => {},
  } satisfies EditOperations,
  ls: {
    exists: async (_p: string) => true,
    stat: async (_p: string) => ({ isDirectory: () => true }),
    readdir: async (_p: string) => [] as string[],
  } satisfies LsOperations,
};

// session.agent.streamFunction が StreamFn として代入可能であること
function streamFnCheck(session: AgentSession): void {
  const fn: StreamFn = (_m, _c, _o) => {
    throw new Error("not called");
  };
  session.agent.streamFunction = fn;
}

describe("pi sdk smoke", () => {
  it("exports required factories at runtime", () => {
    expect(typeof createAgentSession).toBe("function");
    expect(typeof SessionManager.inMemory).toBe("function");
    expect(typeof SettingsManager.inMemory).toBe("function");
    expect(typeof createReadTool).toBe("function");
    expect(typeof createWriteTool).toBe("function");
    expect(typeof createEditTool).toBe("function");
    expect(typeof createLsTool).toBe("function");
    expect(typeof createReadToolDefinition).toBe("function");
    expect(typeof createWriteToolDefinition).toBe("function");
    expect(typeof createEditToolDefinition).toBe("function");
    expect(typeof createLsToolDefinition).toBe("function");
    expect(typeof defineTool).toBe("function");
    expect(typeof createExtensionRuntime).toBe("function");
    expect(typeof ModelRuntime.create).toBe("function");
  });

  it("ModelRuntime exposes getAvailable/getModel on its prototype", () => {
    const proto = ModelRuntime.prototype as ModelRuntime;
    expect(typeof proto.getAvailable).toBe("function");
    expect(typeof proto.getModel).toBe("function");
  });

  it("event names exist in AgentSessionEvent union", () => {
    // 型レベル検証が主目的。listener がコンパイルを通ること自体が証拠。
    const listener: (e: AgentSessionEvent) => string[] = eventNameCheck;
    expect(typeof listener).toBe("function");
  });

  it("ToolDefinition can be built via defineTool", () => {
    const tool: ToolDefinition = defineTool({
      name: "smoke",
      label: "Smoke",
      description: "smoke tool",
      parameters: { type: "object", properties: {} } as never,
      execute: async () => ({ content: [{ type: "text" as const, text: "ok" }], details: {} }),
    });
    expect(tool.name).toBe("smoke");
  });

  it("streamFunction assignment compiles", () => {
    expect(typeof streamFnCheck).toBe("function");
  });

  it("ResourceLoader 11-method implementation type-checks and returns empty resources", async () => {
    const loader: ResourceLoader = new SmokeResourceLoader();
    expect(loader.getExtensions().extensions).toHaveLength(0);
    expect(loader.getSkills().skills).toHaveLength(0);
    expect(loader.getPrompts().prompts).toHaveLength(0);
    expect(loader.getThemes().themes).toHaveLength(0);
    expect(loader.getAgentsFiles().agentsFiles).toHaveLength(0);
    expect(loader.getSystemPrompt()).toBe("smoke");
    expect(loader.getSystemPromptSource()).toBeUndefined();
    expect(loader.getAppendSystemPrompt()).toHaveLength(0);
    expect(loader.getAppendSystemPromptSources()).toHaveLength(0);
    loader.extendResources({});
    await loader.reload();
  });
});
