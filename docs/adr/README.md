# Architecture Decision Records

<!-- doc-suite:generated-index -->

Directory: `docs/adr`

| ID | Title | Status | File |
| --- | --- | --- | --- |
| ADR-034r05iHrZUE6W71rbZ6aR | Absorb provider OAuth and API-key login into the cheaper CLI (cheaper auth) | accepted | [absorb-provider-oauth-and-api-key-login-into-the-cheaper-cli-cheaper-auth.md](./absorb-provider-oauth-and-api-key-login-into-the-cheaper-cli-cheaper-auth.md) |
| ADR-034r05iHfSMW9SGV4dpcy2 | Agent Runtime として Pi SDK (@earendil-works/pi-coding-agent) を採用する | accepted | [agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md](./agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md) |
| ADR-034r05iHrZUE6WjfsVTWjO | real-provider e2e は Ollama auto-detect skip + apm install 隔離 consumer で実 provider 到達を検証する | accepted | [auto-detect-skip-real-provider-e2e-via-isolated-consumer.md](./auto-detect-skip-real-provider-e2e-via-isolated-consumer.md) |
| ADR-034r05iHrZUE6Wb2K7GmPw | bulk-reader の巨大ファイル I/O を有界 read・streaming grep・path-only find にする | accepted | [bulk-reader-huge-file-safe-io.md](./bulk-reader-huge-file-safe-io.md) |
| ADR-034r05iHrZUE6WeDAi63F3 | canonical setup/check entrypoint を mise TOML task として mise.toml に定義する | accepted | [canonical-setup-check-entrypoints-as-mise-toml-tasks.md](./canonical-setup-check-entrypoints-as-mise-toml-tasks.md) |
| ADR-034r05iHrZUE6W3U1rbJq4 | Use js-tiktoken o200k_base as the single canonical tokenizer | accepted | [canonical-tokenizer-js-tiktoken.md](./canonical-tokenizer-js-tiktoken.md) |
| ADR-034r05iHrZUE6WgAk8Cpqq | packages/cli/package.json を canonical version source とし check script で一致を強制する | accepted | [cli-package-json-as-canonical-version-with-check-enforcement.md](./cli-package-json-as-canonical-version-with-check-enforcement.md) |
| ADR-034r05iHrZUE6WizQ4LgPa | copilot adapter は self-contained script で stdout deny JSON + 常に exit 0 とし、.json gap は upstream 待ちとする | accepted | [copilot-adapter-separate-script-never-nonzero-exit.md](./copilot-adapter-separate-script-never-nonzero-exit.md) |
| ADR-034r05iHrZUE6W4XOqUBih | Keep 8192 tokens as the default large-read threshold for the pre_read_code hook | accepted | [default-large-read-threshold.md](./default-large-read-threshold.md) |
| ADR-034r05iHrZUE6WqcIaH1CU | Devin Local provider は Devin CLI 委譲 auth・isolated data dir・vendored stream で実装する | accepted | [devin-local-provider-via-cli-delegated-auth-and-isolated-data-dir.md](./devin-local-provider-via-cli-delegated-auth-and-isolated-data-dir.md) |
| ADR-034r05iHrZUE6W5HVbwRSz | Distribute the cheaper CLI as a committed esbuild bundle inside the APM hook bundle | accepted | [distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md](./distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md) |
| ADR-034r05iHrZUE6WgelFx89a | grep の batchMaxBytes invariant 違反を tool 構築時に reject する | accepted | [grep-batch-max-bytes-invariant-reject-at-construction.md](./grep-batch-max-bytes-invariant-reject-at-construction.md) |
| ADR-034r05iHrZUE6WcElqgQIr | auth interaction の browser 起動を注入可能な URL opener seam にする | accepted | [inject-url-opener-seam-for-auth-interaction.md](./inject-url-opener-seam-for-auth-interaction.md) |
| ADR-066GRjHT22hXEgey2dpq7f | providers guide を canonical とし distributed skill へは auth surface を要約反映する | accepted | [keep-providers-guide-canonical-while-mirroring-auth-surface-in-distributed-skill.md](./keep-providers-guide-canonical-while-mirroring-auth-surface-in-distributed-skill.md) |
| ADR-034r05iHrZUE6W9RDF0V73 | Keep the verbatim pre_read_code deployment as a documented no-op on non-Windsurf targets and defer per-target hook adapters | accepted | [keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md](./keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md) |
| ADR-034r05iHlWosDpLI5iJRbx | Model tier 抽象と Model Resolver | accepted | [model-tier-model-resolver.md](./model-tier-model-resolver.md) |
| ADR-034r05iHrZUE6W2tpzbgKv | Normalize repair abort grace, provider failure classification, and agent-execution timeout anchor | accepted | [normalize-repair-abort-grace-provider-failure-classification-and-agent-execution.md](./normalize-repair-abort-grace-provider-failure-classification-and-agent-execution.md) |
| ADR-034r05iHlWosDpKs4eBT59 | one-shot JSON-first CLI プロセスモデル | accepted | [one-shot-json-first-cli.md](./one-shot-json-first-cli.md) |
| ADR-034r05iHrZUE6Wn59hpaOg | Pi persistent state は userConfigDir/pi 配下へ ModelRuntime.create の明示 path で集約する | accepted | [pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md](./pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md) |
| ADR-034r2qodyqNY09yFc1ftZd | provider/auth guide は auth status を canonical surface として auth type 別に記述する | accepted | [provider-auth-guide-auth-status-canonical-surface-auth-type.md](./provider-auth-guide-auth-status-canonical-surface-auth-type.md) |
| ADR-034r05iHrZUE6WoaNDNjvX | provider 拡張は Pi extension loader ではなく bundled provider plugin の明示 register で行う | accepted | [provider-only-registration-via-explicit-bundled-plugin-contract.md](./provider-only-registration-via-explicit-bundled-plugin-contract.md) |
| ADR-034r05iHrZUE6Wm3dGBfWb | README は user-facing entrypoint に限定し、canonical source は目的別 guide に分離する | accepted | [readme-as-user-entrypoint-with-canonical-guides.md](./readme-as-user-entrypoint-with-canonical-guides.md) |
| ADR-034r05iHrZUE6WdKBH2eNM | development APM project と distributable cheaper package を nested package で物理分離する | accepted | [separate-dev-apm-project-via-nested-distributable-package.md](./separate-dev-apm-project-via-nested-distributable-package.md) |
| ADR-034r05iHrZUE6W82VsDbPq | Treat unmeasurable huge files as over-threshold via a core byte cap (fail-closed) | accepted | [treat-unmeasurable-huge-files-as-over-threshold-via-a-core-byte-cap-fail-closed.md](./treat-unmeasurable-huge-files-as-over-threshold-via-a-core-byte-cap-fail-closed.md) |
| ADR-034r05iHlWosDpP8BzaVZ8 | TypeScript + pnpm monorepo 構成 | accepted | [typescript-pnpm-monorepo.md](./typescript-pnpm-monorepo.md) |
| ADR-034r05iHrZUE6WljM1ZMiX | delegation 計測は userConfigDir の usage.jsonl への best-effort append とする | accepted | [usage-jsonl-in-user-config-dir-best-effort-append.md](./usage-jsonl-in-user-config-dir-best-effort-append.md) |
| ADR-034r05iHlWosDpQ6Z1LgJo | 実 provider smoke 検証に Ollama on Docker を使う | accepted | [use-ollama-on-docker-as-real-provider-for-smoke-verification.md](./use-ollama-on-docker-as-real-provider-for-smoke-verification.md) |
| ADR-034r05iHlWosDpJ3DpG3XW | Worker Definition / Runtime Policy / Runtime Invariants の三層 capability モデル | accepted | [worker-definition-runtime-policy-runtime-invariants-capability.md](./worker-definition-runtime-policy-runtime-invariants-capability.md) |
| ADR-034r05iHlWosDpIq9VUsl7 | Worker を YAML frontmatter + Markdown の宣言的定義にする | accepted | [worker-yaml-frontmatter-markdown.md](./worker-yaml-frontmatter-markdown.md) |
| ADR-034r05iHlWosDpO699OwYF | workspace filesystem boundary は cheaper 提供の sandboxed tool 実装で強制する | accepted | [workspace-filesystem-boundary-cheaper-sandboxed-tool.md](./workspace-filesystem-boundary-cheaper-sandboxed-tool.md) |
| ADR-034r05iHlWosDpMXiQ38yf | 決定論的メタデータは wrapper が実測する | accepted | [wrapper-measured-deterministic-metadata.md](./wrapper-measured-deterministic-metadata.md) |
