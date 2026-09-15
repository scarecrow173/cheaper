import type {
  Api,
  AssistantMessageEventStream,
  Context,
  Model,
  SimpleStreamOptions,
} from "@earendil-works/pi-ai";
import {
  type AgentSession,
  createAgentSession,
  type ModelRuntime,
  SessionManager,
  SettingsManager,
  type ToolDefinition,
} from "@earendil-works/pi-coding-agent";
import { CheaperResourceLoader } from "./resource-loader.js";

/** pi-agent-core の StreamFn と構造上同型（依存追加を避けるためローカル定義）。 */
export type StreamFnLike = (
  model: Model<Api>,
  context: Context,
  options?: SimpleStreamOptions,
) => AssistantMessageEventStream | Promise<AssistantMessageEventStream>;

export interface WorkerSessionOptions {
  /** canonical 化済み workspace root。session cwd になる。 */
  cwd: string;
  model: Model<Api>;
  modelRuntime?: ModelRuntime;
  /** Worker 本文 + Output contract を連結済みの system prompt。 */
  systemPrompt: string;
  appendSystemPrompt?: string[];
  /** budget wrap 済みの sandboxed ToolDefinition 列。repair session では省略。 */
  customTools?: ToolDefinition[];
  /** tools allowlist（custom tool 名を含む）。repair session では省略。 */
  toolNames?: string[];
  /** builtin tool 抑制モード。repair session は "all"（全 tool 無効）。 */
  noTools?: "all" | "builtin";
  /** policy.output.maxTokens（model.maxTokens と min を取る）。 */
  maxOutputTokens: number;
  /** Pi auto-retry 回数。0 で retry 無効。 */
  transientRetries: number;
  /** テスト用 stream seam。指定時は session.agent.streamFunction の差し替え元になる。 */
  streamFn?: StreamFnLike;
}

export interface WorkerSessionHandle {
  session: AgentSession;
  /** 実際に注入された effective maxTokens。 */
  maxTokens: number;
}

/**
 * Worker 実行用の AgentSession を構築する（DESIGN-0004）。
 * - cwd=workspace.real, thinkingLevel=off, noTools="builtin" + tools allowlist
 * - customTools = sandboxed tools、resourceLoader = CheaperResourceLoader
 * - SessionManager/SettingsManager は inMemory、compaction 無効、retry は transientRetries に従う
 * - session.agent.streamFunction を wrap し options.maxTokens に
 *   `min(maxOutputTokens, model.maxTokens)` を注入する（compaction 無効のため
 *   `=== streamSimple` 参照チェックの影響は無い）
 */
export async function createWorkerSession(
  opts: WorkerSessionOptions,
): Promise<WorkerSessionHandle> {
  const { session } = await createAgentSession({
    cwd: opts.cwd,
    ...(opts.modelRuntime ? { modelRuntime: opts.modelRuntime } : {}),
    model: opts.model,
    thinkingLevel: "off",
    noTools: opts.noTools ?? "builtin",
    ...(opts.toolNames ? { tools: opts.toolNames } : {}),
    ...(opts.customTools ? { customTools: opts.customTools } : {}),
    resourceLoader: new CheaperResourceLoader({
      systemPrompt: opts.systemPrompt,
      ...(opts.appendSystemPrompt ? { appendSystemPrompt: opts.appendSystemPrompt } : {}),
    }),
    sessionManager: SessionManager.inMemory(),
    settingsManager: SettingsManager.inMemory({
      compaction: { enabled: false },
      retry: {
        enabled: opts.transientRetries > 0,
        maxRetries: opts.transientRetries,
      },
    }),
  });

  const maxTokens = Math.min(opts.maxOutputTokens, opts.model.maxTokens);
  const base: StreamFnLike = opts.streamFn ?? session.agent.streamFunction;
  session.agent.streamFunction = (model, context, options) =>
    base(model, context, { ...options, maxTokens });

  return { session, maxTokens };
}
