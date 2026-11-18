# Incident root-cause digest（四半期棚卸し）

- 生成日: 2026-11-18
- 対象: `docs/incidents/` 全 30 件（INC-0101 〜 INC-0130 / 2026-01-01 〜 2026-09-28）
- 手法: `scripts/extract-incident-causes.sh` で各レポート末尾節の原因文を決定的に抽出し、同一原因でクラスタリングした結果を精査
- 再実施手順: `docs/runbooks/quarterly-root-cause-digest.md`

## サマリー

30 件の根本原因は **5 パターンに完全に集約** された。各パターン 6 件ずつで、
全パターンが 6 サービス（checkout / search / billing-sync / notify-relay /
ledger-writer / gateway-edge）すべてにまたがって発生している。
サービス固有の問題ではなく、プラットフォーム共通の防護機構の欠如・
適用漏れが原因系となっている。

| # | パターン | 件数 | インシデント | 代表例 |
|---|----------|------|--------------|--------|
| P1 | DB コネクションプール枯渇（デフォルトサイズのまま負荷増大） | 6 | INC-0101〜0106 | INC-0101, INC-0103, INC-0106 |
| P2 | 社内/service-mesh 証明書の失効（更新自動化の適用範囲外） | 6 | INC-0107〜0112 | INC-0107, INC-0110, INC-0112 |
| P3 | ポイズンメッセージ起因のリトライストーム（DLQ 未整備） | 6 | INC-0113〜0118 | INC-0113, INC-0116, INC-0118 |
| P4 | 宣言マニフェストからの設定ドリフト（手動ホットフィックス残置） | 6 | INC-0119〜0124 | INC-0119, INC-0122, INC-0124 |
| P5 | 上流依存先へのブレーカー/バルクヘッド欠如 | 6 | INC-0125〜0130 | INC-0125, INC-0128, INC-0130 |

## P1. DB コネクションプール枯渇

- 対象: INC-0101, INC-0102, INC-0103, INC-0104, INC-0105, INC-0106（全 6 サービス / 2026-02-11 〜 2026-07-16）
- 代表例: INC-0101（checkout）, INC-0103（billing-sync）, INC-0106（gateway-edge）
- 原因（原文）:
  > The database connection pool was exhausted: pool size stayed at its default while concurrency grew, so requests queued on pool acquisition until they timed out. Raising the pool limit and adding an acquisition-timeout alert resolved it.
- 解説: プールサイズをデフォルトのまま運用した結果、同時実行数の増大に追従できず取得待ちがタイムアウトまで積み上がった。初期設定の棚卸し不足が全サービス共通で再現している。
- 適用対策: プール上限の引き上げ + 取得タイムアウトアラート追加。

## P2. 内部証明書の失効（更新自動化の適用漏れ）

- 対象: INC-0107, INC-0108, INC-0109, INC-0110, INC-0111, INC-0112（全 6 サービス / 2026-01-20 〜 2026-09-28）
- 代表例: INC-0107（checkout）, INC-0110（notify-relay）, INC-0112（gateway-edge）
- 原因（原文）:
  > An internal certificate had quietly reached its expiry date. Renewal automation covered only the public-facing certificates, not the service-mesh ones. Manual renewal restored mTLS; coverage was added to the renewal job.
- 解説: 証明書更新の自動化が公開向け証明書のみを対象としており、service-mesh 証明書がサイレントに失効して mTLS が破断した。自動化の「適用範囲の盲点」が典型。
- 適用対策: 手動更新で復旧 + 更新ジョブの対象範囲拡大。

## P3. ポイズンメッセージ起因のリトライストーム

- 対象: INC-0113, INC-0114, INC-0115, INC-0116, INC-0117, INC-0118（全 6 サービス / 2026-01-01 〜 2026-09-09）
- 代表例: INC-0113（checkout）, INC-0116（notify-relay）, INC-0118（gateway-edge）
- 原因（原文）:
  > Consumers could not keep up after one poison message forced repeated redeliveries; the backlog grew until the queue depth alarm fired. Dead-letter routing for unprocessable messages stopped the retry storm.
- 解説: 処理不能メッセージ 1 件が再配送を繰り返し、デッドレター経路が無いためコンシューマー容量を食い潰してバックログ化。メッセージング基盤の標準設定欠如。
- 適用対策: 処理不能メッセージのデッドレター配送を導入。

## P4. 宣言マニフェストからの設定ドリフト

- 対象: INC-0119, INC-0120, INC-0121, INC-0122, INC-0123, INC-0124（全 6 サービス / 2026-02-11 〜 2026-07-16）
- 代表例: INC-0119（checkout）, INC-0122（billing-sync）, INC-0124（gateway-edge）
- 原因（原文）:
  > The deployed configuration had drifted from the declared manifest — a manual hotfix months earlier changed a feature flag that was never written back. Reconciling the manifest and adding a drift check to deploys closed the gap.
- 解説: 数か月前の手動ホットフィックスによる feature flag 変更が宣言へ書き戻されず、実環境と manifest が乖離し続けた。手動変更の追跡手段とドリフト検知の双方が不在。
- 適用対策: manifest との整合回復 + デプロイ時ドリフトチェック追加。

## P5. 上流依存先へのブレーカー/バルクヘッド欠如

- 対象: INC-0125, INC-0126, INC-0127, INC-0128, INC-0129, INC-0130（全 6 サービス / 2026-01-20 〜 2026-09-28）
- 代表例: INC-0125（checkout）, INC-0128（notify-relay）, INC-0130（gateway-edge）
- 原因（原文）:
  > An upstream partner API began exceeding its latency SLO; our client had no circuit breaker, so slow responses exhausted worker capacity and the failure cascaded inward. A bulkhead plus breaker now isolates the dependency.
- 解説: 外部パートナー API のレイテンシ SLO 超過に対し遮断機構が無く、遅延応答がワーカー容量を使い切って障害が内側へ連鎖した。依存先隔離パターンの未適用。
- 適用対策: バルクヘッド + サーキットブレーカーで依存先を隔離。

## 横断観察

- 5 パターンすべてが「既知の防護機構の欠如または適用範囲漏れ」に帰着する
  （プール上限、証明書更新対象、DLQ、ドリフト検知、ブレーカー）。
  未知の障害モードではなく、標準ハードニングの徹底で抑えられる類である。
- 各パターンが 6 サービス全てを順に襲っているため、サービス個別対応ではなく
  プラットフォーム側の共通基盤（設定テンプレート・デプロイチェック・
  ライブラリ標準）で一括是正するのが費用対効果が高い。
- 全 30 件が検出経路として synthetic monitoring 起点であり、原因系ごとの
  事前アラート（プール取得待ち・証明書残日数・キュー深度・ドリフト・依存先
  レイテンシ）への置き換え余地がある。

## 再実施方法

四半期ごとの再棚卸し手順は `docs/runbooks/quarterly-root-cause-digest.md` を参照
（抽出スクリプトと cheaper workflow の両経路を記載）。
