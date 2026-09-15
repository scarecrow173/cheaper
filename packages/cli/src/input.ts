import { readFile } from "node:fs/promises";
import { CheaperError } from "@cheaper/core";

export interface StdinLike extends NodeJS.ReadableStream {
  isTTY?: boolean;
}

function invalidRequest(message: string, details?: Record<string, unknown>): CheaperError {
  return new CheaperError("INVALID_REQUEST", message, details === undefined ? {} : { details });
}

async function readAll(stream: NodeJS.ReadableStream): Promise<string> {
  const chunks: Buffer[] = [];
  for await (const chunk of stream) {
    chunks.push(typeof chunk === "string" ? Buffer.from(chunk) : (chunk as Buffer));
  }
  return Buffer.concat(chunks).toString("utf8");
}

/**
 * task input を `--input` file または stdin から読む（DESIGN-0005）。
 * stdin が TTY のときは待たずに空入力扱い → INVALID_REQUEST。
 * 入力は JSON object 必須（array/scalar/null も拒否）。
 */
export async function readTaskInput(
  input: string,
  stdin: StdinLike = process.stdin,
): Promise<Record<string, unknown>> {
  let text: string;
  if (input === "-") {
    if (stdin.isTTY === true) {
      throw invalidRequest("no task input: stdin is a TTY and --input was not given");
    }
    text = await readAll(stdin);
    if (text.trim().length === 0) {
      throw invalidRequest("no task input: stdin was empty");
    }
  } else {
    try {
      text = await readFile(input, "utf8");
    } catch (e) {
      throw invalidRequest(`cannot read input file: ${input}`, {
        input,
        cause: e instanceof Error ? e.message : String(e),
      });
    }
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw invalidRequest("task input is not valid JSON", { reason: "json-parse" });
  }
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    throw invalidRequest("task input must be a JSON object", { reason: "not-object" });
  }
  return parsed as Record<string, unknown>;
}
