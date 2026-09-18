# Architecture Decision Records

<!-- doc-suite:generated-index -->

Directory: `docs/adr`

| ID | Title | Status | File |
| --- | --- | --- | --- |
| ADR-0001 | Agent Runtime として Pi SDK (@earendil-works/pi-coding-agent) を採用する | accepted | [0001-agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md](./0001-agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md) |
| ADR-0002 | Worker を YAML frontmatter + Markdown の宣言的定義にする | accepted | [0002-worker-yaml-frontmatter-markdown.md](./0002-worker-yaml-frontmatter-markdown.md) |
| ADR-0003 | Worker Definition / Runtime Policy / Runtime Invariants の三層 capability モデル | accepted | [0003-worker-definition-runtime-policy-runtime-invariants-capability.md](./0003-worker-definition-runtime-policy-runtime-invariants-capability.md) |
| ADR-0004 | one-shot JSON-first CLI プロセスモデル | accepted | [0004-one-shot-json-first-cli.md](./0004-one-shot-json-first-cli.md) |
| ADR-0005 | Model tier 抽象と Model Resolver | accepted | [0005-model-tier-model-resolver.md](./0005-model-tier-model-resolver.md) |
| ADR-0006 | 決定論的メタデータは wrapper が実測する | accepted | [0006-wrapper-measured-deterministic-metadata.md](./0006-wrapper-measured-deterministic-metadata.md) |
| ADR-0007 | workspace filesystem boundary は cheaper 提供の sandboxed tool 実装で強制する | accepted | [0007-workspace-filesystem-boundary-cheaper-sandboxed-tool.md](./0007-workspace-filesystem-boundary-cheaper-sandboxed-tool.md) |
| ADR-0008 | TypeScript + pnpm monorepo 構成 | accepted | [0008-typescript-pnpm-monorepo.md](./0008-typescript-pnpm-monorepo.md) |
| ADR-0009 | 実 provider smoke 検証に Ollama on Docker を使う | accepted | [0009-use-ollama-on-docker-as-real-provider-for-smoke-verification.md](./0009-use-ollama-on-docker-as-real-provider-for-smoke-verification.md) |
| ADR-0010 | Normalize repair abort grace, provider failure classification, and agent-execution timeout anchor | accepted | [0010-normalize-repair-abort-grace-provider-failure-classification-and-agent-execution.md](./0010-normalize-repair-abort-grace-provider-failure-classification-and-agent-execution.md) |
| ADR-0011 | Use js-tiktoken o200k_base as the single canonical tokenizer | accepted | [0011-canonical-tokenizer-js-tiktoken.md](./0011-canonical-tokenizer-js-tiktoken.md) |
| ADR-0012 | Keep 8192 tokens as the default large-read threshold for the pre_read_code hook | accepted | [0012-default-large-read-threshold.md](./0012-default-large-read-threshold.md) |
| ADR-0013 | Distribute the cheaper CLI as a committed esbuild bundle inside the APM hook bundle | accepted | [0013-distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md](./0013-distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md) |
| ADR-0014 | Absorb provider OAuth and API-key login into the cheaper CLI (cheaper auth) | accepted | [0014-absorb-provider-oauth-and-api-key-login-into-the-cheaper-cli-cheaper-auth.md](./0014-absorb-provider-oauth-and-api-key-login-into-the-cheaper-cli-cheaper-auth.md) |
| ADR-0015 | Treat unmeasurable huge files as over-threshold via a core byte cap (fail-closed) | accepted | [0015-treat-unmeasurable-huge-files-as-over-threshold-via-a-core-byte-cap-fail-closed.md](./0015-treat-unmeasurable-huge-files-as-over-threshold-via-a-core-byte-cap-fail-closed.md) |
| ADR-0016 | Keep the verbatim pre_read_code deployment as a documented no-op on non-Windsurf targets and defer per-target hook adapters | accepted | [0016-keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md](./0016-keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md) |
| ADR-0017 | bulk-reader の巨大ファイル I/O を有界 read・streaming grep・path-only find にする | accepted | [0017-bulk-reader-huge-file-safe-io.md](./0017-bulk-reader-huge-file-safe-io.md) |
| ADR-0018 | auth interaction の browser 起動を注入可能な URL opener seam にする | accepted | [0018-inject-url-opener-seam-for-auth-interaction.md](./0018-inject-url-opener-seam-for-auth-interaction.md) |
| ADR-0019 | development APM project と distributable cheaper package を nested package で物理分離する | accepted | [0019-separate-dev-apm-project-via-nested-distributable-package.md](./0019-separate-dev-apm-project-via-nested-distributable-package.md) |
| ADR-0020 | canonical setup/check entrypoint を mise TOML task として mise.toml に定義する | accepted | [0020-canonical-setup-check-entrypoints-as-mise-toml-tasks.md](./0020-canonical-setup-check-entrypoints-as-mise-toml-tasks.md) |
| ADR-0022 | packages/cli/package.json を canonical version source とし check script で一致を強制する | accepted | [0022-cli-package-json-as-canonical-version-with-check-enforcement.md](./0022-cli-package-json-as-canonical-version-with-check-enforcement.md) |
| ADR-0023 | copilot adapter は self-contained script で stdout deny JSON + 常に exit 0 とし、.json gap は upstream 待ちとする | accepted | [0023-copilot-adapter-separate-script-never-nonzero-exit.md](./0023-copilot-adapter-separate-script-never-nonzero-exit.md) |
