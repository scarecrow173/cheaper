//! Port of `packages/tool-runtime/src/limits.ts`.
//!
//! sandboxed tool 層の共有上限定数(DESIGN-0019)。すべて入力ファイル
//! 総 byte 数 N と独立した有限値であり、呼び出し引数では引き上げ
//! られない(各 tool schema の `maximum` で固定)。テスト用の縮退は
//! `createXxxTool` の options 経由でのみ行う(既存 `evalPerFileMs` と
//! 同じ注入パターン)。同じ上限を別表現で持たないこと(表現ドリフト防止)。
//!
//! byte・行数・文字数の上限は buffer 長と比較するため `usize`、
//! ms 系の時間上限は `Duration::from_millis` に渡すため `u64` とする
//! (値は TS と同一)。

/// `FileHandle.read` の chunk サイズ (C)。read/grep/sniff 共通。
pub const IO_CHUNK_BYTES: usize = 64 * 1024;
/// text/binary 判定の先頭読み取り byte 数。
pub const SNIFF_BYTES: usize = 8 * 1024;
/// read 1 call が返す本文 byte 上限 (R)。
pub const READ_MAX_BYTES: usize = 50 * 1024;
/// read `limit` 省略時の既定行数。
pub const READ_DEFAULT_LIMIT: usize = 2000;
/// read `limit` の最大値。
pub const READ_MAX_LIMIT: usize = 2000;
/// grep が評価する 1 行の byte 上限 (L)。超過行は未評価で破棄。
pub const GREP_MAX_LINE_BYTES: usize = 256 * 1024;
/// eval worker へ送る 1 batch の行テキスト合計 byte 上限 (B)。
pub const GREP_BATCH_MAX_BYTES: usize = 1024 * 1024;
/// eval worker へ送る 1 batch の行数上限 (K)。
pub const GREP_BATCH_MAX_LINES: usize = 1000;
/// grep `pattern` の文字列長上限。超過は invalid pattern error。
pub const GREP_PATTERN_MAX_CHARS: usize = 4096;
/// per-file 評価予算 ms(batch 間で累積)。
pub const GREP_EVAL_PER_FILE_MS: u64 = 5_000;
/// total 評価予算 ms(batch でリセットしない)。
pub const GREP_EVAL_TOTAL_MS: u64 = 60_000;
/// 走査全体の wall-clock deadline ms(walk+I/O+行分割+eval 待機を含む)。
pub const GREP_SCAN_DEADLINE_MS: u64 = 120_000;
/// 出力 1 行の表示文字数上限。
pub const GREP_OUTPUT_LINE_MAX_CHARS: usize = 500;
/// grep 出力本文の総 byte 上限 (O)。
pub const GREP_OUTPUT_MAX_BYTES: usize = 256 * 1024;
/// skip 理由ごとの代表 path 件数。
pub const SKIP_FOOTER_MAX_ENTRIES: usize = 20;
/// footer 内 1 path の表示文字数。
pub const SKIP_PATH_MAX_CHARS: usize = 200;
/// skip footer 全体の byte 上限 (S)。
pub const SKIP_FOOTER_MAX_BYTES: usize = 4 * 1024;
/// eval worker の `resourceLimits.maxOldGenerationSizeMb`。
pub const GREP_WORKER_MAX_OLD_SPACE_MB: usize = 64;
