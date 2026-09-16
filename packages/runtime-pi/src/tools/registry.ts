import { CheaperError, type SymlinkPolicy, type ToolName } from "@cheaper/core";
import {
  createEditToolDefinition,
  createLsToolDefinition,
  createReadToolDefinition,
  createWriteToolDefinition,
  defineTool,
  type ToolDefinition,
} from "@earendil-works/pi-coding-agent";
import type { MutationJournal } from "../journal.js";
import { type ToolBudget, wrapToolBudget } from "./budget.js";
import { createCountTokensTool } from "./count-tokens.js";
import { createFindTool } from "./find.js";
import { createGrepTool } from "./grep.js";
import {
  createEditOperations,
  createLsOperations,
  createReadOperations,
  createWriteOperations,
} from "./operations.js";

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
}

export interface SandboxedTools {
  /** session.customTools に渡す ToolDefinition 列。 */
  tools: ToolDefinition[];
  /** session.tools allowlist に渡す tool 名。 */
  toolNames: string[];
  budget: ToolBudget;
}

/**
 * ToolName → Pi ToolDefinition 生成（DESIGN-0002/0004）。
 * read/write/edit/ls は Pi の createXxxToolDefinition に boundary 付き operations を注入し、
 * grep/find は自前 tool。全 tool を budget wrap する。
 */
export function createSandboxedTools(opts: SandboxedToolsOptions): SandboxedTools {
  const opsOpts = { root: opts.root, symlinkPolicy: opts.symlinkPolicy, journal: opts.journal };
  const walkOpts = { root: opts.root, symlinkPolicy: opts.symlinkPolicy };
  const tools: ToolDefinition[] = opts.names.map((name): ToolDefinition => {
    switch (name) {
      case "read":
        // defineTool は runtime identity。& AnyToolDefinition 交差を得て
        // ToolDefinition[]（default generic）へ代入可能にするための型上の wrap。
        return defineTool(
          createReadToolDefinition(opts.root, {
            operations: createReadOperations(opsOpts),
          }),
        );
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
  const budgeted = wrapToolBudget(tools, {
    maxToolCalls: opts.maxToolCalls,
    onLimit: opts.onLimit,
  });
  return { tools: budgeted.tools, toolNames: tools.map((t) => t.name), budget: budgeted.budget };
}
