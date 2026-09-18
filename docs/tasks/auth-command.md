---
id: "TASK-034r05iHQeQpDqapXCLs3G"
type: "task"
status: "done"
title: "cheaper auth command (login/status/logout)"
created: "2026-09-16"
updated: "2026-09-16"
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
  implements:
    - "docs/plans/auth-command.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/auth-login-command.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# cheaper auth command (login/status/logout)

## Work

`pi` 未インストール環境でも provider OAuth login できるよう、
`ModelRuntime.login` を吸収する `cheaper auth` を追加する。

- `auth status` / `auth login <provider> [--method]` / `auth logout <provider>`
- readline ベースの AuthInteraction（select 自動回答・manual_code・
  device_code/auth_url 表示・best-effort browser open）
- providers.md を pi 不要の手順に更新、bundle 再生成

## Acceptance Criteria

- [x] status/logout が envelope で動作
- [x] 未知 provider・非対応 type・非 TTY で適切に失敗
- [x] bundle に auth コマンドが含まれる
- [x] docs/index 更新・全検証 pass
