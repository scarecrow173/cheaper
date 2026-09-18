# DESIGN Documents

<!-- doc-suite:generated-index -->

Directory: `docs/designs`

| ID | Title | Status | File |
| --- | --- | --- | --- |
| DESIGN-OVERVIEW | System Design Overview | draft | [overview.md](./overview.md) |
| DESIGN-0001 | コアドメインモデル: Worker Definition・Resolver・Runtime Policy・tool vocabulary・error model | approved | [0001-worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md](./0001-worker-definition-resolver-runtime-policy-tool-vocabulary-error-model.md) |
| DESIGN-0002 | workspace filesystem boundary と sandboxed tools | approved | [0002-workspace-filesystem-boundary-sandboxed-tools.md](./0002-workspace-filesystem-boundary-sandboxed-tools.md) |
| DESIGN-0003 | Model Resolver | approved | [0003-model-resolver.md](./0003-model-resolver.md) |
| DESIGN-0004 | Pi 実行エンジン: session・制限・abort・retry・出力抽出・repair・usage | approved | [0004-pi-execution-engine.md](./0004-pi-execution-engine.md) |
| DESIGN-0005 | CLI contract・result/error envelope・exit code | approved | [0005-cli-contract-envelope-exit-codes.md](./0005-cli-contract-envelope-exit-codes.md) |
| DESIGN-0006 | bulk-reader contract | approved | [0006-bulk-reader-contract.md](./0006-bulk-reader-contract.md) |
| DESIGN-0007 | cheap-writer contract | superseded | [0007-cheap-writer-contract.md](./0007-cheap-writer-contract.md) |
| DESIGN-0008 | リポジトリ構成とツールチェーン | approved | [0008-repository-structure-toolchain.md](./0008-repository-structure-toolchain.md) |
| DESIGN-0009 | Ollama real-provider smoke verification setup | approved | [0009-ollama-real-provider-smoke-verification-setup.md](./0009-ollama-real-provider-smoke-verification-setup.md) |
| DESIGN-0010 | Repair abort and failure normalization hardening | approved | [0010-repair-abort-and-failure-normalization-hardening.md](./0010-repair-abort-and-failure-normalization-hardening.md) |
| DESIGN-0011 | code-writer and doc-writer worker contracts | approved | [0011-writer-worker-contracts.md](./0011-writer-worker-contracts.md) |
| DESIGN-0012 | Token counter contract (core module, count_tokens tool, count-tokens CLI) | approved | [0012-token-counter-contract.md](./0012-token-counter-contract.md) |
| DESIGN-0013 | APM skills and pre_read_code delegation hook | approved | [0013-apm-skills-and-pre-read-hook.md](./0013-apm-skills-and-pre-read-hook.md) |
| DESIGN-0014 | cheaper CLI bundle layout inside .apm/hooks/cli | approved | [0014-cli-bundle-layout.md](./0014-cli-bundle-layout.md) |
| DESIGN-0015 | Write-scope and privacy deny enforcement design | approved | [0015-write-scope-and-privacy-deny.md](./0015-write-scope-and-privacy-deny.md) |
| DESIGN-0016 | cheaper auth command design (provider OAuth absorbed by the CLI) | approved | [0016-auth-command.md](./0016-auth-command.md) |
| DESIGN-0017 | Huge-file byte cap guard and fail-closed hook | approved | [0017-huge-file-byte-cap-guard-and-fail-closed-hook.md](./0017-huge-file-byte-cap-guard-and-fail-closed-hook.md) |
| DESIGN-0018 | Cross-agent distribution verification | approved | [0018-cross-agent-distribution-verification.md](./0018-cross-agent-distribution-verification.md) |
| DESIGN-0019 | Bulk-reader huge-file safe I/O | approved | [0019-bulk-reader-huge-file-safe-io.md](./0019-bulk-reader-huge-file-safe-io.md) |
| DESIGN-0020 | Auth URL opener seam (injectable browser launch) | approved | [0020-auth-url-opener-seam.md](./0020-auth-url-opener-seam.md) |
| DESIGN-0021 | Development APM project / distributable cheaper package layout | approved | [0021-dev-apm-project-and-distributable-package-layout.md](./0021-dev-apm-project-and-distributable-package-layout.md) |
| DESIGN-0022 | Repository-owned setup/check entrypoints via mise TOML tasks | approved | [0022-repo-owned-setup-check-entrypoints.md](./0022-repo-owned-setup-check-entrypoints.md) |
| DESIGN-0023 | grep batchMaxBytes invariant enforcement | approved | [0023-grep-batch-max-bytes-invariant-enforcement.md](./0023-grep-batch-max-bytes-invariant-enforcement.md) |
| DESIGN-0024 | Unified version source of truth | approved | [0024-unified-version-source-of-truth.md](./0024-unified-version-source-of-truth.md) |
| DESIGN-0025 | Copilot pre-read hook adapter | approved | [0025-copilot-pre-read-hook-adapter.md](./0025-copilot-pre-read-hook-adapter.md) |
| DESIGN-0026 | Real-provider worker E2E via Ollama | approved | [0026-ollama-real-provider-worker-e2e.md](./0026-ollama-real-provider-worker-e2e.md) |
| DESIGN-0027 | Delegation usage measurement log (usage.jsonl) | approved | [0027-delegation-usage-measurement-log.md](./0027-delegation-usage-measurement-log.md) |
| DESIGN-0028 | User-facing documentation information architecture | approved | [0028-user-facing-docs-information-architecture.md](./0028-user-facing-docs-information-architecture.md) |
| DESIGN-0029 | Pi state paths under userConfigDir/pi | approved | [0029-pi-state-paths-under-userconfigdir-pi.md](./0029-pi-state-paths-under-userconfigdir-pi.md) |
| DESIGN-0030 | Provider-only registration layer | approved | [0030-provider-only-registration-layer.md](./0030-provider-only-registration-layer.md) |
| DESIGN-0031 | Devin Local provider plugin | implemented | [0031-devin-local-provider-plugin.md](./0031-devin-local-provider-plugin.md) |
