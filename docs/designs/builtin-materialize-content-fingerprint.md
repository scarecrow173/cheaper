---
id: "DESIGN-7G7UEmtrTGU4vh6og30Q0g"
type: "design"
status: "approved"
title: "builtin materialize fingerprint 設計"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "docs/specs/builtin-materialize-content-fingerprint.md"
    - "docs/adr/builtin-materialize-content-fingerprint.md"
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
  related:
    - "docs/specs/builtin-materialize-content-fingerprint.md"
    - "docs/adr/builtin-materialize-content-fingerprint.md"
    - "docs/plans/builtin-materialize-content-fingerprint.md"
  refines:
    - "docs/specs/builtin-materialize-content-fingerprint.md"
  refined-by: []
  derives-from:
    - "docs/specs/builtin-materialize-content-fingerprint.md"
    - "docs/adr/builtin-materialize-content-fingerprint.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "crates/cheaper-cli/src/embedded.rs"
  defers: []
  deferred-by: []
---

# 設計: builtin materialize content fingerprint

## 変更点

`crates/cheaper-cli/src/embedded.rs` のみ。

### 1. fingerprint 関数

`include_dir` 0.7 の `Dir::files()`/`entries()` は**直下のみ**列挙する
(`workers/schemas/*.json` の nested file を含まない)。そのため
`extract_dir` と同じ `entries()` 再帰で file を集める共通 helper
`collect_files(&Dir, &mut Vec<&File>)` を導入し、**fingerprint 入力と
extract 出力を同一走査から導出**する(入力集合と展開集合の drift を
構造的に防ぐ)。

```rust
fn collect_files<'a>(dir: &'a Dir<'a>, out: &mut Vec<&'a File<'a>>) {
    for entry in dir.entries() {
        match entry {
            DirEntry::File(f) => out.push(f),
            DirEntry::Dir(d) => collect_files(d, out),
        }
    }
}

fn embedded_fingerprint() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut files: Vec<&File> = Vec::new();
    collect_files(&WORKERS_DIR, &mut files);
    files.sort_by_key(|f| f.path());   // 順序を決定的化
    let mut hasher = DefaultHasher::new();
    for f in files {
        f.path().hash(&mut hasher);
        f.contents().hash(&mut hasher);
    }
    "policy.yaml".hash(&mut hasher);
    POLICY_YAML.hash(&mut hasher);
    "version.json".hash(&mut hasher);
    VERSION_JSON.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
```

- `extract_dir` も `collect_files` 経由に書き換え、worker file の
  列挙を1箇所に集約する(extract の出力対象 = fingerprint の入力対象)。
- `DefaultHasher::new()` は固定 key で同一 toolchain 上の process 間
  決定的。rustc 版間で出力が変わり得るが、影響は新 dir 生成のみで
  staleness にはならない(ADR の許容短所)。外部 hash crate は追加しない。
- `file.path()` は include root 相対 — dir 名に関係なく stable。

### 2. dir 名

`materialize_builtin_dir` 内の既存の `version` 解決
(`embedded_version().ok_or_else(...)`)はそのまま使い、dir 名のみ
変更する:

```rust
let target = std::env::temp_dir()
    .join(format!("cheaper-builtin-{version}-{}", embedded_fingerprint()));
```

- `builtin_dir()` が返す経路が fingerprint 込みに変わるのみ。
  `.ready`、extract、cache `OnceLock` は不変。
- file header の doc comment(`<temp>/cheaper-builtin-<version>/`)も
  `cheaper-builtin-<version>-<fingerprint>` へ合わせて更新する。

### 3. 検証

- unit test: fingerprint が決定的(同一入力→同一値)、contents 差分で
  変わること。`materialize` は static `OnceLock` cache を持ち in-process
  再試行ができないため、fingerprint 関数と dir 名生成を分離してテスト。
- 手動確認: dev binary で `builtin_dir` を実行 → dir 名に hash が出る
  ことを確認(#113/#114 作業中に観測した stale cache の再発防止を目視)。

## 非変更

- `resolve_builtin_dir`/`CliHooks.builtin_dir` override。
- `extract_builtin` の展開仕様。
- workflow/run での builtin dir 利用側(透過)。
