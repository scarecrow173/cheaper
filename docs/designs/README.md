# DESIGN Documents

<!-- doc-suite:generated-index -->

Directory: `docs/designs`

| ID | Title | Status | File |
| --- | --- | --- | --- |
| DESIGN-OVERVIEW | System Design Overview | draft | [overview.md](./overview.md) |
| DESIGN-034EehnT1xOPAee9dK22Kx | Setup-boundary abort classification via core ErrorCode reverse lookup | approved | [abort-classification-at-setup-boundary.md](./abort-classification-at-setup-boundary.md) |
| DESIGN-034z1EgnQwax4VamK2xmdT | APM executable payload layout and platform launchers | superseded | [apm-executable-payload-layout-and-platform-launchers.md](./apm-executable-payload-layout-and-platform-launchers.md) |
| DESIGN-kXMpXYHxw0mt0AE0SeAHTM | APM hook 宣言の per-target サブパッケージ分離 設計 | approved | [apm-hook-per-target-package-split.md](./apm-hook-per-target-package-split.md) |
| DESIGN-034r05iHy2gdnoFHbdllXV | APM skills and pre_read_code delegation hook | approved | [apm-skills-and-pre-read-hook.md](./apm-skills-and-pre-read-hook.md) |
| DESIGN-034r05iHy2gdnoJ5j3v9Nk | cheaper auth command design (provider OAuth absorbed by the CLI) | approved | [auth-command.md](./auth-command.md) |
| DESIGN-Hz0wDP9E6GjkHNLwOEOGiw | Auth terminal prompt I/O boundary (injected stderr for text/select prompts) | approved | [auth-io-boundary.md](./auth-io-boundary.md) |
| DESIGN-034r05iHy2gdnoOvi1vrou | Auth URL opener seam (injectable browser launch) | approved | [auth-url-opener-seam.md](./auth-url-opener-seam.md) |
| DESIGN-C4VbdOatUn4aVt3blVe8pk | auto tier classification by workload effectiveCost percentile | approved | [auto-tier-classification.md](./auto-tier-classification.md) |
| DESIGN-034H3c448Lvz0CfBjd5Xlz | Bounded hook request evaluation and hook-config registration lifecycle | approved | [bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md](./bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md) |
| DESIGN-034zWclUCjCIH7M27XEu4j | ToolScheduler の共有 admission 設計 | approved | [bounded-tool-scheduler.md](./bounded-tool-scheduler.md) |
| DESIGN-7G7UEmtrTGU4vh6og30Q0g | builtin materialize fingerprint 設計 | approved | [builtin-materialize-content-fingerprint.md](./builtin-materialize-content-fingerprint.md) |
| DESIGN-034r05iHrZUE6WxkSJi9GF | bulk-reader contract | approved | [bulk-reader-contract.md](./bulk-reader-contract.md) |
| DESIGN-034r05iHy2gdnoNaWH2bos | Bulk-reader huge-file safe I/O | approved | [bulk-reader-huge-file-safe-io.md](./bulk-reader-huge-file-safe-io.md) |
| DESIGN-giTr4G7NzwSzWJTVgBo3ax | bulk-reader 出力上限・INVALID_OUTPUT 指針の skill 文書設計 | approved | [bulk-reader-output-limits-and-invalid-output-skill-docs.md](./bulk-reader-output-limits-and-invalid-output-skill-docs.md) |
| DESIGN-034zIUgKgoIuBD5BY6jLjf | Canonical Tool Runtime と Pi adapter | approved | [canonical-tool-runtime.md](./canonical-tool-runtime.md) |
| DESIGN-034Dmk2RqH0fMzF3gLrR0T | cargo source install distribution and thin PATH launchers | approved | [cargo-source-install-distribution-and-thin-path-launchers.md](./cargo-source-install-distribution-and-thin-path-launchers.md) |
| DESIGN-034r05iHrZUE6Wy5X0Hc88 | cheap-writer contract | superseded | [cheap-writer-contract.md](./cheap-writer-contract.md) |
| DESIGN-034z1EhuztlLSb2x2XXxgD | cheaper hook subcommands replacing Node-based hook adapters | approved | [cheaper-hook-subcommands-replacing-node-based-hook-adapters.md](./cheaper-hook-subcommands-replacing-node-based-hook-adapters.md) |
| DESIGN-034DkkcOv0Sos8KzcEP3oB | cheaper-usage SKILL.md hook path section rewrite | approved | [cheaper-usage-skill-hook-path-section-rewrite.md](./cheaper-usage-skill-hook-path-section-rewrite.md) |
| DESIGN-034r05iHy2gdnoH3XDJcXx | cheaper CLI bundle layout inside .apm/hooks/cli | superseded | [cli-bundle-layout.md](./cli-bundle-layout.md) |
| DESIGN-034r05iHrZUE6WvDuYjZB4 | CLI contract・result/error envelope・exit code | approved | [cli-contract-envelope-exit-codes.md](./cli-contract-envelope-exit-codes.md) |
| DESIGN-034KsaxkFcfUHzvbR5hEB1 | Codex/Cursor/Copilot hook rewrite enablement | approved | [codex-cursor-copilot-hook-rewrite-enablement.md](./codex-cursor-copilot-hook-rewrite-enablement.md) |
| DESIGN-034r05iHy2gdnoUHGunDtE | Copilot pre-read hook adapter | approved | [copilot-pre-read-hook-adapter.md](./copilot-pre-read-hook-adapter.md) |
| DESIGN-034zJ3dYi8Rs6I6PIyc3Vu | Core resource manifest normalization design | approved | [core-resource-manifest-normalization.md](./core-resource-manifest-normalization.md) |
| DESIGN-034r05iHy2gdnoLTC43RP5 | Cross-agent distribution verification | approved | [cross-agent-distribution-verification.md](./cross-agent-distribution-verification.md) |
| DESIGN-034rA79Uog7JOa7evw6rOL | Cross-agent native routing adapter architecture | approved | [cross-agent-native-routing-adapter-architecture.md](./cross-agent-native-routing-adapter-architecture.md) |
| DESIGN-034r05iHy2gdnoXu6h71XL | Delegation usage measurement log (usage.jsonl) | approved | [delegation-usage-measurement-log.md](./delegation-usage-measurement-log.md) |
| DESIGN-034Kp4aEcwrsOReUSPyxU9 | Deterministic route tests for ubuntu CI flake (issue #157) | approved | [deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md](./deterministic-route-tests-for-ubuntu-ci-flake-issue-157.md) |
| DESIGN-034r05iHy2gdnoPoUu7BiO | Development APM project / distributable cheaper package layout | approved | [dev-apm-project-and-distributable-package-layout.md](./dev-apm-project-and-distributable-package-layout.md) |
| DESIGN-034r05iHy2gdnp1MdNwuw6 | Devin Local provider plugin | approved | [devin-local-provider-plugin.md](./devin-local-provider-plugin.md) |
| DESIGN-034Gt74p3BYG40NTwu2kST | Devin native hooks and APM bootstrap | approved | [devin-native-hooks-and-apm-bootstrap.md](./devin-native-hooks-and-apm-bootstrap.md) |
| DESIGN-034DlAaOWn3RyI3onq5uj1 | devin usage accounting contract test and docs | approved | [devin-usage-accounting-contract-test-and-docs.md](./devin-usage-accounting-contract-test-and-docs.md) |
| DESIGN-034KIsSCvVMAxGDZY1dQTd | exec-contract cleanup — 内容審査を mandatory 層から除去 | approved | [exec-contract-content-audit-removal.md](./exec-contract-content-audit-removal.md) |
| DESIGN-034r05iHy2gdnoS2PWvNo0 | grep batchMaxBytes invariant enforcement | approved | [grep-batch-max-bytes-invariant-enforcement.md](./grep-batch-max-bytes-invariant-enforcement.md) |
| DESIGN-034r6oM1pW6bz2SSvR3rWT | grep eval-budget test stabilization via explicit budget injection | approved | [grep-eval-budget-test-stability.md](./grep-eval-budget-test-stability.md) |
| DESIGN-034ILuADFr70oZoUROR13q | hook-config legacy conflict gate demotion to warning | approved | [hook-config-legacy-conflict-gate-demotion-to-warning.md](./hook-config-legacy-conflict-gate-demotion-to-warning.md) |
| DESIGN-034IefWW6OJ9rvESaNvKUK | Hook eval dedup verdict cache and leader election | approved | [hook-eval-dedup-verdict-cache-and-leader-election.md](./hook-eval-dedup-verdict-cache-and-leader-election.md) |
| DESIGN-034r05iHy2gdnoLt52txFG | Huge-file byte cap guard and fail-closed hook | approved | [huge-file-byte-cap-guard-and-fail-closed-hook.md](./huge-file-byte-cap-guard-and-fail-closed-hook.md) |
| DESIGN-034FntkIZGiffhdU4caSkN | invocation-scoped command grant（--grant）・grantRequests・cheaper policy grant の設計 | approved | [invocation-scoped-command-grants.md](./invocation-scoped-command-grants.md) |
| DESIGN-034A8ubkLAdLl7ICQZRYM5 | managed process canonical executor の設計 | approved | [managed-process-canonical-tool.md](./managed-process-canonical-tool.md) |
| DESIGN-034Au1ncZwENCfsEQyf0cD | managed process lifecycle hardening | approved | [managed-process-lifecycle-hardening.md](./managed-process-lifecycle-hardening.md) |
| DESIGN-034rzP2XX0QSo5LRQDfr9B | model pricing catalog and curated exceptions | approved | [model-pricing-catalog-and-curated-exceptions.md](./model-pricing-catalog-and-curated-exceptions.md) |
| DESIGN-034r05iHrZUE6Wu1cMdeJ1 | Model Resolver | approved | [model-resolver.md](./model-resolver.md) |
| DESIGN-034DV9ypzmxdAobGSJ6Cm6 | Observability foundation logging and instrumentation wiring | approved | [observability-foundation-logging-and-instrumentation-wiring.md](./observability-foundation-logging-and-instrumentation-wiring.md) |
| DESIGN-034r05iHrZUE6WBzjS90b8 | Ollama real-provider smoke verification setup | approved | [ollama-real-provider-smoke-verification-setup.md](./ollama-real-provider-smoke-verification-setup.md) |
| DESIGN-034r05iHy2gdnoWvkDigEM | Real-provider worker E2E via Ollama | approved | [ollama-real-provider-worker-e2e.md](./ollama-real-provider-worker-e2e.md) |
| DESIGN-034zTjGVOLExU7QfmMVo3c | One-dispatch release workflow | superseded | [one-dispatch-release-workflow.md](./one-dispatch-release-workflow.md) |
| DESIGN-034AE2m77s53ISTYWjz28a | One-shot workflow execution and CLI composition | approved | [one-shot-workflow-execution-and-cli-composition.md](./one-shot-workflow-execution-and-cli-composition.md) |
| DESIGN-034CXyXncBigEivCETJLBD | OpenAI Codex subscription provider design | approved | [openai-codex-subscription-provider.md](./openai-codex-subscription-provider.md) |
| DESIGN-034A8ubRdZsG57LdsNNTEw | Operation execution metrics | approved | [operation-execution-metrics.md](./operation-execution-metrics.md) |
| DESIGN-034zSP4AJ6aRzdtIxTHW1Y | OperationManager の state と bounded drain | approved | [operation-manager-lifecycle.md](./operation-manager-lifecycle.md) |
| DESIGN-034KLwE8GiJQ2zvQY20ob8 | opt-in security.execDenyPatterns profile — A9 の降格実装 | approved | [opt-in-exec-deny-patterns.md](./opt-in-exec-deny-patterns.md) |
| DESIGN-034zWr0ZeceJCJaL3ZRK3H | Payload untracking migration | superseded | [payload-untracking-migration.md](./payload-untracking-migration.md) |
| DESIGN-034KuDkWOmNXiBddEQ4XWK | 権限・Runtime Policy・Tool 実行制限の責務ベース再設計 | approved | [permission-model-responsibility-redesign.md](./permission-model-responsibility-redesign.md) |
| DESIGN-034r05iHrZUE6WvaQsRRsM | Pi 実行エンジン: session・制限・abort・retry・出力抽出・repair・usage | approved | [pi-execution-engine.md](./pi-execution-engine.md) |
| DESIGN-034zAKcWuB8SuFkOePsubg | Pi sibling tool-call parallelism inside Workers | approved | [pi-sibling-tool-call-parallelism-inside-workers.md](./pi-sibling-tool-call-parallelism-inside-workers.md) |
| DESIGN-034r05iHy2gdnoZAxhwPFa | Pi state paths under userConfigDir/pi | approved | [pi-state-paths-under-userconfigdir-pi.md](./pi-state-paths-under-userconfigdir-pi.md) |
| DESIGN-034CM9o41G2FjaP5cV125c | Plugin catalog snapshot persistence and user models merge | approved | [plugin-catalog-snapshot-persistence-and-user-models-merge.md](./plugin-catalog-snapshot-persistence-and-user-models-merge.md) |
| DESIGN-034EMWNJbQpMB0N3ZTXM5A | policy-gated command tool の grant 境界と managed-subprocess 共有基盤の設計 | approved | [policy-gated-command-tool.md](./policy-gated-command-tool.md) |
| DESIGN-034EDkfp76WJ5Sn8aKMnUv | Post-feature structural refactoring | approved | [post-feature-structural-refactoring.md](./post-feature-structural-refactoring.md) |
| DESIGN-034DyR22wfkjn91McXS5jv | Post-migration structural cleanup reorganization | approved | [post-migration-structural-cleanup-reorganization.md](./post-migration-structural-cleanup-reorganization.md) |
| DESIGN-034JbqoCWm85pKC033fgfd | PreToolUse route resolver and delegated workflow execution | approved | [pretooluse-route-resolver-and-delegated-workflow-execution.md](./pretooluse-route-resolver-and-delegated-workflow-execution.md) |
| DESIGN-034r2qodyo7KRfBsGRJQty | provider/auth guide runtime sync design | approved | [provider-auth-guide-runtime-sync.md](./provider-auth-guide-runtime-sync.md) |
| DESIGN-034r05iHy2gdnp0zOYnFNU | Provider-only registration layer | approved | [provider-only-registration-layer.md](./provider-only-registration-layer.md) |
| DESIGN-034r05iHy2gdnoCKJhKNGD | Repair abort and failure normalization hardening | approved | [repair-abort-and-failure-normalization-hardening.md](./repair-abort-and-failure-normalization-hardening.md) |
| DESIGN-034r05iHy2gdnoRcfSVFq3 | Repository-owned setup/check entrypoints via mise TOML tasks | approved | [repo-owned-setup-check-entrypoints.md](./repo-owned-setup-check-entrypoints.md) |
| DESIGN-034r05iHrZUE6Wzyjd1NS8 | リポジトリ構成とツールチェーン | approved | [repository-structure-toolchain.md](./repository-structure-toolchain.md) |
| DESIGN-034A8FyPKpnV3KM2HypFGu | ResourceManager job lease と atomic operation admission 設計 | approved | [resource-manager-job-leases.md](./resource-manager-job-leases.md) |
| DESIGN-034IPoKswcH3LaUZdnb0XT | Restrict hook-config orphan sweep to cheaper-owned temp files | approved | [restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md](./restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md) |
| DESIGN-034KqI29dOTRXERRPTp5gV | usage.jsonl run error record の診断 field 拡充 (issue #141) | approved | [run-error-record-observability.md](./run-error-record-observability.md) |
| DESIGN-034zSR05NE5hXXhkTRLMgW | Run-scoped artifact store | approved | [run-scoped-artifact-store.md](./run-scoped-artifact-store.md) |
| DESIGN-034Bbq1qY8WFBdL26tUdBw | Rust + Rig runtime migration architecture | approved | [rust-rig-runtime-migration-architecture.md](./rust-rig-runtime-migration-architecture.md) |
| DESIGN-034z1EfqYaEBfyNvqZyFSD | SEA executable build pipeline and embedded resource resolution | approved | [sea-executable-build-pipeline-and-embedded-resource-resolution.md](./sea-executable-build-pipeline-and-embedded-resource-resolution.md) |
| DESIGN-034rBoPPFNHXR5eSfv0hsO | Shell read-candidate extraction and shared large-read verdict | approved | [shell-read-candidate-extraction.md](./shell-read-candidate-extraction.md) |
| DESIGN-0005kq1c3Wo7rdnwZR8Sp3 | Synchronize distributed cheaper-usage skill and parent-agent integration guide | approved | [synchronize-distributed-skill-and-parent-agent-guide.md](./synchronize-distributed-skill-and-parent-agent-guide.md) |
| DESIGN-034r05iHy2gdnoEfk3jERu | Token counter contract (core module, count_tokens tool, count-tokens CLI) | approved | [token-counter-contract.md](./token-counter-contract.md) |
| DESIGN-034ETcI8q4hTRH502qnQZc | tool-operator built-in worker と cheaper.optionalTools opt-in 縮退機構の設計 | approved | [tool-operator-built-in-worker.md](./tool-operator-built-in-worker.md) |
| DESIGN-034D8wP2Y8qWiuviJbQINn | Trailing-separator guard for workspace boundary predicate | approved | [trailing-separator-guard-for-workspace-boundary-predicate.md](./trailing-separator-guard-for-workspace-boundary-predicate.md) |
| DESIGN-034KP4lfgfD7DdGiDV1YuT | unified exec grant model — command.grants / process.tasks の統合 | draft | [unified-exec-grant-model.md](./unified-exec-grant-model.md) |
| DESIGN-034r05iHy2gdnoT4LG4KTu | Unified version source of truth | approved | [unified-version-source-of-truth.md](./unified-version-source-of-truth.md) |
| DESIGN-034r2lHf6FAFVelj9LrhZh | Align usage.jsonl recording-scope docs with implemented contract | approved | [usage-log-recording-scope-docs.md](./usage-log-recording-scope-docs.md) |
| DESIGN-034rzNZ2BZGTPuOwHmyqh4 | Usage savings telemetry | approved | [usage-savings-telemetry.md](./usage-savings-telemetry.md) |
| DESIGN-034r05iHy2gdnoY3GBf8iz | User-facing documentation information architecture | approved | [user-facing-docs-information-architecture.md](./user-facing-docs-information-architecture.md) |
| DESIGN-034zQkHD65kAeG9xVGSoFX | Version-pinned launcher acquisition | superseded | [version-pinned-launcher-acquisition.md](./version-pinned-launcher-acquisition.md) |
| DESIGN-034D9Xs2f06fbVcxdT08le | Windows credential snapshot sync for Devin provider | approved | [windows-credential-snapshot-sync-for-devin-provider.md](./windows-credential-snapshot-sync-for-devin-provider.md) |
| DESIGN-034KwO2y86CrXOmaoae1jS | Windsurf-compat range normalization and registration surfacing (issue #161) | approved | [windsurf-compat-range-normalization-and-registration-surfacing-issue-161.md](./windsurf-compat-range-normalization-and-registration-surfacing-issue-161.md) |
| DESIGN-034r05iHrZUE6WrdtQIOmE | コアドメインモデル: Worker Definition・Resolver・Runtime Policy・tool vocabulary・error model | approved | [worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md](./worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md) |
| DESIGN-LAY8HNBqNyjH7L5DFvTPc2 | execution.maxTurns nullable 化・stall timeout・budget 枯渇 checkpoint/resume の実装設計 | approved | [worker-run-nullable-maxturns-and-stall-timeout.md](./worker-run-nullable-maxturns-and-stall-timeout.md) |
| DESIGN-EmeEdXi9TWZZW2PzZsp29y | worker schema magnitude 撤廃と INVALID_OUTPUT raw output 永続化の実装設計 | approved | [worker-schema-structure-not-magnitude.md](./worker-schema-structure-not-magnitude.md) |
| DESIGN-034zJ38fX1W3g0Zrceda1E | WorkflowCompiler の validation と materialization 境界 | approved | [workflow-contract-compiler.md](./workflow-contract-compiler.md) |
| DESIGN-034zT2MINVpakJb3AhS0m5 | Workflow file loader と interface CLI の接続設計 | approved | [workflow-file-interface.md](./workflow-file-interface.md) |
| DESIGN-034AQQsGub6ocuyzZg3Fuv | Design workflow orchestration benchmark harness | approved | [workflow-orchestration-benchmark.md](./workflow-orchestration-benchmark.md) |
| DESIGN-034zSWw1LYrRvcB471oQm2 | Workflow resource inference port 実装 | approved | [workflow-resource-inference.md](./workflow-resource-inference.md) |
| DESIGN-034r05iHrZUE6WsgJOy6Ny | workspace filesystem boundary と sandboxed tools | approved | [workspace-filesystem-boundary-sandboxed-tools.md](./workspace-filesystem-boundary-sandboxed-tools.md) |
| DESIGN-034r05iHy2gdnoIvdC141H | Write-scope and privacy deny enforcement design | approved | [write-scope-and-privacy-deny.md](./write-scope-and-privacy-deny.md) |
| DESIGN-034r05iHy2gdnoDFAeW1DN | code-writer and doc-writer worker contracts | approved | [writer-worker-contracts.md](./writer-worker-contracts.md) |
