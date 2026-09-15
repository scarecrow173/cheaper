/**
 * テスト専用サポート（`@cheaper/runtime-pi/testsupport` subpath export）。
 * 本番コードパスからは参照しない。CLI e2e の CHEAPER_TEST_FAKE_STREAM
 * フックと runtime-pi 内部テストが共有する scripted fake stream seam。
 */
import {
  type AssistantMessageEventStream,
  type Context,
  createFauxCore,
  createProvider,
  type FauxModelDefinition,
  type FauxProviderState,
  type FauxResponseStep,
  type Model,
  type Provider,
  type SimpleStreamOptions,
} from "@earendil-works/pi-ai";

export type { FauxResponseStep } from "@earendil-works/pi-ai";
export {
  fauxAssistantMessage,
  fauxText,
  fauxThinking,
  fauxToolCall,
} from "@earendil-works/pi-ai";

export interface FakeStreamCall {
  model: Model<string>;
  context: Context;
  options: SimpleStreamOptions | undefined;
}

export interface FakePi {
  /** createAgentSession の model オプションに渡す fake Model。 */
  model: Model<string>;
  /**
   * ModelRuntime.registerNativeProvider() に登録する fake Provider。
   * auth は常に解決する keyless 実装のため prompt() の auth 検証を通過する。
   */
  provider: Provider;
  /**
   * session.agent.streamFunction に差し替える scripted StreamFn。
   * pi-ai faux provider を薄く wrap し、呼び出し引数を calls に記録する。
   */
  streamFn: (
    model: Model<string>,
    context: Context,
    options?: SimpleStreamOptions,
  ) => AssistantMessageEventStream;
  /** これまでの stream 呼び出し（context/options を検証するための記録）。 */
  calls: FakeStreamCall[];
  /** 応答 queue を置き換える。 */
  setResponses: (steps: FauxResponseStep[]) => void;
  /** 応答 queue に追加する。 */
  appendResponses: (steps: FauxResponseStep[]) => void;
  /** queue 残数（想定外の呼び出しが無いことの検証用）。 */
  getPendingResponseCount: () => number;
  state: FauxProviderState;
}

export interface CreateFakePiOptions {
  models?: FauxModelDefinition[];
  responses?: FauxResponseStep[];
}

/**
 * 実 provider を呼ばない session/run テスト用の fake stream seam（TASK-0031）。
 * 内部は pi-ai 同梱の faux provider。応答が尽きると error event を emit するため、
 * 想定外の余分な stream 呼び出しも検出できる。
 */
export function createFakePi(options: CreateFakePiOptions = {}): FakePi {
  const core = createFauxCore({
    provider: "cheaper-fake",
    api: "cheaper-fake-api",
    models: options.models ?? [{ id: "fake-model", name: "Fake Model" }],
  });
  // pi-ai の fauxProvider() と同じ構成で Provider を作り、ModelRuntime に登録可能にする。
  // auth.resolve が常に成功する keyless 実装なので prompt() の auth 検証を通過する。
  const provider = createProvider({
    id: core.provider,
    auth: { apiKey: { name: "Faux", resolve: async () => ({ auth: {} }) } },
    models: core.models,
    api: {
      stream: core.stream,
      streamSimple: core.streamSimple,
      fetchDeferred: core.fetchDeferred,
      cancelDeferred: core.cancelDeferred,
    },
  });
  if (options.responses) core.setResponses(options.responses);
  const calls: FakeStreamCall[] = [];
  return {
    model: core.getModel(),
    provider,
    streamFn(model, context, opts) {
      calls.push({ model, context, options: opts });
      return core.stream(model, context, opts);
    },
    calls,
    setResponses: core.setResponses,
    appendResponses: core.appendResponses,
    getPendingResponseCount: core.getPendingResponseCount,
    state: core.state,
  };
}
