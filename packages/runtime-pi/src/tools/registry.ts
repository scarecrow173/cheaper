import { CheaperError, type PathDenylist, type SymlinkPolicy, type ToolName } from "@cheaper/core";
import {
  createEditToolDefinition,
  createLsToolDefinition,
  createWriteToolDefinition,
  defineTool,
  type ToolDefinition,
} from "@earendil-works/pi-coding-agent";
import type { MutationJournal } from "../journal.js";
import { type ToolBudget, wrapToolBudget } from "./budget.js";
import { createCountTokensTool } from "./count-tokens.js";
import { createFindTool } from "./find.js";
import { createGrepTool } from "./grep.js";
import { type ToolMeter, wrapToolMeter } from "./meter.js";
import { createEditOperations, createLsOperations, createWriteOperations } from "./operations.js";
import { createReadTool } from "./read.js";

export interface SandboxedToolsOptions {
  /** canonical 化済み workspace root。 */
  root: string;
  symlinkPolicy: SymlinkPolicy;
  journal: MutationJournal;
  /** effective tool 名（Worker tools ∩ policy tools.allow）。 */
  names: ToolName[];
  maxToolCalls: number;
  /** maxToolCalls 到達時フック（run 側で session.abort() を起動）。 */
  onLimit: () => void;
  /** privacy deny list（DESIGN-0015）。全 tool の path access に適用。 */
  denylist?: PathDenylist | undefined;
  /** write scope（正規化済み display path 集合）。設定時 write/edit/mkdir を制限。 */
  writeScope?: ReadonlySet<string> | undefined;
}

export interface SandboxedTools {
  /** session.customTools に渡す ToolDefinition 列。 */
  tools: ToolDefinition[];
  /** session.tools allowlist に渡す tool 名。 */
  toolNames: string[];
  budget: ToolBudget;
  /** tool output の token 集計（meta.savings.sourceTokens の供給元）。 */
  meter: ToolMeter;
}

/**
 * ToolName → Pi ToolDefinition 生成（DESIGN-0002/0004/0019）。
 * write/edit/ls は Pi の createXxxToolDefinition に boundary 付き operations を注入し、
 * read/grep/find は自前 tool。全 tool を budget wrap する。
 */
export function createSandboxedTools(opts: SandboxedToolsOptions): SandboxedTools {
  const opsOpts = {
    root: opts.root,
    symlinkPolicy: opts.symlinkPolicy,
    journal: opts.journal,
    denylist: opts.denylist,
    writeScope: opts.writeScope,
  };
  const walkOpts = {
    root: opts.root,
    symlinkPolicy: opts.symlinkPolicy,
    denylist: opts.denylist,
  };
  const tools: ToolDefinition[] = opts.names.map((name): ToolDefinition => {
    switch (name) {
      case "read":
        return createReadTool(opsOpts);
      case "write":
        return defineTool(
          createWriteToolDefinition(opts.root, {
            operations: createWriteOperations(opsOpts),
          }),
        );
      case "edit":
        return defineTool(
          createEditToolDefinition(opts.root, {
            operations: createEditOperations(opsOpts),
          }),
        );
      case "ls":
        return defineTool(
          createLsToolDefinition(opts.root, {
            operations: createLsOperations(opsOpts),
          }),
        );
      case "grep":
        return createGrepTool(walkOpts);
      case "find":
        return createFindTool(walkOpts);
      case "count_tokens":
        return createCountTokensTool(walkOpts);
      default: {
        // ToolName union 外は到達不能（capability check で既に UNKNOWN_TOOL 済み）
        const never: never = name;
        throw new CheaperError("UNKNOWN_TOOL", `unknown tool: ${String(never)}`, {
          details: { tool: name },
        });
      }
    }
  });
  // meter は budget 内側: budget を通過した call の成功 output のみ計測する。
  const metered = wrapToolMeter(tools);
  const budgeted = wrapToolBudget(metered.tools, {
    maxToolCalls: opts.maxToolCalls,
    onLimit: opts.onLimit,
  });
  return {
    tools: budgeted.tools,
    toolNames: tools.map((t) => t.name),
    budget: budgeted.budget,
    meter: metered.meter,
  };
}
