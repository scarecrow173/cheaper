# Organic candidate prompt

Sent verbatim to every candidate (both arms). It states the goal only —
no mention of delegation, workers, skills, workflows, or evaluation.

---

このプロジェクトの docs/ 配下に API 仕様書が散らばっていて、どこに
何があるか把握しづらくなっています。docs/ 内の全ファイルを対象に、
エンドポイント（HTTP method + path）ごとの一覧をまとめた inventory
ドキュメントを reports/api-inventory.md に作成してください。一覧は
サービス領域（billing / auth / notify / ledger / catalog / shipping）
ごとにグループ化してください。

また、この棚卸しは docs が増えるたびに定期的にやり直したいので、
同じ手順を今後も簡単に回せるようにしておいてください。

作業はこのディレクトリ内だけで完結させてください。

---

## What it is designed to elicit (kept from candidates)

- `docs/` corpus is ~150 KB across 24 files in 6 domains → bulk-read
  territory (cheaper-bulk-read / tool-operator).
- A structured markdown artifact → doc-write territory
  (cheaper-doc-write).
- "同じ手順を今後も簡単に回せるように" → repeated-routine territory
  (cheaper-usage L0 → cheaper-workflow-discover L1 →
  cheaper-workflow-create L2; or an ad-hoc script in the control arm).
- Nothing in the wording names cheaper, skills, workers, or workflows;
  the arm-A `AGENTS.md` pointer is the only discoverability aid, which
  is what an organic repo would carry.
