import {
  createExtensionRuntime,
  type ExtensionRuntime,
  type LoadExtensionsResult,
  type ResourceLoader,
} from "@earendil-works/pi-coding-agent";

// package index から export されない型は ResourceLoader メソッドの Parameters で参照する
type ResourceExtensionPaths = Parameters<ResourceLoader["extendResources"]>[0];
type ResourceLoaderReloadOptions = Parameters<ResourceLoader["reload"]>[0];

export interface CheaperResourceLoaderOptions {
  /** worker.instructions を system prompt として供給する。 */
  systemPrompt: string;
  /** 追加の system prompt セクション（例: 出力契約の指示）。 */
  appendSystemPrompt?: string[];
}

/**
 * Worker 実行専用の最小 ResourceLoader（DESIGN-0004）。
 * extension / skill / prompt template / theme / AGENTS.md は一切読み込まない。
 * 外部リソースを持たないため reload / extendResources は no-op。
 */
export class CheaperResourceLoader implements ResourceLoader {
  private readonly runtime: ExtensionRuntime = createExtensionRuntime();
  private readonly systemPrompt: string;
  private readonly appendSystemPrompt: string[];

  constructor(options: CheaperResourceLoaderOptions) {
    this.systemPrompt = options.systemPrompt;
    this.appendSystemPrompt = options.appendSystemPrompt ?? [];
  }

  getExtensions(): LoadExtensionsResult {
    return { extensions: [], errors: [], runtime: this.runtime };
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

  getSystemPrompt(): string | undefined {
    return this.systemPrompt;
  }

  getSystemPromptSource(): { path: string } | undefined {
    return undefined;
  }

  getAppendSystemPrompt(): string[] {
    return this.appendSystemPrompt;
  }

  getAppendSystemPromptSources(): Array<{ path: string }> {
    return [];
  }

  extendResources(_paths: ResourceExtensionPaths): void {
    // no-op: Worker 実行では外部リソースを持たない
  }

  async reload(_options?: ResourceLoaderReloadOptions): Promise<void> {
    // no-op
  }
}
