# docs/

`cheaper` のドキュメント。利用者向け guide・開発者向け記録・
設計決定記録の 3 層で構成する。

## User guides

利用者が `cheaper` を使うための guide。

- [usage.md](guides/usage.md) — CLI の command・option・
  stdin/stdout contract・envelope・exit code・auth・計測 log
- [configuration.md](guides/configuration.md) — config dir の
  解決・優先度・`models.yaml`・`policy.yaml`・環境変数
- [providers.md](guides/providers.md) — provider / auth の設定・
  `models.json`・OAuth login・tier 割当
- [workers.md](guides/workers.md) — worker の概念・built-in
  worker・custom worker・schema・policy との関係
- [parent-agent-integration.md](guides/parent-agent-integration.md) —
  parent agent との役割分担・skills/hooks・install・target 別
  挙動・連携確認

## Contributor / development

リポジトリ開発の entrypoint は
[../CONTRIBUTING.md](../CONTRIBUTING.md)。Coding Agent を使う
場合は [../AGENTS.md](../AGENTS.md)（agent 向け指示）。

## Architecture / decision records

document-driven development の記録。設計・決定・実装・検証を
canonical document として管理する。

| 種類 | 場所 | 内容 |
| --- | --- | --- |
| idea | [ideas/](ideas/) | 未採用・保留中の設計案、promote/discard 待ちの deferred point |
| spec | [specs/](specs/) | 何を作るか・受け入れ条件 |
| ADR | [adr/](adr/) | 重要な設計決定とその根拠 |
| design | [designs/](designs/) | spec/ADR を実装可能な形に詳細化した設計 |
| discovery | [discovery/](discovery/) | 調査・issue 整理・仮説・gap 分析 |
| plan | [plans/](plans/) | 実装単位の file map・task 分解・検証方針 |
| task | [tasks/](tasks/) | plan を薄い実装 slice に分けた作業単位 |
| impl | [impl/](impl/) | 実装記録（ir）と実験・検証記録（exp） |
| test-spec | [test-specs/](test-specs/) | 何を検証すべきかの正準仕様 |
| handoff | [handoff/](handoff/) | フェーズ間・version 間の引き継ぎ記録 |
| guide | [guides/](guides/) | 上記の user-facing guide |

一般的な利用には guide だけで足りる。設計判断・実装履歴・
検証結果が必要なときだけ architecture 記録を参照する。
