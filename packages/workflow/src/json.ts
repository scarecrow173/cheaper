import { CheaperError, type ErrorCode } from "@cheaper/core";
import type { JsonValue } from "./types.js";

export const hasOwn = (value: object, key: string): boolean => Object.hasOwn(value, key);
export function isObject(value: unknown): value is Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) return false;
  const prototype = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}

/** 外部 JSON 境界と snapshot の唯一の clone。getter/toJSON は実行しない。 */
export function cloneJson(value: unknown, code: ErrorCode = "INVALID_REQUEST"): JsonValue {
  const ancestors = new Set<object>();
  function visit(current: unknown): JsonValue {
    if (current === null || typeof current === "string" || typeof current === "boolean") return current;
    if (typeof current === "number" && Number.isFinite(current)) return current;
    if (typeof current !== "object" || current === null || (!Array.isArray(current) && !isObject(current))) {
      throw new CheaperError(code, "value must be plain JSON");
    }
    if (ancestors.has(current)) throw new CheaperError(code, "cyclic JSON value");
    ancestors.add(current);
    try {
      if (Object.getOwnPropertySymbols(current).length !== 0) throw new CheaperError(code, "symbol JSON key");
      if (Array.isArray(current)) {
        if (Object.keys(current).length !== current.length) throw new CheaperError(code, "sparse or extended JSON array");
        return Array.from({ length: current.length }, (_, index) => {
          const descriptor = Object.getOwnPropertyDescriptor(current, String(index));
          if (!descriptor || !("value" in descriptor)) throw new CheaperError(code, "invalid JSON array entry");
          return visit(descriptor.value);
        });
      }
      return Object.fromEntries(Object.getOwnPropertyNames(current).sort().map((key) => {
        const descriptor = Object.getOwnPropertyDescriptor(current, key);
        if (!descriptor || !("value" in descriptor) || !descriptor.enumerable) throw new CheaperError(code, "invalid JSON property");
        return [key, visit(descriptor.value)];
      }));
    } finally {
      ancestors.delete(current);
    }
  }
  return visit(value);
}
