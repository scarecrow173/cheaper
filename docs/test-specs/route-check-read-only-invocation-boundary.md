---
id: "TSPEC-034N7wHOe8W5M8Q6Uirqv7"
type: "test-spec"
status: "proposed"
title: "Route check read-only invocation boundary"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/route-check-diagnostics-issue-185.md"
    - "docs/designs/route-check-read-only-diagnostics-boundary.md"
  derived-by: []
  verifies:
    - "docs/specs/route-check-diagnostics-issue-185.md"
    - "docs/adr/use-a-read-only-embedded-resource-boundary-for-route-check.md"
    - "docs/designs/route-check-read-only-diagnostics-boundary.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Route check read-only invocation boundary

## Purpose

`route check` が invocation 検出から dispatch・error path まで strictly read-only であることを保証する。detection 規約と read-only contract が再定義された場合にのみ廃止する。

## Feature

Feature: `route check` invocation は option 順や malformed 形状に関わらず検出され、observability sink・generic Err path・全副作用経路が完全に抑止される。

## Rules

- `is_route_check_invocation` は strict parse と同じ `STRING_OPTIONS`/`BOOL_OPTIONS` arity table と `--` terminator 規約で token を畳み、先頭 positional が `["route", "check"]` の場合に検出する。余分な positional があっても検出し、reject は strict parse に任せる。
- global option の reorder（`--log debug route check`、`route --log debug check`、`route -- check`、`-- route check`）を同一 contract で検出する。
- unknown option・missing value を含む malformed invocation でも fail-safe に検出し、suppression と Err-path gate を有効化する。
- `route checks`・`route CHECK` のような非一致語は検出せず、通常の unknown-command error に委ねる。global `--help`/`--version`（`route check --help`/`--version` を含む）は既存 precedence を維持し、no-mutation contract の対象外とする。
- 検出時、observability `Activation` は level Off・stderr sink 無効として構築し、file/stderr tracing sink を一切初期化しない。ambient `CHEAPER_LOG`/`CHEAPER_LOG_FORMAT`/`--log`/`--log-format` が有効でも同じとする。
- dispatch の generic Err arm は detection flag で `resolved_user_config_dir`・`resolve_builtin_dir`・`record_usage*`・`finalize_worker_history` を bypass し、sanitized error envelope・stderr diagnostic・mapped exit のみを返す。
- `route check` は process spawn、workflow/worker 実行、network/auth、config・credential・History・Usage・RouteLogs・tracing file・diagnostic cache の write、embedded builtin の temp materialization を一切行わない。

## Examples

- Given `CHEAPER_LOG=debug` を設定した環境、when `route check` を clean temp dir で実行すると、`.cheaper/logs`・tracing subscriber・`cheaper-builtin-*` temp dir は生成されない。
- Given `cheaper --log debug route check` の invocation、when parse 前に detection されると、logging は完全に Off として扱われる。
- Given `cheaper route -- check` や `cheaper -- route check` の reorder、when detection を評価すると、`--` 以後の positional から `route check` として検出される。
- Given `cheaper route check --bogus` の malformed invocation、when strict parse が失敗しても、suppression と Err-path gate は有効のままになる。
- Given `cheaper route checks` や `cheaper route CHECK`、when detection を評価すると、検出されず通常の unknown-command error になる。
- Given `route check --help`、when strict parse が評価されると、既存の Help action として処理され diagnostics 実行にはならない。

## Guarantees

- detection が strict parse の option arity・`--` terminator 規約と一致し、reordered/malformed invocation を fail-safe に捉えることを固定する。
- ambient logging 有効を含む全経路で file/stderr sink・`.cheaper/logs` が生成されないことを fresh process で固定する（subscriber first-wins edge を避ける）。
- Err path が Usage/History/builtin-dir/user-config 解決へ進まないことを固定する。
- spawn・network・auth・全 filesystem mutation の probe が cross-platform で否定されることを固定する。

## Non-goals

- report の内容・diagnostic code・exit class の意味づけは output contract spec が担う。
- 非 `route check` command の observability・Err path 挙動は検証しない。
- product-wide panic boundary の sanitization は本 spec の範囲外とする。

## Risk

これらの保証が欠けると、ambient logging 下で診断対象 workspace へ log file が書き込まれ、malformed invocation が suppression をすり抜けて Usage/History/credential を変更し、strict read-only contract が名義だけのものになる。
