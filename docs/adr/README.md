# Architecture Decision Records

<!-- doc-suite:generated-index -->

Directory: `docs/adr`

| ID | Title | Status | File |
| --- | --- | --- | --- |
| ADR-034r05iHrZUE6W71rbZ6aR | Absorb provider OAuth and API-key login into the cheaper CLI (cheaper auth) | accepted | [absorb-provider-oauth-and-api-key-login-into-the-cheaper-cli-cheaper-auth.md](./absorb-provider-oauth-and-api-key-login-into-the-cheaper-cli-cheaper-auth.md) |
| ADR-034r05iHfSMW9SGV4dpcy2 | Agent Runtime として Pi SDK (@earendil-works/pi-coding-agent) を採用する | accepted | [agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md](./agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md) |
| ADR-pINoxFu3eduWaJCNBtOT1M | auth text/select prompt を injected CliIo の stderr writer へ bridge する | accepted | [auth-terminal-prompt-io-boundary.md](./auth-terminal-prompt-io-boundary.md) |
| ADR-034r05iHrZUE6WjfsVTWjO | real-provider e2e は Ollama auto-detect skip + apm install 隔離 consumer で実 provider 到達を検証する | accepted | [auto-detect-skip-real-provider-e2e-via-isolated-consumer.md](./auto-detect-skip-real-provider-e2e-via-isolated-consumer.md) |
| ADR-034zWjPbCrCggmdSxHvnzR | Bounded tool scheduler | accepted | [bounded-tool-scheduler.md](./bounded-tool-scheduler.md) |
| ADR-034r05iHrZUE6Wb2K7GmPw | bulk-reader の巨大ファイル I/O を有界 read・streaming grep・path-only find にする | accepted | [bulk-reader-huge-file-safe-io.md](./bulk-reader-huge-file-safe-io.md) |
| ADR-034r05iHrZUE6WeDAi63F3 | canonical setup/check entrypoint を mise TOML task として mise.toml に定義する | accepted | [canonical-setup-check-entrypoints-as-mise-toml-tasks.md](./canonical-setup-check-entrypoints-as-mise-toml-tasks.md) |
| ADR-034r05iHrZUE6W3U1rbJq4 | Use js-tiktoken o200k_base as the single canonical tokenizer | accepted | [canonical-tokenizer-js-tiktoken.md](./canonical-tokenizer-js-tiktoken.md) |
| ADR-034r05iHrZUE6WgAk8Cpqq | packages/cli/package.json を canonical version source とし check script で一致を強制する | accepted | [cli-package-json-as-canonical-version-with-check-enforcement.md](./cli-package-json-as-canonical-version-with-check-enforcement.md) |
| ADR-034rBhRB7QyWkAxRiTDXXq | Conservative shell read-candidate extraction inside the shared routing policy | accepted | [conservative-shell-read-candidate-extraction-inside-the-shared-routing-policy.md](./conservative-shell-read-candidate-extraction-inside-the-shared-routing-policy.md) |
| ADR-034r05iHrZUE6WizQ4LgPa | copilot adapter は self-contained script で stdout deny JSON + 常に exit 0 とし、.json gap は upstream 待ちとする | accepted | [copilot-adapter-separate-script-never-nonzero-exit.md](./copilot-adapter-separate-script-never-nonzero-exit.md) |
| ADR-034r05iHrZUE6W4XOqUBih | Keep 8192 tokens as the default large-read threshold for the pre_read_code hook | accepted | [default-large-read-threshold.md](./default-large-read-threshold.md) |
| ADR-034r05iHrZUE6WqcIaH1CU | Devin Local provider は Devin CLI 委譲 auth・isolated data dir・vendored stream で実装する | accepted | [devin-local-provider-via-cli-delegated-auth-and-isolated-data-dir.md](./devin-local-provider-via-cli-delegated-auth-and-isolated-data-dir.md) |
| ADR-034z1xMK43jRs3D79Urhpc | Distribute cheaper as Node 26 SEA standalone executables shared by direct release and APM hook bundle payload | accepted | [distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md](./distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md) |
| ADR-034r05iHrZUE6W5HVbwRSz | Distribute the cheaper CLI as a committed esbuild bundle inside the APM hook bundle | accepted | [distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md](./distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md) |
| ADR-034zGZpjoQlppwkkOs5gur | Tool Runtime / Workflow Engine epic の package 配置と共有契約を先行固定する | accepted | [fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md](./fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md) |
| ADR-034r05iHrZUE6WgelFx89a | grep の batchMaxBytes invariant 違反を tool 構築時に reject する | accepted | [grep-batch-max-bytes-invariant-reject-at-construction.md](./grep-batch-max-bytes-invariant-reject-at-construction.md) |
| ADR-034r6mq2Glfp3YlWyQdlIO | 大規模 fixture テストの eval budget は test 側で明示注入して環境非依存にする | accepted | [grep-eval-budget-test-stability-budget-injection.md](./grep-eval-budget-test-stability-budget-injection.md) |
| ADR-034r05iHrZUE6WcElqgQIr | auth interaction の browser 起動を注入可能な URL opener seam にする | accepted | [inject-url-opener-seam-for-auth-interaction.md](./inject-url-opener-seam-for-auth-interaction.md) |
| ADR-034zTe2Gt1o8oAkX43ilEm | Keep artifact storage invocation-local and bounded | accepted | [keep-artifact-storage-invocation-local-and-bounded.md](./keep-artifact-storage-invocation-local-and-bounded.md) |
| ADR-066GRjHT22hXEgey2dpq7f | providers guide を canonical とし distributed skill へは auth surface を要約反映する | accepted | [keep-providers-guide-canonical-while-mirroring-auth-surface-in-distributed-skill.md](./keep-providers-guide-canonical-while-mirroring-auth-surface-in-distributed-skill.md) |
| ADR-034r05iHrZUE6W9RDF0V73 | Keep the verbatim pre_read_code deployment as a documented no-op on non-Windsurf targets and defer per-target hook adapters | accepted | [keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md](./keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md) |
| ADR-034rzGnXqru47VgHNL16cS | measured offload は tool output token 和・estimated savings は pricing seam 経由 | accepted | [measured-offload-tool-output-token-estimated-savings-pricing-seam.md](./measured-offload-tool-output-token-estimated-savings-pricing-seam.md) |
| ADR-034r05iHlWosDpLI5iJRbx | Model tier 抽象と Model Resolver | accepted | [model-tier-model-resolver.md](./model-tier-model-resolver.md) |
| ADR-034rzTJYDRmre6D5OS8HDW | Normalize native hook payloads to a canonical routing request via per-target adapters and a shared policy module | accepted | [normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md](./normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md) |
| ADR-034r05iHrZUE6W2tpzbgKv | Normalize repair abort grace, provider failure classification, and agent-execution timeout anchor | accepted | [normalize-repair-abort-grace-provider-failure-classification-and-agent-execution.md](./normalize-repair-abort-grace-provider-failure-classification-and-agent-execution.md) |
| ADR-034zThoekpLqCcHhYO4MPe | One-dispatch release workflow | accepted | [one-dispatch-release-workflow.md](./one-dispatch-release-workflow.md) |
| ADR-034r05iHlWosDpKs4eBT59 | one-shot JSON-first CLI プロセスモデル | accepted | [one-shot-json-first-cli.md](./one-shot-json-first-cli.md) |
| ADR-034zT6bqTxOvK1nP0ELSyR | OperationManager の cancel 要求と実 settle を分離する | accepted | [operation-manager-settlement.md](./operation-manager-settlement.md) |
| ADR-034r05iHrZUE6Wn59hpaOg | Pi persistent state は userConfigDir/pi 配下へ ModelRuntime.create の明示 path で集約する | accepted | [pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md](./pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md) |
| ADR-034zA5nZuUdGLDMTGh2V6z | Pi sibling tool-call parallelism via runtime output-contract guidance | accepted | [pi-sibling-tool-call-parallelism-via-runtime-output-contract-guidance.md](./pi-sibling-tool-call-parallelism-via-runtime-output-contract-guidance.md) |
| ADR-P0zbksjMxOkaP5fE5ZkVfX | Price-based percentile auto tier classification | accepted | [price-based-percentile-auto-tier-classification.md](./price-based-percentile-auto-tier-classification.md) |
| ADR-034rzI0Yqw3L8AFM9diZct | Pricing metadata layer と curated exceptions | accepted | [pricing-metadata-layer-and-curated-exceptions.md](./pricing-metadata-layer-and-curated-exceptions.md) |
| ADR-034r2qodyqNY09yFc1ftZd | provider/auth guide は auth status を canonical surface として auth type 別に記述する | accepted | [provider-auth-guide-auth-status-canonical-surface-auth-type.md](./provider-auth-guide-auth-status-canonical-surface-auth-type.md) |
| ADR-034r05iHrZUE6WoaNDNjvX | provider 拡張は Pi extension loader ではなく bundled provider plugin の明示 register で行う | accepted | [provider-only-registration-via-explicit-bundled-plugin-contract.md](./provider-only-registration-via-explicit-bundled-plugin-contract.md) |
| ADR-034r05iHrZUE6Wm3dGBfWb | README は user-facing entrypoint に限定し、canonical source は目的別 guide に分離する | accepted | [readme-as-user-entrypoint-with-canonical-guides.md](./readme-as-user-entrypoint-with-canonical-guides.md) |
| ADR-034r05iHrZUE6WdKBH2eNM | development APM project と distributable cheaper package を nested package で物理分離する | accepted | [separate-dev-apm-project-via-nested-distributable-package.md](./separate-dev-apm-project-via-nested-distributable-package.md) |
| ADR-034r05iHrZUE6W82VsDbPq | Treat unmeasurable huge files as over-threshold via a core byte cap (fail-closed) | accepted | [treat-unmeasurable-huge-files-as-over-threshold-via-a-core-byte-cap-fail-closed.md](./treat-unmeasurable-huge-files-as-over-threshold-via-a-core-byte-cap-fail-closed.md) |
| ADR-034r05iHlWosDpP8BzaVZ8 | TypeScript + pnpm monorepo 構成 | accepted | [typescript-pnpm-monorepo.md](./typescript-pnpm-monorepo.md) |
| ADR-034r2eId2hXJGWDjBylKVE | usage.jsonl の記録対象 contract は実装を canonical として文書を揃える | accepted | [usage-jsonl-contract-canonical.md](./usage-jsonl-contract-canonical.md) |
| ADR-034r05iHrZUE6WljM1ZMiX | delegation 計測は userConfigDir の usage.jsonl への best-effort append とする | accepted | [usage-jsonl-in-user-config-dir-best-effort-append.md](./usage-jsonl-in-user-config-dir-best-effort-append.md) |
| ADR-034r05iHlWosDpQ6Z1LgJo | 実 provider smoke 検証に Ollama on Docker を使う | accepted | [use-ollama-on-docker-as-real-provider-for-smoke-verification.md](./use-ollama-on-docker-as-real-provider-for-smoke-verification.md) |
| ADR-034zIOoBXZtluQdBMlJQge | Version pinned launcher acquisition | accepted | [version-pinned-launcher-acquisition.md](./version-pinned-launcher-acquisition.md) |
| ADR-034r05iHlWosDpJ3DpG3XW | Worker Definition / Runtime Policy / Runtime Invariants の三層 capability モデル | accepted | [worker-definition-runtime-policy-runtime-invariants-capability.md](./worker-definition-runtime-policy-runtime-invariants-capability.md) |
| ADR-034r05iHlWosDpIq9VUsl7 | Worker を YAML frontmatter + Markdown の宣言的定義にする | accepted | [worker-yaml-frontmatter-markdown.md](./worker-yaml-frontmatter-markdown.md) |
| ADR-034zTpAnvDWILMV1LH66Wd | Workflow の既知 resource と safety fallback を分離する | accepted | [workflow-resource-inference-sources.md](./workflow-resource-inference-sources.md) |
| ADR-034r05iHlWosDpO699OwYF | workspace filesystem boundary は cheaper 提供の sandboxed tool 実装で強制する | accepted | [workspace-filesystem-boundary-cheaper-sandboxed-tool.md](./workspace-filesystem-boundary-cheaper-sandboxed-tool.md) |
| ADR-034r05iHlWosDpMXiQ38yf | 決定論的メタデータは wrapper が実測する | accepted | [wrapper-measured-deterministic-metadata.md](./wrapper-measured-deterministic-metadata.md) |
