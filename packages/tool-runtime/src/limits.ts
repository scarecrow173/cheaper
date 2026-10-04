/**
 * sandboxed tool 層の共有上限定数（DESIGN-0019）。
 * すべて入力ファイル総 byte 数 N と独立した有限値であり、呼び出し引数では
 * 引き上げられない（各 tool schema の `maximum` で固定）。テスト用の縮退は
 * `createXxxTool` の options 経由でのみ行う（既存 `evalPerFileMs` と同じ注入
 * パターン）。同じ上限を別表現で持たないこと（表現ドリフト防止）。
 */

/** `FileHandle.read` の chunk サイズ (C)。read/grep/sniff 共通。 */
export const IO_CHUNK_BYTES = 64 * 1024;
/** text/binary 判定の先頭読み取り byte 数。 */
export const SNIFF_BYTES = 8 * 1024;
/** read 1 call が返す本文 byte 上限 (R)。 */
export const READ_MAX_BYTES = 50 * 1024;
/** read `limit` 省略時の既定行数。 */
export const READ_DEFAULT_LIMIT = 2000;
/** read `limit` の最大値。 */
export const READ_MAX_LIMIT = 2000;
/** grep が評価する 1 行の byte 上限 (L)。超過行は未評価で破棄。 */
export const GREP_MAX_LINE_BYTES = 256 * 1024;
/** eval worker へ送る 1 batch の行テキスト合計 byte 上限 (B)。 */
export const GREP_BATCH_MAX_BYTES = 1024 * 1024;
/** eval worker へ送る 1 batch の行数上限 (K)。 */
export const GREP_BATCH_MAX_LINES = 1000;
/** grep `pattern` の文字列長上限。超過は invalid pattern error。 */
export const GREP_PATTERN_MAX_CHARS = 4096;
/** per-file 評価予算 ms（batch 間で累積）。 */
export const GREP_EVAL_PER_FILE_MS = 5000;
/** total 評価予算 ms（batch でリセットしない）。 */
export const GREP_EVAL_TOTAL_MS = 60_000;
/** 走査全体の wall-clock deadline ms（walk+I/O+行分割+eval 待機を含む）。 */
export const GREP_SCAN_DEADLINE_MS = 120_000;
/** 出力 1 行の表示文字数上限。 */
export const GREP_OUTPUT_LINE_MAX_CHARS = 500;
/** grep 出力本文の総 byte 上限 (O)。 */
export const GREP_OUTPUT_MAX_BYTES = 256 * 1024;
/** skip 理由ごとの代表 path 件数。 */
export const SKIP_FOOTER_MAX_ENTRIES = 20;
/** footer 内 1 path の表示文字数。 */
export const SKIP_PATH_MAX_CHARS = 200;
/** skip footer 全体の byte 上限 (S)。 */
export const SKIP_FOOTER_MAX_BYTES = 4 * 1024;
/** eval worker の `resourceLimits.maxOldGenerationSizeMb`。 */
export const GREP_WORKER_MAX_OLD_SPACE_MB = 64;
