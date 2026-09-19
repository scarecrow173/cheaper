import { defineConfig } from "vitest/config";

// e2e は test ごとに bundled CLI / hook script を子プロセスで spawn
// する。default の 5s testTimeout は full-suite 並行負荷下で境界的
// なため、process-spawn 系の実行時間に余裕を持たせる。beforeEach の
// apm availability 確認や package copy も同じ負荷を受けるため
// hookTimeout も引き上げる。
export default defineConfig({
  test: {
    testTimeout: 20_000,
    hookTimeout: 30_000,
  },
});
