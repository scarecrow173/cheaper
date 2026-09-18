import { defineConfig } from "vitest/config";

// e2e は test ごとに bundled CLI / hook script を子プロセスで spawn
// する。default の 5s testTimeout は full-suite 並行負荷下で境界的
// なため、process-spawn 系の実行時間に余裕を持たせる。
export default defineConfig({
  test: {
    testTimeout: 20_000,
  },
});
