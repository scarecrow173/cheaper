import { parse as parseYaml } from "yaml";
import { CheaperError } from "../errors.js";

export interface ParsedWorkerFile {
  frontmatter: unknown;
  body: string;
}

function invalid(sourcePath: string, reason: string, cause?: unknown): CheaperError {
  const options: { details: Record<string, unknown>; cause?: unknown } = {
    details: { sourcePath, reason },
  };
  if (cause !== undefined) options.cause = cause;
  return new CheaperError(
    "INVALID_WORKER_DEFINITION",
    `invalid worker definition: ${reason}`,
    options,
  );
}

export function parseWorkerFile(content: string, sourcePath: string): ParsedWorkerFile {
  const lines = content.split(/\r?\n/);
  if (lines[0] !== "---") {
    throw invalid(sourcePath, "frontmatter-not-found");
  }
  const close = lines.findIndex((l, i) => i > 0 && l.trim() === "---");
  if (close === -1) {
    throw invalid(sourcePath, "frontmatter-not-closed");
  }
  let frontmatter: unknown;
  try {
    frontmatter = parseYaml(lines.slice(1, close).join("\n"));
  } catch (e) {
    throw invalid(sourcePath, "frontmatter-yaml-error", e);
  }
  if (typeof frontmatter !== "object" || frontmatter === null || Array.isArray(frontmatter)) {
    throw invalid(sourcePath, "frontmatter-not-object");
  }
  const body = lines
    .slice(close + 1)
    .join("\n")
    .trim();
  if (body.length === 0) {
    throw invalid(sourcePath, "body-empty");
  }
  return { frontmatter, body };
}
