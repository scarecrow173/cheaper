import { CheaperError, type ErrorCode } from "@cheaper/core";
import { cloneJson, hasOwn, isObject } from "./json.js";
import type { Binding, FromBinding, JsonSchema, JsonValue } from "./types.js";

export function pointerSegments(pointer: string): string[] {
  if (pointer === "") return [];
  if (!pointer.startsWith("/") || /~(?:[^01]|$)/.test(pointer)) {
    throw new CheaperError("INVALID_REQUEST", `invalid JSON Pointer: ${pointer}`);
  }
  return pointer
    .slice(1)
    .split("/")
    .map((part) => part.replace(/~1/g, "/").replace(/~0/g, "~"));
}

/** reserved key を含む object は必ず exact wrapper として解釈する。 */
export function parseBinding(value: unknown): Binding | undefined {
  if (!isObject(value) || !["$input", "$from", "$literal"].some((key) => hasOwn(value, key)))
    return undefined;
  const keys = Object.keys(value).sort().join(",");
  if (keys === "$input" && typeof value.$input === "string" && value.$input.length > 0)
    return value as Binding;
  if (keys === "$literal") return value as Binding;
  if (
    keys === "$from,path" &&
    typeof value.$from === "string" &&
    value.$from.length > 0 &&
    typeof value.path === "string"
  ) {
    pointerSegments(value.path);
    return value as FromBinding;
  }
  throw new CheaperError("INVALID_REQUEST", "invalid binding wrapper", {
    details: { keys: Object.keys(value) },
  });
}

export function scanBindings(value: JsonValue): { inputs: string[]; from: FromBinding[] } {
  const inputs = new Set<string>();
  const from: FromBinding[] = [];
  function scan(node: JsonValue): void {
    const binding = parseBinding(node);
    if (binding) {
      if ("$input" in binding) inputs.add(binding.$input);
      else if ("$from" in binding) from.push(binding);
      return;
    }
    if (Array.isArray(node)) {
      for (const child of node) scan(child);
    } else if (isObject(node)) {
      for (const child of Object.values(node)) scan(child as JsonValue);
    }
  }
  scan(value);
  return { inputs: [...inputs].sort(), from };
}

export function resolvePointer(
  result: JsonValue,
  pointer: string,
  code: ErrorCode = "INVALID_INPUT",
): JsonValue {
  let value = result;
  for (const segment of pointerSegments(pointer)) {
    if (
      value === null ||
      typeof value !== "object" ||
      !hasOwn(value, segment) ||
      (Array.isArray(value) &&
        (!/^(0|[1-9][0-9]*)$/.test(segment) || Number(segment) >= value.length))
    ) {
      throw new CheaperError(code, `missing value at JSON Pointer: ${pointer}`, {
        details: { pointer, segment },
      });
    }
    value = (value as Record<string, JsonValue>)[segment] as JsonValue;
  }
  return cloneJson(value, code);
}

/** deferFrom は compile 表示専用。hook は元 template に対して全 binding を一度で解決する。 */
export function materializeBindings(
  value: JsonValue,
  inputs: Readonly<Record<string, JsonValue>>,
  results: ReadonlyMap<string, JsonValue> = new Map(),
  options: { deferFrom?: boolean; code?: ErrorCode } = {},
): JsonValue {
  const code = options.code ?? "INVALID_INPUT";
  function materialize(node: JsonValue): JsonValue {
    const binding = parseBinding(node);
    if (binding) {
      if ("$literal" in binding) return cloneJson(binding.$literal, code);
      if ("$input" in binding) {
        if (!hasOwn(inputs, binding.$input))
          throw new CheaperError(code, `missing input: ${binding.$input}`);
        return cloneJson(inputs[binding.$input], code);
      }
      if (options.deferFrom) return cloneJson(binding);
      if (!results.has(binding.$from))
        throw new CheaperError(code, `missing job result: ${binding.$from}`);
      return resolvePointer(results.get(binding.$from) as JsonValue, binding.path, code);
    }
    if (Array.isArray(node)) return node.map(materialize);
    if (isObject(node))
      return Object.fromEntries(
        Object.keys(node)
          .sort()
          .map((key) => [key, materialize(node[key] as JsonValue)]),
      );
    return node;
  }
  return materialize(value);
}

/** 閉じた単純 schema で証明できる不可能性だけ拒否。ref/union/open schema は runtime に残す。 */
export function validatePointerSchema(schema: JsonSchema, pointer: string): void {
  let current: unknown = schema;
  for (const segment of pointerSegments(pointer)) {
    if (current === true) return;
    if (current === false)
      throw new CheaperError("INVALID_REQUEST", `impossible output path: ${pointer}`);
    if (
      !isObject(current) ||
      ["$ref", "$dynamicRef", "anyOf", "oneOf", "allOf", "if", "not"].some((key) =>
        hasOwn(current as object, key),
      )
    )
      return;
    const type = current.type;
    if (typeof type !== "string") return;
    if (type === "object") {
      const properties = current.properties;
      if (isObject(properties) && hasOwn(properties, segment)) current = properties[segment];
      else if (current.additionalProperties === false && !hasOwn(current, "patternProperties"))
        throw new CheaperError("INVALID_REQUEST", `unknown closed output property: ${pointer}`);
      else return;
    } else if (type === "array") {
      if (
        !/^(0|[1-9][0-9]*)$/.test(segment) ||
        (typeof current.maxItems === "number" && Number(segment) >= current.maxItems)
      ) {
        throw new CheaperError("INVALID_REQUEST", `impossible array output path: ${pointer}`);
      }
      if (Array.isArray(current.prefixItems) && Number(segment) < current.prefixItems.length)
        current = current.prefixItems[Number(segment)];
      else if (current.items === false)
        throw new CheaperError("INVALID_REQUEST", `impossible array output path: ${pointer}`);
      else current = current.items;
    } else {
      throw new CheaperError("INVALID_REQUEST", `primitive output has no child: ${pointer}`);
    }
  }
}
