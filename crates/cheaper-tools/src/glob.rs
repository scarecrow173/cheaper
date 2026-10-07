//! `path.posix.matchesGlob`(minimatch v10 ベース)の Rust port。
//!
//! `find`/`grep` 双方が使う共有 glob matcher。残る既知の差分は
//! `docs/impl/ir/phase-2-port-cheaper-tools.md` に記載する。
//!
//! Node 実測で確認した主要な挙動:
//!
//! - pattern / path は UTF-16 code unit 列として照合する。`?` は
//!   astral char に対して 2 unit を要求する(`??` で `😀` に一致)。
//!   POSIX class(`[:alpha:]` 等)を含む segment は regex `/u` flag 相当で
//!   code point 単位に評価される。
//! - `\` は `/` へ正規化してから評価する(path.posix の区切り変換)。
//! - `**` は「segment 全体がちょうど `**`」のときのみ globstar。
//!   途中の `**` は 1 segment を必ず消費し、末尾の `**` は最終 segment
//!   を必ず消費する(`a/**` は `a` に一致しない)。
//! - dot 規則: segment pattern が wildcard/class で始まる場合 `.` 始まり
//!   の path segment に一致しない(literal `.` 始まりや `[.]` は除く)。
//!   `.`+magic / `..`+magic 始まりは `.`/`..` そのものへの一致を禁止。
//!   extglob 先頭の segment にはこの guard が無い(Node 仕様)。
//! - brace expansion(`{a,b}`、`{1..9..2}`、`{a..z}`、nested、pad)と
//!   extglob(`?` `*` `+` `@` `!` + `(...)`)をサポートする。
//!   `!` は「残りが alternative+suffix に full match しない」guard として
//!   実装する(minimatch の `#fillNegs` + `(?!(?:...(?:$|/)))` 相当)。
//! - `nocase` は `process.platform` 依存(win32/darwin のみ)。
//!   `nocaseMagicOnly` 既定が nocase に従うため、全 literal segment は
//!   nocase でも厳密一致、magic 部分のみ case-insensitive。
//!
//! 正規表現生成の代わりに等価な memo 化 DP で評価する。

use std::collections::HashMap;
use std::collections::HashSet;

use crate::glob_tables;

/// `path.posix.matchesGlob` の nocase(`process.platform === 'win32' ||
/// 'darwin'`)。`nocaseMagicOnly` が true 扱いなので、全 literal
/// segment は nocase でも厳密一致のまま。
const NOCASE: bool = cfg!(any(windows, target_os = "macos"));

// ---- UTF-16 code unit 定数 ----

const STAR: u16 = 0x2A; // '*'
const QMARK: u16 = 0x3F; // '?'
const LBRACK: u16 = 0x5B; // '['
const RBRACK: u16 = 0x5D; // ']'
const LBRACE: u16 = 0x7B; // '{'
const RBRACE: u16 = 0x7D; // '}'
const BANG: u16 = 0x21; // '!'
const CARET: u16 = 0x5E; // '^'
const DASH: u16 = 0x2D; // '-'
const DOT: u16 = 0x2E; // '.'
const SLASH: u16 = 0x2F; // '/'
const BSLASH: u16 = 0x5C; // '\'
const PIPE: u16 = 0x7C; // '|'
const LPAREN: u16 = 0x28; // '('
const RPAREN: u16 = 0x29; // ')'
const PLUS: u16 = 0x2B; // '+'
const AT: u16 = 0x40; // '@'

/// brace expansion の内部単位型。UTF-16 code unit より広い `u32` を
/// 使い、brace-expansion の `escOpen`/`escClose` sentinel は Unicode
/// 範囲外の値(≥ 0x110000)とする。以前は PUA(U+E000/U+E001)を
/// sentinel に共用していたため、入力中の literal U+E000/U+E001 が
/// `{`/`}` へ化ける collision があった — Node 側は
/// `'\0OPEN<random>\0'` 形式の衝突不能な sentinel を使う。
const ESC_OPEN: u32 = 0x110000;
const ESC_CLOSE: u32 = 0x110001;
type EUnits = Vec<u32>;

const E_LBRACE: u32 = b'{' as u32;
const E_RBRACE: u32 = b'}' as u32;
const E_COMMA: u32 = b',' as u32;
const E_DOT: u32 = b'.' as u32;
const E_DASH: u32 = b'-' as u32;
const E_ZERO: u32 = b'0' as u32;
const E_DOLLAR: u32 = b'$' as u32;
const E_BSLASH: u32 = b'\\' as u32;

/// `path.posix.matchesGlob` の pattern 上限(UTF-16 code unit 数)。
/// 超過で Node は `TypeError("pattern is too long")` を投げる。
/// path 側には同様の上限は無い。
const MAX_PATTERN_UNITS: usize = 65536;

/// brace expansion の安全上限(minimatch 既定値)。
const MAX_EXPAND: usize = 100_000;
const MAX_EXPAND_LEN: usize = 4_000_000;

type Units = Vec<u16>;

/// `[:name:]` POSIX named class のうち Unicode 属性テーブルを
/// 必要とするもの(`ascii`/`xdigit` は inline range、`graph` は
/// negated class として別扱い)。
#[derive(Clone, Copy)]
enum Named {
    /// `\p{L}\p{Nl}\p{Nd}`
    Alnum,
    /// `\p{L}\p{Nl}`
    Alpha,
    /// `\p{Zs}\t`
    Blank,
    /// `\p{Cc}`
    Cntrl,
    /// `\p{Nd}`
    Digit,
    /// `\p{Ll}`
    Lower,
    /// `\p{C}`(Node の `print` 定義はこれ)
    Print,
    /// `\p{P}`
    Punct,
    /// `\p{Z}\t\r\n\v\f`
    Space,
    /// `\p{Lu}`
    Upper,
    /// `\p{L}\p{Nl}\p{Nd}\p{Pc}`
    Word,
}

fn named_table(n: Named) -> &'static [(u32, u32)] {
    match n {
        Named::Alnum => glob_tables::ALNUM,
        Named::Alpha => glob_tables::ALPHA,
        Named::Blank => glob_tables::BLANK,
        Named::Cntrl => glob_tables::CNTRL,
        Named::Digit => glob_tables::DIGIT,
        Named::Lower => glob_tables::LOWER,
        Named::Print => glob_tables::PRINT,
        Named::Punct => glob_tables::PUNCT,
        Named::Space => glob_tables::SPACE,
        Named::Upper => glob_tables::UPPER,
        Named::Word => glob_tables::WORD,
    }
}

fn in_ranges(t: &[(u32, u32)], cp: u32) -> bool {
    t.binary_search_by(|&(lo, hi)| {
        if cp < lo {
            std::cmp::Ordering::Greater
        } else if cp > hi {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Equal
        }
    })
    .is_ok()
}

fn in_named(n: Named, cp: u32) -> bool {
    in_ranges(named_table(n), cp)
}

/// `\p{Z}` | `\p{C}`(`[:graph:]` の negated class が参照する集合)。
fn in_z_or_c(cp: u32) -> bool {
    in_ranges(glob_tables::GRAPH_NEG, cp)
}

fn is_starstar(p: &[u16]) -> bool {
    p == [STAR, STAR]
}
fn is_dot(p: &[u16]) -> bool {
    p == [DOT]
}
fn is_dotdot(p: &[u16]) -> bool {
    p == [DOT, DOT]
}
fn is_hi(u: u16) -> bool {
    (0xD800..=0xDBFF).contains(&u)
}
fn is_lo(u: u16) -> bool {
    (0xDC00..=0xDFFF).contains(&u)
}

/// `casefold` 相当の 1-char orbit。JS `toLowerCase()`/`toUpperCase()` が
/// 多文字に変換する場合(e.g. 'İ'→'i̇')は orbit に含めない
/// (minimatch の `caseFold` と同じ近似)。
fn fold_orbit(u: u16) -> [Option<u16>; 2] {
    let Some(c) = char::from_u32(u as u32) else {
        return [None; 2];
    };
    let lo = {
        let mut it = c.to_lowercase();
        match (it.next(), it.next()) {
            (Some(x), None) => u16::try_from(x as u32).ok(),
            _ => None,
        }
    };
    let up = {
        let mut it = c.to_uppercase();
        match (it.next(), it.next()) {
            (Some(x), None) => u16::try_from(x as u32).ok(),
            _ => None,
        }
    };
    [lo, up]
}

/// nocase での 1 unit 一致判定。fold orbit + `ß`↔`ẞ`。
fn unit_eq_ci(a: u16, b: u16) -> bool {
    if a == b {
        return true;
    }
    for cand in fold_orbit(a).into_iter().flatten() {
        if cand == b {
            return true;
        }
    }
    // ß(0xDF) は uppercase が "SS" 複数文字のため ẞ(0x1E9E) への orbit を
    // 明示する。
    (a == 0xDF && b == 0x1E9E) || (a == 0x1E9E && b == 0xDF)
}

fn unit_eq(a: u16, b: u16) -> bool {
    a == b || (NOCASE && unit_eq_ci(a, b))
}

// =====================================================================
// 公開エントリ
// =====================================================================

/// `path.posix.matchesGlob` が pattern > 65536 UTF-16 unit に投げる
/// `TypeError("pattern is too long")` 相当。`describeError`/`cap` で
/// `name === "TypeError"` と観測されるよう専用型にする。
#[derive(Debug)]
pub struct PatternTooLongError;

impl std::fmt::Display for PatternTooLongError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("pattern is too long")
    }
}

impl std::error::Error for PatternTooLongError {}

/// `path.posix.matchesGlob(path, pattern)` 相当。
///
/// pattern が 65536 UTF-16 unit を超えると `PatternTooLongError`
/// (`TypeError("pattern is too long")` 相当)を返す。検査は呼出し時点
/// (file 毎の lazy 評価)で行うため、対象 file が 0 件なら発火しない。
/// path の長さは検査しない(Node と同じ)。
pub fn matches_glob(path: &str, pattern: &str) -> Result<bool, crate::ToolError> {
    let pat: Vec<u16> = pattern
        .encode_utf16()
        .map(|u| if u == BSLASH { SLASH } else { u })
        .collect();
    // Node は pattern.length > 65536 で TypeError を投げる
    // (`pattern.length === 0` 判定より先に throw するが、空 pattern は
    // 常に制限内なので順序は不問)。
    if pat.len() > MAX_PATTERN_UNITS {
        return Err(Box::new(PatternTooLongError));
    }
    // `pattern.length === 0` は `path === ''` にのみ一致する。
    if pat.is_empty() {
        return Ok(path.is_empty());
    }
    let file = file_parts(path);
    Ok(expand_braces(&pat).iter().any(|alt| {
        // `**/../p/p` 分割で partset が増えるため、preprocess は
        // partset の集合を受け取り全 alternative を返す。
        let partsets = preprocess(vec![slash_split(alt)]);
        partsets.iter().any(|parts| {
            let parsed: Vec<Seg> = parts.iter().map(|p| parse_segment(p)).collect();
            match_parsed(&file, &parsed)
        })
    }))
}

/// `Minimatch#match`/`matchOne` 相当。globstar を含めば専用経路、
/// なければ segment 列の直接比較。
fn match_parsed(file: &[Units], pat: &[Seg]) -> bool {
    if let Some(firstgs) = pat.iter().position(|s| matches!(s, Seg::Globstar)) {
        match_globstar(file, pat, firstgs)
    } else {
        match_one(file, pat, 0, 0, file.len())
    }
}

// =====================================================================
// slash split / preprocess
// =====================================================================

/// `p.split(/\/+/)` 相当(連続 `/` は 1 区切りに潰れる)。
fn slash_split(p: &[u16]) -> Vec<Units> {
    let mut out: Vec<Units> = Vec::new();
    let mut cur: Units = Vec::new();
    let mut i = 0;
    while i < p.len() {
        if p[i] == SLASH {
            out.push(std::mem::take(&mut cur));
            while i < p.len() && p[i] == SLASH {
                i += 1;
            }
        } else {
            cur.push(p[i]);
            i += 1;
        }
    }
    out.push(cur);
    out
}

/// `levelTwoFileOptimize` 相当(`.''` 除去と `..` 解消を安定まで)。
fn file_parts(path: &str) -> Vec<Units> {
    let u: Vec<u16> = path.encode_utf16().collect();
    let mut parts = slash_split(&u);
    loop {
        let mut changed = false;
        let mut i = 1;
        while i + 1 < parts.len() {
            if parts[i].is_empty() || is_dot(&parts[i]) {
                parts.remove(i);
                changed = true;
            } else {
                i += 1;
            }
        }
        if parts.len() == 2 && is_dot(&parts[0]) && (is_dot(&parts[1]) || parts[1].is_empty()) {
            parts.pop();
            changed = true;
        }
        changed |= resolve_dotdot(&mut parts, false);
        if !changed {
            break;
        }
    }
    if parts.is_empty() {
        parts.push(Vec::new());
    }
    parts
}

/// `parts` 中の `p/..` を可能な範囲で解消する(`p` が `.`/`..`/`**` の
/// 場合は解消しない)。`need_dot` は `firstPhasePreProcess`(pattern 側)
/// 固有の挙動で、`dd===1 && parts[dd+1]==='**'` のとき `.` を残す。
/// `levelTwoFileOptimize`(file 側)にはこの分岐が無い。
fn resolve_dotdot(parts: &mut Vec<Units>, need_dot: bool) -> bool {
    let mut changed = false;
    let mut dd: isize = 0;
    loop {
        let from = (dd + 1).max(0) as usize;
        let Some(d) = (from..parts.len()).find(|&k| is_dotdot(&parts[k])) else {
            break;
        };
        let prev_ok = d > 0 && {
            let p = &parts[d - 1];
            !is_dot(p) && !is_dotdot(p) && !is_starstar(p)
        };
        if prev_ok {
            if need_dot && d == 1 && parts.get(d + 1).is_some_and(|p| is_starstar(p)) {
                parts.splice(d - 1..=d, vec![vec![DOT]]);
            } else {
                parts.drain(d - 1..=d);
            }
            if parts.is_empty() {
                parts.push(Vec::new());
            }
            changed = true;
            dd = d as isize - 2;
        } else {
            dd = d as isize;
        }
    }
    changed
}

/// `firstPhasePreProcess` 相当。`**` run dedup・`**/../p/p` 分割・
/// `.`/空 segment 除去・`..` 解消を安定するまで繰り返す。
/// `secondPhasePreProcess`(partset 同士の merge dedupe)は、merge が
/// 要求する `*`/`**` 整列が `**/../p/p` 分割の生成物では生じ得ないため
/// union 意味論を変えないので省略する。
fn preprocess(mut glob_parts: Vec<Vec<Units>>) -> Vec<Vec<Units>> {
    loop {
        let mut changed = false;
        let mut i = 0;
        while i < glob_parts.len() {
            changed |= phase_one(&mut glob_parts, i);
            i += 1;
        }
        if !changed {
            return glob_parts;
        }
    }
}

fn phase_one(glob_parts: &mut Vec<Vec<Units>>, idx: usize) -> bool {
    let mut changed = false;
    {
        let mut extras: Vec<Vec<Units>> = Vec::new();
        let parts = &mut glob_parts[idx];
        let mut gi = 0;
        while gi < parts.len() {
            if !is_starstar(&parts[gi]) {
                gi += 1;
                continue;
            }
            let gs = gi;
            let mut gss = gs;
            while gss + 1 < parts.len() && is_starstar(&parts[gss + 1]) {
                gss += 1;
            }
            if gss > gs {
                parts.drain(gs + 1..=gss);
                changed = true;
            }
            let p_ok =
                |q: Option<&Units>| q.is_some_and(|q| !q.is_empty() && !is_dot(q) && !is_dotdot(q));
            if parts.get(gs + 1).is_some_and(|q| is_dotdot(q))
                && p_ok(parts.get(gs + 2))
                && p_ok(parts.get(gs + 3))
            {
                parts.remove(gs);
                let mut other = parts.clone();
                other[gs] = vec![STAR, STAR];
                extras.push(other);
                changed = true;
                continue;
            }
            gi += 1;
        }
        glob_parts.extend(extras);
    }
    {
        let parts = &mut glob_parts[idx];
        let mut i = 1;
        while i + 1 < parts.len() {
            if parts[i].is_empty() || is_dot(&parts[i]) {
                parts.remove(i);
                changed = true;
            } else {
                i += 1;
            }
        }
        if parts.len() == 2 && is_dot(&parts[0]) && (is_dot(&parts[1]) || parts[1].is_empty()) {
            parts.pop();
            changed = true;
        }
        changed |= resolve_dotdot(parts, true);
    }
    changed
}

// =====================================================================
// brace expansion(brace-expansion v5 相当)
// =====================================================================
//
// `\` は入力に存在しない(`/` へ変換済み)ため escapeBraces の `\x`
// 系列は到達不能。sentinel が生成されるのは `{}` prefix の literal 化と
// `{a},b}` 型の group 取り込み(`escClose`)のみ。

/// `balanced('{', '}', str)` 相当。Some((open_idx, close_idx))。
fn balanced(s: &[u32]) -> Option<(usize, usize)> {
    let idx_of = |c: u32, from: usize| -> isize {
        s.iter()
            .enumerate()
            .skip(from)
            .find(|&(_, &u)| u == c)
            .map(|(i, _)| i as isize)
            .unwrap_or(-1)
    };
    let mut ai = idx_of(E_LBRACE, 0);
    let mut bi = idx_of(E_RBRACE, ai.max(0) as usize + 1);
    let mut i = ai;
    let mut result: Option<(usize, usize)> = None;
    if ai >= 0 && bi > 0 {
        let mut begs: Vec<isize> = Vec::new();
        let mut left = s.len() as isize;
        let mut right = -1isize;
        while i >= 0 && result.is_none() {
            if i == ai {
                begs.push(i);
                ai = idx_of(E_LBRACE, i as usize + 1);
            } else if begs.len() == 1 {
                result = Some((begs.pop().unwrap() as usize, bi as usize));
            } else {
                let beg = begs.pop().unwrap();
                if beg < left {
                    left = beg;
                    right = bi;
                }
                bi = idx_of(E_RBRACE, i as usize + 1);
            }
            i = if ai < bi && ai >= 0 { ai } else { bi };
        }
        if !begs.is_empty() && right >= 0 {
            result = Some((left as usize, right as usize));
        }
    }
    result
}

/// `expand(str)` 相当。トップレベルで `{}` prefix は literal 化する。
/// 入出力は UTF-16 unit、内部表現は `EUnits`(sentinel 含む `u32`)。
fn expand_braces(pat: &[u16]) -> Vec<Units> {
    if pat.is_empty() {
        return Vec::new();
    }
    let mut s: EUnits = pat.iter().map(|&u| u32::from(u)).collect();
    if pat.starts_with(&[LBRACE, RBRACE]) {
        // `{}` prefix の literal 化(`'\\{\\}' + rest` 相当 — `\` は
        // 入力に無いので sentinel 置換で表現する)。
        s[0] = ESC_OPEN;
        s[1] = ESC_CLOSE;
    }
    expand_inner(s, true)
        .iter()
        .map(|item| unescape_braces(item))
        .collect()
}

/// `unescapeBraces` 相当。sentinel を `{`/`}` literal へ戻し、
/// 戻り値は sentinel を含まない UTF-16 unit 列となる。
fn unescape_braces(s: &[u32]) -> Units {
    s.iter()
        .map(|&u| match u {
            ESC_OPEN => LBRACE,
            ESC_CLOSE => RBRACE,
            // expansion が生成するのは入力 UTF-16 unit / sentinel /
            // ASCII(数字・符号・`e+`)のみ。
            u => u16::try_from(u).expect("brace expansion only emits UTF-16 units"),
        })
        .collect()
}

/// `combine` 相当。`acc[a] + pre + values[v]` を全組合せ生成。
fn combine(acc: &[EUnits], pre: &[u32], values: &[EUnits], drop_empties: bool) -> Vec<EUnits> {
    let mut out: Vec<EUnits> = Vec::new();
    let mut length = 0usize;
    for a in acc {
        for v in values {
            if out.len() >= MAX_EXPAND {
                return out;
            }
            let mut e = a.clone();
            e.extend_from_slice(pre);
            e.extend_from_slice(v);
            if drop_empties && e.is_empty() {
                continue;
            }
            if length + e.len() > MAX_EXPAND_LEN {
                return out;
            }
            length += e.len();
            out.push(e);
        }
    }
    out
}

/// `expand_` 相当。
fn expand_inner(mut str: EUnits, mut is_top: bool) -> Vec<EUnits> {
    let mut acc: Vec<EUnits> = vec![Vec::new()];
    let mut drop_empties = false;
    let mut first_group = true;
    loop {
        let Some((b0, b1)) = balanced(&str) else {
            // brace なし: 残りは literal。
            return combine(&acc, &str, &[Vec::new()], drop_empties);
        };
        let pre: EUnits = str[..b0].to_vec();
        let body: EUnits = str[b0 + 1..b1].to_vec();
        let post: EUnits = str[b1 + 1..].to_vec();
        // `$` で終わる pre の `{...}` は literal(`a${b,c}` 等)。
        if pre.last() == Some(&E_DOLLAR) {
            let mut lit = pre.clone();
            lit.push(E_LBRACE);
            lit.extend_from_slice(&body);
            lit.push(E_RBRACE);
            acc = combine(&acc, &lit, &[Vec::new()], drop_empties && post.is_empty());
            first_group = false;
            if post.is_empty() {
                break;
            }
            str = post;
            continue;
        }
        let is_seq = is_num_seq(&body) || is_alpha_seq(&body);
        let is_options = body.contains(&E_COMMA);
        if !is_seq && !is_options {
            // `{a},b}` 型: post に `,`(非連続)+`}` があれば group を取り込む。
            if post_merges_group(&post) {
                let mut s = pre;
                s.push(E_LBRACE);
                s.extend_from_slice(&body);
                s.push(ESC_CLOSE);
                s.extend_from_slice(&post);
                str = s;
                is_top = true;
                continue;
            }
            // 展開しない brace: 残り全体が literal。
            let mut lit = pre;
            lit.push(E_LBRACE);
            lit.extend_from_slice(&body);
            lit.push(E_RBRACE);
            lit.extend_from_slice(&post);
            return combine(&acc, &lit, &[Vec::new()], drop_empties);
        }
        if first_group {
            drop_empties = is_top && !is_seq;
            first_group = false;
        }
        let values: Vec<EUnits> = if is_seq {
            expand_sequence(&body, is_alpha_seq(&body))
        } else {
            let mut n = parse_comma_parts(&body);
            if n.len() == 1 {
                // `x{{a,b}}y` → `x{a}y`/`x{b}y`: 単一部分を再帰展開して
                // brace で再包装。
                n = expand_inner(n.remove(0), false)
                    .into_iter()
                    .map(|mut v| {
                        let mut e = vec![E_LBRACE];
                        e.append(&mut v);
                        e.push(E_RBRACE);
                        e
                    })
                    .collect();
                if n.len() == 1 {
                    let mut p = pre.clone();
                    p.extend_from_slice(&n[0]);
                    acc = combine(&acc, &p, &[Vec::new()], drop_empties && post.is_empty());
                    if post.is_empty() {
                        break;
                    }
                    str = post;
                    continue;
                }
            }
            // 空になる value は drop 対象として数えないため、ここで
            // filter しながら収集する。
            let mut drops = drop_empties && post.is_empty() && pre.is_empty();
            if drops {
                drops = acc.iter().all(|a| a.is_empty());
            }
            let mut vals: Vec<EUnits> = Vec::new();
            let mut vals_len = 0usize;
            'outer: for part in n {
                for mut v in expand_inner(part, false) {
                    if drops && v.is_empty() {
                        continue;
                    }
                    if vals.len() >= MAX_EXPAND || vals_len + v.len() > MAX_EXPAND_LEN {
                        break 'outer;
                    }
                    vals_len += v.len();
                    vals.push(std::mem::take(&mut v));
                }
            }
            vals
        };
        acc = combine(&acc, &pre, &values, drop_empties && post.is_empty());
        if post.is_empty() {
            break;
        }
        str = post;
    }
    acc
}

/// `parseCommaParts` 相当(`,` split、ただし nested `{}` 内は分割しない)。
fn parse_comma_parts(s: &[u32]) -> Vec<EUnits> {
    if s.is_empty() {
        return vec![Vec::new()];
    }
    let Some((b0, b1)) = balanced(s) else {
        return s.split(|&u| u == E_COMMA).map(<[u32]>::to_vec).collect();
    };
    let mut parts: Vec<EUnits> = s[..b0]
        .split(|&u| u == E_COMMA)
        .map(<[u32]>::to_vec)
        .collect();
    {
        let last = parts.last_mut().unwrap();
        last.push(E_LBRACE);
        last.extend_from_slice(&s[b0 + 1..b1]);
        last.push(E_RBRACE);
    }
    let post = &s[b1 + 1..];
    let mut post_parts = parse_comma_parts(post);
    if !post.is_empty() {
        let first = post_parts.remove(0);
        parts.last_mut().unwrap().extend_from_slice(&first);
        parts.extend(post_parts);
    }
    parts
}

/// `post.match(/,(?!,).*\}/)`: `,`(直後が `,` でない)の後に `}` が来るか。
fn post_merges_group(post: &[u32]) -> bool {
    for (i, &u) in post.iter().enumerate() {
        if u == E_COMMA && post.get(i + 1) != Some(&E_COMMA) && post[i + 1..].contains(&E_RBRACE) {
            return true;
        }
    }
    false
}

/// `^-?\d+\.\.-?\d+(\.\.-?\d+)?$`
fn is_num_seq(body: &[u32]) -> bool {
    let parts = split_dots(body);
    if !(2..=3).contains(&parts.len()) {
        return false;
    }
    parts[..2].iter().all(|p| is_signed_digits(p))
        && (parts.len() == 2 || is_signed_digits(parts[2]))
}

/// `^[a-zA-Z]\.\.[a-zA-Z](\.\.-?\d+)?$`
fn is_alpha_seq(body: &[u32]) -> bool {
    let parts = split_dots(body);
    if !(2..=3).contains(&parts.len()) {
        return false;
    }
    let is_a =
        |p: &[u32]| p.len() == 1 && p[0] < 0x80 && (p[0] as u8 as char).is_ascii_alphabetic();
    is_a(parts[0]) && is_a(parts[1]) && (parts.len() == 2 || is_signed_digits(parts[2]))
}

/// `body.split(/\.\./)`
fn split_dots(body: &[u32]) -> Vec<&[u32]> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i + 1 < body.len() {
        if body[i] == E_DOT && body[i + 1] == E_DOT {
            out.push(&body[start..i]);
            i += 2;
            start = i;
        } else {
            i += 1;
        }
    }
    out.push(&body[start..]);
    out
}

/// u32 unit の ASCII digit 判定('0'..'9')。
fn is_digit_u(u: u32) -> bool {
    (E_ZERO..=0x39).contains(&u)
}

/// `-?\d+` のみ。
fn is_signed_digits(p: &[u32]) -> bool {
    let digits = if p.first() == Some(&E_DASH) {
        &p[1..]
    } else {
        p
    };
    !digits.is_empty() && digits.iter().all(|&u| is_digit_u(u))
}

/// `numeric(str)`:`!isNaN ? parseInt(str,10) : charCodeAt(0)`。
/// 呼び出し側は `-?\d+` または 1 alpha char のみを渡す。
///
/// brace-expansion は値を JS number で扱うため f64 とする。i64 fold
/// だと i64 range を超えた時点で debug panic / release wrap するが、
/// JS では 2^53 以降で丸まるだけで overflow しない
/// (`{9223372036854775808..}` → `9223372036854776000`)。逐次
/// `a*10+d` fold は parseInt の correctly-rounded parse と末尾 bit が
/// ずれ得るため、`str::parse::<f64>`(両者とも correctly rounded)を
/// 使う。巨大入力は `inf` になり `String()` 側の `"Infinity"` が
/// 生成される(JS の parseInt も Infinity を返す)。
fn numeric(p: &[u32]) -> f64 {
    if is_signed_digits(p) {
        let s: String = p.iter().map(|&u| char::from_u32(u).unwrap()).collect();
        s.parse::<f64>().unwrap_or(f64::NAN)
    } else {
        f64::from(p[0])
    }
}

/// `String(i)` 相当。brace-expansion が sequence 値を生成するときの
/// JS `Number.prototype.toString` — `cheaper-core` の canonical
/// formatter(1e21/1e-6 exponential 境界・`-0`→`0` 含む)を使う。
/// 非 finite は `JSON.stringify` ではなく `String()` 側の `"Infinity"`/
/// `"NaN"` を返す(数値 parse が Infinity を生成し得るため防御)。
fn js_num_to_string(x: f64) -> String {
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x.is_infinite() {
        return if x < 0.0 {
            "-Infinity".to_string()
        } else {
            "Infinity".to_string()
        };
    }
    cheaper_core::json::number::format_number(x)
}

/// `/^-?0\d/`
fn is_padded(p: &[u32]) -> bool {
    let digits = if p.first() == Some(&E_DASH) {
        &p[1..]
    } else {
        p
    };
    digits.len() >= 2 && digits[0] == E_ZERO && is_digit_u(digits[1])
}

/// `expandSequence` 相当。`x`/`y`/`incr`/`i` は全て JS number 意味論の
/// `f64`(i64 overflow 回避 + `String(i)` format parity)。
fn expand_sequence(body: &[u32], is_alpha: bool) -> Vec<EUnits> {
    let n = split_dots(body);
    let (Some(&n0), Some(&n1)) = (n.first(), n.get(1)) else {
        return Vec::new();
    };
    let x = numeric(n0);
    let y = numeric(n1);
    let width = n0.len().max(n1.len());
    let mut incr: f64 = if n.len() == 3 {
        numeric(n[2]).abs().max(1.0)
    } else {
        1.0
    };
    let reverse = y < x;
    if reverse {
        incr = -incr;
    }
    let pad = n.iter().any(|p| is_padded(p));
    let mut out: Vec<EUnits> = Vec::new();
    let mut length = 0usize;
    let mut i = x;
    while (if reverse { i >= y } else { i <= y }) && out.len() < MAX_EXPAND {
        let c: EUnits = if is_alpha {
            // `String.fromCharCode(i)` 相当(ToUint16)。alpha range 中の
            // `\` は JS 側で '' に潰れる。
            let code = i.rem_euclid(65536.0) as u32;
            if code == E_BSLASH {
                Vec::new()
            } else {
                vec![code]
            }
        } else {
            // `String(i)` — '-' 込みで stringify してから pad する
            // (`'-' + z + c.slice(1)` / `z + c`)。
            let mut c = js_num_to_string(i);
            if pad {
                let need = width.saturating_sub(c.len());
                if need > 0 {
                    let z = "0".repeat(need);
                    c = if i < 0.0 {
                        format!("-{z}{}", &c[1..])
                    } else {
                        format!("{z}{c}")
                    };
                }
            }
            c.bytes().map(u32::from).collect()
        };
        if length + c.len() > MAX_EXPAND_LEN {
            break;
        }
        length += c.len();
        out.push(c);
        i += incr;
    }
    out
}

/// extglob 内の raw node(中間表現)。
#[derive(Clone)]
enum AstNode {
    /// token 化前の string chunk(非空のみ)。
    Str(Units),
    /// nested extglob。
    Ext(ExtAst),
}

#[derive(Clone)]
struct ExtAst {
    ty: u16,
    alts: Vec<Vec<AstNode>>,
    /// `#emptyExt` 相当(閉じ `)` 直前の alternative が空)。
    empty_ext: bool,
    /// 元 segment 上の `[ty_pos, close後)` 範囲(literal 化時に使う)。
    raw_start: usize,
    raw_end: usize,
    /// `#negs` 用の construction 順 id(`!` のみ有効、他は `u32::MAX`)。
    /// `#fillNegs` は construction 逆順で処理するため id で整列する。
    neg_id: u32,
}

const MAX_EXT_DEPTH: usize = 2;

fn is_ext_ty(c: u16) -> bool {
    matches!(c, BANG | QMARK | PLUS | STAR | AT)
}

/// `adoptionAnyMap` 相当(深さ 2 超の nested extglob を再帰対象にするか)。
fn can_adopt(parent_ty: u16, child_ty: u16) -> bool {
    let children: &[u16] = match parent_ty {
        BANG => &[QMARK, AT],
        QMARK => &[QMARK, AT],
        AT => &[QMARK, AT],
        STAR => &[STAR, PLUS, QMARK, AT],
        PLUS => &[PLUS, AT, QMARK, STAR],
        _ => &[],
    };
    children.contains(&child_ty)
}

/// `#parseAST` の root(type null)走査。`in_brace` で `[`〜`]` 内の
/// `|`/`)`/`(` を extglob 境界と誤認しないように追跡する。
fn parse_seg_ast(g: &[u16]) -> Vec<AstNode> {
    let mut parts: Vec<AstNode> = Vec::new();
    let mut acc: Units = Vec::new();
    let mut in_brace = false;
    let mut brace_start = 0usize;
    let mut brace_neg = false;
    // `#negs` 相当: `!` extglob の construction 順 id。
    let mut neg_ctr = 0u32;
    let mut i = 0usize;
    while i < g.len() {
        let c = g[i];
        i += 1;
        if in_brace {
            if i == brace_start + 1 && (c == CARET || c == BANG) {
                brace_neg = true;
            } else if c == RBRACK && !(i == brace_start + 2 && brace_neg) {
                in_brace = false;
            }
            acc.push(c);
            continue;
        }
        if c == LBRACK {
            in_brace = true;
            brace_start = i;
            brace_neg = false;
            acc.push(c);
            continue;
        }
        // root では `extDepth(0) <= maxDepth(2)` が常に真。
        if is_ext_ty(c) && g.get(i) == Some(&LPAREN) {
            if let Some((ext, next)) = parse_ext_ast(g, c, i, 0, &mut neg_ctr) {
                if !acc.is_empty() {
                    parts.push(AstNode::Str(std::mem::take(&mut acc)));
                }
                parts.push(AstNode::Ext(ext));
                i = next;
                continue;
            }
        }
        acc.push(c);
    }
    if !acc.is_empty() {
        parts.push(AstNode::Str(acc));
    }
    parts
}

/// `open`(`(` の index)から extglob 内部を parse する。
/// unterminated の場合は `None`(呼び出し側で type char を通常 token
/// として処理する)。
fn parse_ext_ast(
    g: &[u16],
    ty: u16,
    open: usize,
    depth: usize,
    neg_ctr: &mut u32,
) -> Option<(ExtAst, usize)> {
    // `!` node は `!(` を読んだ時点で `#negs` に積まれる(JS では
    // `new AST` が `(` 直後に走る)ので、nested より先に id を採番する。
    let neg_id = if ty == BANG {
        let id = *neg_ctr;
        *neg_ctr += 1;
        id
    } else {
        u32::MAX
    };
    let mut alts: Vec<Vec<AstNode>> = vec![Vec::new()];
    let mut acc: Units = Vec::new();
    let mut in_brace = false;
    let mut brace_start = 0usize;
    let mut brace_neg = false;
    let mut i = open + 1;
    while i < g.len() {
        let c = g[i];
        i += 1;
        if in_brace {
            if i == brace_start + 1 && (c == CARET || c == BANG) {
                brace_neg = true;
            } else if c == RBRACK && !(i == brace_start + 2 && brace_neg) {
                in_brace = false;
            }
            acc.push(c);
            continue;
        }
        match c {
            LBRACK => {
                in_brace = true;
                brace_start = i;
                brace_neg = false;
                acc.push(c);
            }
            PIPE => {
                if !acc.is_empty() {
                    alts.last_mut()
                        .unwrap()
                        .push(AstNode::Str(std::mem::take(&mut acc)));
                }
                alts.push(Vec::new());
            }
            RPAREN => {
                // `acc === ''` のとき `#emptyExt`(末尾 alternative 空)。
                let empty_ext = acc.is_empty();
                if !acc.is_empty() {
                    alts.last_mut()
                        .unwrap()
                        .push(AstNode::Str(std::mem::take(&mut acc)));
                }
                return Some((
                    ExtAst {
                        ty,
                        alts,
                        empty_ext,
                        raw_start: open - 1,
                        raw_end: i,
                        neg_id,
                    },
                    i,
                ));
            }
            c if is_ext_ty(c) && g.get(i) == Some(&LPAREN) => {
                let depth_add = if can_adopt(ty, c) { 0 } else { 1 };
                // `extDepth <= maxDepth || canAdoptType(c)`:
                // adopt 可能な nested extglob は depth 制限を超えても再帰する。
                if depth <= MAX_EXT_DEPTH || can_adopt(ty, c) {
                    if let Some((ext, next)) = parse_ext_ast(g, c, i, depth + depth_add, neg_ctr) {
                        if !acc.is_empty() {
                            alts.last_mut()
                                .unwrap()
                                .push(AstNode::Str(std::mem::take(&mut acc)));
                        }
                        alts.last_mut().unwrap().push(AstNode::Ext(ext));
                        i = next;
                        continue;
                    }
                }
                // 再帰しない/失敗 → type char は通常文字として蓄積。
                acc.push(c);
            }
            _ => acc.push(c),
        }
    }
    None
}

// ---- `#flatten` 相当 ----

/// `adoptionMap`: parent type → adopt 可能な child extglob type。
fn adopt_map(ty: u16) -> &'static [u16] {
    match ty {
        BANG => &[AT],
        QMARK => &[QMARK, AT],
        AT => &[AT],
        STAR => &[STAR, PLUS, QMARK, AT],
        PLUS => &[PLUS, AT],
        _ => &[],
    }
}

/// `adoptionWithSpaceMap`: 空 alternative 追加つきで adopt 可能な組。
fn adopt_space_map(ty: u16) -> &'static [u16] {
    match ty {
        BANG => &[QMARK],
        AT => &[QMARK],
        PLUS => &[QMARK, STAR],
        _ => &[],
    }
}

/// `usurpMap`: parent type + child type → usurp 後の type。
fn usurp_type(ty: u16, gty: u16) -> Option<u16> {
    match ty {
        BANG => (gty == BANG).then_some(AT),
        QMARK => matches!(gty, STAR | PLUS).then_some(STAR),
        AT => is_ext_ty(gty).then_some(gty),
        PLUS => matches!(gty, QMARK | STAR).then_some(STAR),
        _ => None,
    }
}

/// type-null node 列の flatten(各 extglob を再帰 flatten)。
fn flatten_nodes(nodes: &mut [AstNode]) {
    for n in nodes.iter_mut() {
        if let AstNode::Ext(e) = n {
            flatten_ext(e);
        }
    }
}

/// extglob の `#flatten`: adopt / adoptWithSpace / usurp を fixpoint
/// まで行う(最大 10 pass)。
fn flatten_ext(ext: &mut ExtAst) {
    let mut iterations = 0;
    loop {
        let mut done = true;
        let mut i = 0;
        while i < ext.alts.len() {
            // まず child alternative(type null)の内部を flatten。
            flatten_nodes(&mut ext.alts[i]);
            let gty = match ext.alts[i].as_slice() {
                [AstNode::Ext(gc)] => Some(gc.ty),
                _ => None,
            };
            if let Some(gty) = gty {
                if adopt_map(ext.ty).contains(&gty) {
                    // alt i を grandchild の alternatives で置き換える。
                    let Some(AstNode::Ext(gc)) = ext.alts.get_mut(i).map(|a| a.remove(0)) else {
                        unreachable!()
                    };
                    ext.alts.splice(i..=i, gc.alts);
                    done = false;
                } else if adopt_space_map(ext.ty).contains(&gty) {
                    // grandchild に空 alternative を追加してから adopt。
                    let Some(AstNode::Ext(mut gc)) = ext.alts.get_mut(i).map(|a| a.remove(0))
                    else {
                        unreachable!()
                    };
                    gc.alts.push(Vec::new());
                    ext.alts.splice(i..=i, gc.alts);
                    done = false;
                } else if ext.alts.len() == 1 {
                    if let Some(nt) = usurp_type(ext.ty, gty) {
                        let Some(AstNode::Ext(gc)) = ext.alts.get_mut(i).map(|a| a.remove(0))
                        else {
                            unreachable!()
                        };
                        ext.alts = gc.alts;
                        ext.ty = nt;
                        ext.empty_ext = false;
                        done = false;
                    }
                }
            }
            i += 1;
        }
        iterations += 1;
        if done || iterations >= 10 {
            break;
        }
    }
}

// ---- `#fillNegs` 相当 ----

/// `#fillNegs`: 各 `!` extglob の alternative に「祖先 partlist で
/// その後にある全 part」を copyIn する。`#negs` は construction 順に
/// 積まれ LIFO で処理されるため、neg_id の降順に 1 個ずつ処理する
/// (copyIn された part の clone は `#filledNegs` 済みで再登録されず、
/// こちらでも copy は処理済みの `!` の snapshot を持つ)。
fn fill_negs(nodes: &mut [AstNode]) {
    while let Some(tid) = max_neg_id(nodes) {
        let mut chain: Vec<Vec<AstNode>> = Vec::new();
        let _ = fill_walk(nodes, tid, &mut chain);
    }
}

/// 未処理(neg_id != u32::MAX)の `!` node の最大 neg_id。
fn max_neg_id(nodes: &[AstNode]) -> Option<u32> {
    let mut m = None;
    for n in nodes {
        if let AstNode::Ext(e) = n {
            if e.ty == BANG && e.neg_id != u32::MAX {
                m = Some(m.map_or(e.neg_id, |x: u32| x.max(e.neg_id)));
            }
            for a in &e.alts {
                if let Some(x) = max_neg_id(a) {
                    m = Some(m.map_or(x, |y: u32| y.max(x)));
                }
            }
        }
    }
    m
}

/// neg_id == `tid` の `!` node を深さ優先で探し、見つけたら
/// (1) その後続兄弟 + `chain`(外側から内側に向かう順…ではなく
/// 内側→外側の順で積まれる)を alternative に append、
/// (2) unwind 時に祖先 partlist の「ext 以降の parts」を chain に積む。
/// chain は内側→外側の順になる(JS の `#fillNegs` の walk-up と同じ)。
fn fill_walk(nodes: &mut [AstNode], tid: u32, chain: &mut Vec<Vec<AstNode>>) -> bool {
    for j in 0..nodes.len() {
        let AstNode::Ext(ext) = &nodes[j] else {
            continue;
        };
        if ext.ty == BANG && ext.neg_id == tid {
            // innermost の suffix = この `!` 以降の兄弟 part。
            let mut suffix: Vec<AstNode> = nodes[j + 1..].to_vec();
            for s in chain.iter() {
                suffix.extend(s.iter().cloned());
            }
            let AstNode::Ext(ext) = &mut nodes[j] else {
                unreachable!()
            };
            for alt in &mut ext.alts {
                alt.extend(suffix.iter().cloned());
            }
            ext.neg_id = u32::MAX;
            return true;
        }
        let mut found = false;
        if let AstNode::Ext(ext) = &mut nodes[j] {
            for alt in &mut ext.alts {
                if fill_walk(alt, tid, chain) {
                    found = true;
                    break;
                }
            }
        }
        if found {
            // 祖先 partlist でこの ext 以降の parts を copyIn 対象に追加。
            chain.push(nodes[j + 1..].to_vec());
            return true;
        }
    }
    false
}

enum Seg {
    /// magic を持たない literal(完全一致で評価)。
    Lit(Units),
    /// wildcard/class/extglob を含む magic segment。
    Magic(MagicSeg),
    /// `**` globstar。
    Globstar,
}

struct MagicSeg {
    seq: Seq,
    uflag: bool,
}

/// segment または extglob alternative の token 列。
struct Seq {
    elems: Vec<Elem>,
    /// `isStart()` 相当(直前が `!` extglob のみでも true)。
    is_start: bool,
    /// memo 用の一意 id。
    id: u32,
}

enum Elem {
    /// literal 1 unit。
    Lit(u16),
    /// `*` run の縮約結果。`nonempty` は `[^/]+?`(starNoEmpty)。
    Star { nonempty: bool },
    /// `?`(`[^/]`)。
    Qmark,
    /// `[...]` class。
    Class(ClassPat),
    /// `$.` poison。この seq は一致しない。
    Never,
    /// extglob `?() *() +() @() !()`。
    Ext(ExtPat),
}

struct ExtPat {
    ty: u16,
    alts: Vec<Seq>,
    is_start: bool,
    is_end: bool,
    empty_ext: bool,
    /// literal 化時に使う raw 文字列。
    raw: Units,
}

/// `[...]` class 1 個分。
struct ClassPat {
    negate: bool,
    /// 正方向 member 候補。
    pos: Vec<ClassItem>,
    /// `[:graph:]`(negated class `[^\p{Z}\p{C}]` 相当)。
    graph_neg: bool,
    needs_u: bool,
}

enum ClassItem {
    Unit(u16),
    Range(u16, u16),
    Named(Named),
}

struct Builder<'a> {
    g: &'a [u16],
    next_id: u32,
}

impl<'a> Builder<'a> {
    fn new_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// string chunk を token 化して `elems` に積む。
    /// `no_empty` は `#parseGlob` の `noEmpty` 相当: chunk 全体が `*`
    /// のみなら `[^/]+?`(starNoEmpty)を使う。
    fn push_str(&mut self, elems: &mut Vec<Elem>, s: &[u16], no_empty: bool, uflag: &mut bool) {
        let all_stars = no_empty && s.iter().all(|&u| u == STAR);
        let mut i = 0;
        while i < s.len() {
            match s[i] {
                STAR => {
                    while i < s.len() && s[i] == STAR {
                        i += 1;
                    }
                    elems.push(Elem::Star {
                        nonempty: all_stars,
                    });
                }
                QMARK => {
                    elems.push(Elem::Qmark);
                    i += 1;
                }
                LBRACK => match parse_class(s, i) {
                    ClassOut::Unterminated => {
                        elems.push(Elem::Lit(LBRACK));
                        i += 1;
                    }
                    ClassOut::Poison => {
                        // `$.` は残り全体を消費する poison。
                        elems.push(Elem::Never);
                        i = s.len();
                    }
                    ClassOut::Literal { end, unit } => {
                        elems.push(Elem::Lit(unit));
                        i = end;
                    }
                    ClassOut::Class(pat, end) => {
                        *uflag |= pat.needs_u;
                        elems.push(Elem::Class(pat));
                        i = end;
                    }
                },
                c => {
                    elems.push(Elem::Lit(c));
                    i += 1;
                }
            }
        }
    }

    /// AstNode 列 → `Seq`。`is_start`/`is_end` は minimatch AST の
    /// `isStart()`/`isEnd()` に対応する。戻り値の bool は nested
    /// extglob alternative を含む u flag の OR(segment 単位の regex flag)。
    fn build_seq(&mut self, nodes: &[AstNode], is_start: bool, is_end: bool) -> (Seq, bool) {
        // ext 自身の is_start/is_end は part 列上の位置で決まる。
        // AstNode 列は非空 string と ext のみなので、index がそのまま
        // parentIndex 相当になる。
        let node_is_start = |j: usize| -> bool {
            is_start
                && nodes[..j]
                    .iter()
                    .all(|n| matches!(n, AstNode::Ext(e) if e.ty == BANG))
        };
        let node_is_end = |j: usize| -> bool { is_end && j + 1 == nodes.len() };
        // `noEmpty`: seq が isStart && isEnd で全 part が string のとき、
        // `*` のみからなる string part は `[^/]+?`(starNoEmpty)になる。
        // `!(a|b)*` の `*` が neg alternative に copyIn されたとき
        // `a[^/]+?` になるのはこの規則による(alternative は全 string)。
        let no_empty = is_start && is_end && nodes.iter().all(|n| matches!(n, AstNode::Str(_)));

        let mut elems: Vec<Elem> = Vec::new();
        let mut uflag = false;
        for (j, node) in nodes.iter().enumerate() {
            match node {
                AstNode::Str(s) => self.push_str(&mut elems, s, no_empty, &mut uflag),
                AstNode::Ext(ext) => {
                    let (e, sub_uflag) = self.build_ext(ext, node_is_start(j), node_is_end(j));
                    uflag |= sub_uflag;
                    elems.push(e);
                }
            }
        }
        // 空 extglob の literal 化: `isStart && isEnd && !body &&
        // type !== '!'`。`!(…)` は emptyExt でも starNoEmpty 側へ行く。
        if let [Elem::Ext(ext)] = elems.as_mut_slice() {
            if ext.is_start
                && ext.is_end
                && ext.ty != BANG
                && ext.alts.iter().all(|a| a.elems.is_empty())
            {
                elems = ext.raw.iter().map(|&u| Elem::Lit(u)).collect();
                uflag = false;
            }
        }
        (
            Seq {
                elems,
                is_start,
                id: self.new_id(),
            },
            uflag,
        )
    }

    fn build_ext(&mut self, ext: &ExtAst, ext_is_start: bool, ext_is_end: bool) -> (Elem, bool) {
        let mut alts = Vec::with_capacity(ext.alts.len());
        let mut uflag = false;
        for alt_nodes in &ext.alts {
            // alternative の isStart: JS では alt node は `|` 検出時に
            // 生成され、その時点で親 extglob の `#parts` はまだ空なので
            // 全 alternative の #parentIndex は 0 になる。つまり親 ext が
            // isStart なら全 alternative が isStart。
            let alt_start = ext_is_start;
            // alternative の isEnd: 親が `!` なら常に true、
            // それ以外は ext 自身の isEnd を引き継ぐ。
            let alt_end = ext.ty == BANG || ext_is_end;
            let (seq, alt_uflag) = self.build_seq(alt_nodes, alt_start, alt_end);
            uflag |= alt_uflag;
            alts.push(seq);
        }
        (
            Elem::Ext(ExtPat {
                ty: ext.ty,
                alts,
                is_start: ext_is_start,
                is_end: ext_is_end,
                empty_ext: ext.empty_ext,
                raw: self.g[ext.raw_start..ext.raw_end].to_vec(),
            }),
            uflag,
        )
    }
}

/// segment 1 個分の parse(`Minimatch#parse` の glob→`set` 変換相当)。
fn parse_segment(g: &[u16]) -> Seg {
    if is_starstar(g) {
        return Seg::Globstar;
    }
    let mut ast = parse_seg_ast(g);
    // `#flatten`: nested extglob の adopt/usurp を fixpoint まで適用。
    flatten_nodes(&mut ast);
    // `#fillNegs`: `!` extglob の alternative に後続 part を copyIn。
    fill_negs(&mut ast);
    let mut builder = Builder { g, next_id: 0 };
    let (seq, uflag) = builder.build_seq(&ast, true, true);
    if seq.elems.iter().all(|e| matches!(e, Elem::Lit(_))) {
        // magic を持たない segment は unescape 済み literal として
        // 完全一致で評価する(`[x]`→`x` は Lit 化済み)。
        return Seg::Lit(
            seq.elems
                .iter()
                .map(|e| match e {
                    Elem::Lit(u) => *u,
                    _ => unreachable!(),
                })
                .collect(),
        );
    }
    Seg::Magic(MagicSeg { seq, uflag })
}

// =====================================================================
// parseClass 相当(brace-expressions.js)
// =====================================================================

enum ClassOut {
    /// `[` が閉じない → literal `[`。
    Unterminated,
    /// `$.` poison。この seq は一致しない。
    Poison,
    /// 1 文字 literal `[x]`(magic 扱いにならない)。
    Literal { end: usize, unit: u16 },
    /// 正常な class。
    Class(ClassPat, usize),
}

/// `[:name:]` 形式の POSIX class token を認識する。
/// `(消費長, item候補 or graph_neg, needs_u)` を返す。
fn named_class_at(g: &[u16], i: usize) -> Option<(usize, NamedItem)> {
    const TOKENS: &[(&str, NamedItem)] = &[
        ("[:alnum:]", NamedItem::Named(Named::Alnum)),
        ("[:alpha:]", NamedItem::Named(Named::Alpha)),
        ("[:blank:]", NamedItem::Named(Named::Blank)),
        ("[:cntrl:]", NamedItem::Named(Named::Cntrl)),
        ("[:digit:]", NamedItem::Named(Named::Digit)),
        ("[:lower:]", NamedItem::Named(Named::Lower)),
        ("[:print:]", NamedItem::Named(Named::Print)),
        ("[:punct:]", NamedItem::Named(Named::Punct)),
        ("[:space:]", NamedItem::Named(Named::Space)),
        ("[:upper:]", NamedItem::Named(Named::Upper)),
        ("[:word:]", NamedItem::Named(Named::Word)),
        ("[:ascii:]", NamedItem::Ascii),
        ("[:xdigit:]", NamedItem::Xdigit),
        ("[:graph:]", NamedItem::Graph),
    ];
    for (tok, item) in TOKENS {
        let len = tok.len();
        if g.len() - i >= len && tok.bytes().enumerate().all(|(k, b)| g[i + k] == b as u16) {
            return Some((len, *item));
        }
    }
    None
}

#[derive(Clone, Copy)]
enum NamedItem {
    Named(Named),
    /// `[:ascii:]`(`\x00-\x7f`)。
    Ascii,
    /// `[:xdigit:]`(`A-Fa-f0-9`)。
    Xdigit,
    /// `[:graph:]`(negated class `[^\p{Z}\p{C}]` 相当)。
    Graph,
}

/// `parseClass` 相当。`g[pos] == '['` 前提。
/// `\` は入力に存在しない(path.posix 変換済み)ため escape 分岐は要らない。
fn parse_class(g: &[u16], pos: usize) -> ClassOut {
    let mut ranges: Vec<ClassItem> = Vec::new();
    let mut graph_neg = false;
    let mut i = pos + 1;
    let mut saw_start = false;
    let mut uflag = false;
    let mut negate = false;
    let mut end_pos: Option<usize> = None;
    let mut range_start: Option<u16> = None;
    while i < g.len() {
        let c = g[i];
        if (c == BANG || c == CARET) && i == pos + 1 {
            negate = true;
            i += 1;
            continue;
        }
        if c == RBRACK && saw_start {
            end_pos = Some(i + 1);
            break;
        }
        saw_start = true;
        if c == LBRACK {
            if let Some((len, item)) = named_class_at(g, i) {
                // `[a-[:alpha:]]` 型は invalid(`$.` poison)。
                if range_start.is_some() {
                    return ClassOut::Poison;
                }
                i += len;
                match item {
                    NamedItem::Named(n) => {
                        ranges.push(ClassItem::Named(n));
                        uflag = true;
                    }
                    NamedItem::Ascii => ranges.push(ClassItem::Range(0x00, 0x7F)),
                    NamedItem::Xdigit => {
                        ranges.push(ClassItem::Range(0x30, 0x39)); // 0-9
                        ranges.push(ClassItem::Range(0x41, 0x46)); // A-F
                        ranges.push(ClassItem::Range(0x61, 0x66)); // a-f
                    }
                    NamedItem::Graph => {
                        graph_neg = true;
                        uflag = true;
                    }
                }
                continue;
            }
        }
        if let Some(start) = range_start.take() {
            // range 確定。逆順は捨てる。
            if c > start {
                ranges.push(ClassItem::Range(start, c));
            } else if c == start {
                ranges.push(ClassItem::Unit(c));
            }
            i += 1;
            continue;
        }
        // `c-]` は `c` と `-` の literal 相当。
        if g.get(i + 1) == Some(&DASH) && g.get(i + 2) == Some(&RBRACK) {
            ranges.push(ClassItem::Unit(c));
            ranges.push(ClassItem::Unit(DASH));
            i += 2;
            continue;
        }
        if g.get(i + 1) == Some(&DASH) {
            range_start = Some(c);
            i += 2;
            continue;
        }
        ranges.push(ClassItem::Unit(c));
        i += 1;
    }
    // 閉じない class(`endPos < i` 相当)→ literal `[`。
    let Some(end) = end_pos else {
        return ClassOut::Unterminated;
    };
    // range/negate が一切ない → `$.` poison(残り全体を消費)。
    if ranges.is_empty() && !graph_neg {
        return ClassOut::Poison;
    }
    // 1 文字の非 negate class → literal 化(`[x]` → `x`)。
    if !graph_neg && !negate && ranges.len() == 1 {
        if let ClassItem::Unit(u) = ranges[0] {
            return ClassOut::Literal { end, unit: u };
        }
    }
    ClassOut::Class(
        ClassPat {
            negate,
            pos: ranges,
            graph_neg,
            needs_u: uflag,
        },
        end,
    )
}

// =====================================================================
// matcher(memo 化 DP)
// =====================================================================

/// continuation frame。`seq` の `ti` 以降の評価、あるいは extglob の
/// 繰り返し継続を表す。
#[derive(Clone, Copy)]
struct Frame {
    /// 対象 seq の id。
    seq: u32,
    /// 再開位置(iter frame の場合は extglob elem の index)。
    ti: usize,
    /// この continuation が評価されるときの allow_dot。
    ad: bool,
    /// `Some(alt_ad)` のとき、pop は「`seq[ti]` の extglob の追加反復」
    /// を意味し、alternative は `alt_ad` で評価する。
    iter_alts_ad: Option<bool>,
    /// 次の continuation(arena index)。
    next: Option<usize>,
}

/// seq のポインタを id から resolve するための registry。
/// parse 時に積んだ `seqs` arena への index を id と一致させる。
struct Ctx<'a> {
    f: &'a [u16],
    uflag: bool,
    /// Seq id → `&Seq`。
    seqs: Vec<&'a Seq>,
    /// continuation arena。
    conts: Vec<Frame>,
    /// memo: (seq_id, ti, si, cont_id, ad) → bool。
    memo: HashMap<(u32, usize, usize, usize, bool), bool>,
    /// 計算中の再帰ループ検出。
    active: HashSet<(u32, usize, usize, usize, bool)>,
}

/// `file` segment 文字列中、`si` が code point 境界かどうか。
/// (`/u` regex で `[^/]` が surrogate pair を割らないのと同じ制約)
fn is_cp_boundary(f: &[u16], si: usize) -> bool {
    !(si > 0 && is_hi(f[si - 1]) && is_lo(f[si]))
}

impl<'a> Ctx<'a> {
    fn new(f: &'a [u16]) -> Self {
        Ctx {
            f,
            uflag: false,
            seqs: Vec::new(),
            conts: Vec::new(),
            memo: HashMap::new(),
            active: HashSet::new(),
        }
    }

    fn push_cont(&mut self, frame: Frame) -> usize {
        self.conts.push(frame);
        self.conts.len() - 1
    }
}

/// `Seq` を先に registry へ積み直す(再帰的に)。`parse_segment` の結果
/// `Seq` 群は一意な `id` を持つので、id → `&Seq` を張る。
fn register_seqs<'a>(seq: &'a Seq, seqs: &mut Vec<&'a Seq>) {
    if seq.id as usize >= seqs.len() {
        seqs.resize(seq.id as usize + 1, seq);
    }
    seqs[seq.id as usize] = seq;
    for e in &seq.elems {
        if let Elem::Ext(ext) = e {
            for a in &ext.alts {
                register_seqs(a, seqs);
            }
        }
    }
}

/// seq 先頭の guard(`addPatternStart` 系)。`si` は現在位置。
/// `ad` は allowDot(親 extglob の dot 伝播)。
/// `Some(false)` 相当の false を返したらこの seq は一致しない。
fn start_guard_ok(seq: &Seq, f: &[u16], si: usize, ad: bool) -> bool {
    if !seq.is_start {
        return true;
    }
    let Some(e0) = seq.elems.first() else {
        return true;
    };
    // parts[0] が string 由来でなければ guard は掛からない。
    if matches!(e0, Elem::Ext(_)) {
        return true;
    }
    // `.`/`..` だけの seq は例外(justDots)。
    if seq.elems.len() == 1 || seq.elems.len() == 2 {
        let only_dots = seq.elems.iter().all(|e| matches!(e, Elem::Lit(DOT)));
        if only_dots && !seq.elems.is_empty() && seq.elems.len() <= 2 {
            // `parts.length === 1 && justDots.has(parts[0])`
            // — ただし `..` は parts が 2 つのとき string '..' のみ。
            // elems==[Lit('.')] or [Lit('.'),Lit('.')] を許可する。
            if seq.elems.len() == 1 || seq.elems.len() == 2 {
                return true;
            }
        }
    }
    // `addPatternStart = ['[', '.']`: 生成 regex が `[`(class/star/qmark)
    // で始まるか。literal `.` は `\.` に escape されるため該当しない。
    // 例外: `graph` と正 member の混在 class は `(...|...)` union に
    // なるため src は `(` で始まり aps に入らない。
    let aps_lead = |e: &Elem| -> bool {
        match e {
            Elem::Star { .. } | Elem::Qmark => true,
            Elem::Class(c) => !(c.graph_neg && !c.pos.is_empty()),
            _ => false,
        }
    };
    // needNoTrav: `(dot && aps(src[0])) || src.startsWith('\.') &&
    // aps(src[2]) || src.startsWith('\..') && aps(src[4])`
    let lit_dot = |e: Option<&Elem>| matches!(e, Some(Elem::Lit(DOT)));
    let trav = (ad && aps_lead(e0))
        || (lit_dot(seq.elems.first()) && aps_lead(seq.elems.get(1).unwrap_or(&Elem::Never)))
        || (lit_dot(seq.elems.first())
            && lit_dot(seq.elems.get(1))
            && seq.elems.get(2).is_some_and(aps_lead));
    if trav {
        // `(?!(?:^|/)\.\.?(?:$|/))` — si==0 のときのみ活性。
        if si == 0 && (f == [DOT] || f == [DOT, DOT]) {
            return false;
        }
    } else {
        // needNoDot: `!dot && !allowDot && aps(src[0])`。
        if !ad && aps_lead(e0) && f.get(si) == Some(&DOT) {
            return false;
        }
    }
    true
}

/// seq_match:`seq.elems[ti..]` + `cont` が `f[si..]` に full match するか。
fn seq_match(
    ctx: &mut Ctx<'_>,
    seq_id: u32,
    ti: usize,
    si: usize,
    cont: Option<usize>,
    ad: bool,
) -> bool {
    let key = (seq_id, ti, si, cont.unwrap_or(usize::MAX), ad);
    if let Some(&r) = ctx.memo.get(&key) {
        return r;
    }
    if !ctx.active.insert(key) {
        // 再帰ループ(空 match する `*`/`+` 反復等)→ false。
        return false;
    }
    let seq = ctx.seqs[seq_id as usize];
    let r = seq_match_inner(ctx, seq, ti, si, cont, ad);
    ctx.active.remove(&key);
    ctx.memo.insert(key, r);
    r
}

#[allow(clippy::too_many_arguments)]
fn seq_match_inner(
    ctx: &mut Ctx<'_>,
    seq: &Seq,
    ti: usize,
    si: usize,
    cont: Option<usize>,
    ad: bool,
) -> bool {
    let elems = &seq.elems;
    if ti == elems.len() {
        // seq 消尽 → continuation へ。
        return cont_match(ctx, cont, si);
    }
    // seq 先頭(ti==0)での guard(is_start かつ先頭 elem が string 系)。
    if ti == 0 && !start_guard_ok(seq, ctx.f, si, ad) {
        return false;
    }
    let e = &elems[ti];
    match e {
        Elem::Lit(u) => {
            si < ctx.f.len()
                && unit_eq(ctx.f[si], *u)
                && seq_match(ctx, seq.id, ti + 1, si + 1, cont, ad)
        }
        Elem::Qmark => {
            if si >= ctx.f.len() {
                return false;
            }
            let step =
                if ctx.uflag && is_hi(ctx.f[si]) && si + 1 < ctx.f.len() && is_lo(ctx.f[si + 1]) {
                    2
                } else {
                    1
                };
            seq_match(ctx, seq.id, ti + 1, si + step, cont, ad)
        }
        Elem::Star { nonempty } => {
            // `[^/]*?`(または `+?`)。`k` は消費 unit 数。
            let start = si + usize::from(*nonempty);
            let mut k = start;
            while k <= ctx.f.len() {
                if (!ctx.uflag || is_cp_boundary(ctx.f, k))
                    && seq_match(ctx, seq.id, ti + 1, k, cont, ad)
                {
                    return true;
                }
                k += 1;
            }
            false
        }
        Elem::Never => false,
        Elem::Class(c) => {
            let Some((hit, step)) = class_match(c, ctx.f, si, ctx.uflag) else {
                return false;
            };
            hit && seq_match(ctx, seq.id, ti + 1, si + step, cont, ad)
        }
        Elem::Ext(ext) => ext_match(ctx, seq, ti, si, cont, ad, ext),
    }
}

/// continuation pop。
fn cont_match(ctx: &mut Ctx<'_>, cont: Option<usize>, si: usize) -> bool {
    let Some(c) = cont else {
        // continuation 終端 = segment 末尾。
        return si == ctx.f.len();
    };
    let fr = ctx.conts[c];
    if let Some(alt_ad) = fr.iter_alts_ad {
        // extglob 反復: 止めて `seq[ti+1..]` を評価するか、もう 1 反復
        // (alternative を `alt_ad` で評価して同一 frame に戻る)か。
        if seq_match(ctx, fr.seq, fr.ti + 1, si, fr.next, fr.ad) {
            return true;
        }
        let seq = ctx.seqs[fr.seq as usize];
        let Elem::Ext(ext) = &seq.elems[fr.ti] else {
            return false;
        };
        let alts = filtered_alts(ext);
        return alts
            .iter()
            .any(|a| seq_match(ctx, a.id, 0, si, Some(c), alt_ad));
    }
    seq_match(ctx, fr.seq, fr.ti, si, fr.next, fr.ad)
}

/// extglob alternative の filter(`isStart && isEnd` のとき空 re を除く)。
/// `!` の alternative は常に `(?:$|\/)` が付くため filter しない。
fn filtered_alts(ext: &ExtPat) -> Vec<&Seq> {
    ext.alts
        .iter()
        .filter(|a| ext.ty == BANG || !(ext.is_start && ext.is_end) || !a.elems.is_empty())
        .collect()
}

/// extglob element の評価。
fn ext_match(
    ctx: &mut Ctx<'_>,
    seq: &Seq,
    ti: usize,
    si: usize,
    cont: Option<usize>,
    ad: bool,
    ext: &ExtPat,
) -> bool {
    // `!` の starNoDot guard: `isStart && !dot && !allowDot`。
    if ext.ty == BANG && ext.is_start && !ad && ctx.f.get(si) == Some(&DOT) {
        return false;
    }
    match ext.ty {
        BANG => {
            if ext.empty_ext {
                // `!()`/`!(a|)` → `(startNoDot)?[^/]+?`。
                let mut k = si + 1;
                while k <= ctx.f.len() {
                    if (!ctx.uflag || is_cp_boundary(ctx.f, k))
                        && seq_match(ctx, seq.id, ti + 1, k, cont, ad)
                    {
                        return true;
                    }
                    k += 1;
                }
                return false;
            }
            // guard: `#fillNegs` 済みの alternative(copyIn で `!` 以降の
            // part が連結済み、末尾は `(?:$|\/)` 相当)が残り全体に
            // full match するならこの `!` は失敗。
            for a in &ext.alts {
                if seq_match(ctx, a.id, 0, si, None, ad) {
                    return false;
                }
            }
            // `[^/]*?`: 任意の prefix を消費して rest へ。
            let mut k = si;
            while k <= ctx.f.len() {
                if (!ctx.uflag || is_cp_boundary(ctx.f, k))
                    && seq_match(ctx, seq.id, ti + 1, k, cont, ad)
                {
                    return true;
                }
                k += 1;
            }
            false
        }
        QMARK => {
            // `(?:alts)?` — 空 or 1 alternative。
            if seq_match(ctx, seq.id, ti + 1, si, cont, ad) {
                return true;
            }
            let rest = ctx.push_cont(Frame {
                seq: seq.id,
                ti: ti + 1,
                ad,
                iter_alts_ad: None,
                next: cont,
            });
            filtered_alts(ext)
                .iter()
                .any(|a| seq_match(ctx, a.id, 0, si, Some(rest), ad))
        }
        AT => {
            let rest = ctx.push_cont(Frame {
                seq: seq.id,
                ti: ti + 1,
                ad,
                iter_alts_ad: None,
                next: cont,
            });
            filtered_alts(ext)
                .iter()
                .any(|a| seq_match(ctx, a.id, 0, si, Some(rest), ad))
        }
        STAR | PLUS => {
            // `(?:bodyND)(?:bodyD)*?` 相当。is_start && !ad のとき
            // 最初の反復は noDot 評価、以降は dot 許可。
            let split = ext.is_start && !ad;
            let alt_ad_first = if split { false } else { ad };
            let alt_ad_iter = if split { true } else { ad };
            // 反復継続 frame(`ti` の ext を繰り返す)。
            let iter = ctx.push_cont(Frame {
                seq: seq.id,
                ti,
                ad,
                iter_alts_ad: Some(alt_ad_iter),
                next: cont,
            });
            // 0 反復(`*` のみ)。
            if ext.ty == STAR && seq_match(ctx, seq.id, ti + 1, si, cont, ad) {
                return true;
            }
            // 1 回以上の反復。
            filtered_alts(ext)
                .iter()
                .any(|a| seq_match(ctx, a.id, 0, si, Some(iter), alt_ad_first))
        }
        _ => false,
    }
}

// =====================================================================
// class match / segment match
// =====================================================================

/// `[...]` class が `f[si]` の 1 code point (u-mode) または 1 unit
/// (非 u-mode) に一致するか。一致したら消費 unit 数を返す。
fn class_match(c: &ClassPat, f: &[u16], si: usize, uflag: bool) -> Option<(bool, usize)> {
    let &u = f.get(si)?;
    if !uflag {
        let hit = class_hit16(c, u);
        return Some((hit, 1));
    }
    // u-mode: code point 単位で評価。
    let (cp, step) = if is_hi(u) && si + 1 < f.len() && is_lo(f[si + 1]) {
        (
            0x10000 + (((u as u32) - 0xD800) << 10) + (f[si + 1] as u32 - 0xDC00),
            2,
        )
    } else {
        (u as u32, 1)
    };
    Some((class_hit_u(c, cp), step))
}

fn class_hit16(c: &ClassPat, u: u16) -> bool {
    let pos = c.pos.iter().any(|it| match it {
        ClassItem::Unit(x) => unit_eq(u, *x),
        ClassItem::Range(lo, hi) => {
            (u >= *lo && u <= *hi)
                || (NOCASE
                    && fold_orbit(u)
                        .into_iter()
                        .flatten()
                        .any(|v| v >= *lo && v <= *hi))
        }
        ClassItem::Named(_) => false,
    });
    let mut hit = if c.negate { !pos } else { pos };
    if c.graph_neg {
        hit |= if c.negate {
            is_zc(u as u32)
        } else {
            !is_zc(u as u32)
        };
    }
    hit
}

fn class_hit_u(c: &ClassPat, cp: u32) -> bool {
    let pos = c.pos.iter().any(|it| match it {
        ClassItem::Unit(x) => cp == *x as u32 || (NOCASE && cp_eq_ci(cp, *x as u32)),
        ClassItem::Range(lo, hi) => {
            cp >= *lo as u32 && cp <= *hi as u32
                || (NOCASE
                    && cp_fold_orbit(cp)
                        .into_iter()
                        .flatten()
                        .any(|v| v >= *lo as u32 && v <= *hi as u32))
        }
        ClassItem::Named(n) => {
            in_named(*n, cp)
                || (NOCASE
                    && cp_fold_orbit(cp)
                        .into_iter()
                        .flatten()
                        .any(|v| in_named(*n, v)))
        }
    });
    let mut hit = if c.negate { !pos } else { pos };
    if c.graph_neg {
        hit |= if c.negate { is_zc(cp) } else { !is_zc(cp) };
    }
    hit
}

/// cp 版 fold orbit。
fn cp_fold_orbit(cp: u32) -> [Option<u32>; 3] {
    let Some(c) = char::from_u32(cp) else {
        return [None; 3];
    };
    let lo = {
        let mut it = c.to_lowercase();
        match (it.next(), it.next()) {
            (Some(x), None) => Some(x as u32),
            _ => None,
        }
    };
    let up = {
        let mut it = c.to_uppercase();
        match (it.next(), it.next()) {
            (Some(x), None) => Some(x as u32),
            _ => None,
        }
    };
    [Some(cp), lo, up]
}

fn cp_eq_ci(a: u32, b: u32) -> bool {
    cp_fold_orbit(a).into_iter().flatten().any(|v| v == b)
}

/// `\p{Z}` | `\p{C}` の判定(graph_neg 用)。
fn is_zc(cp: u32) -> bool {
    in_z_or_c(cp)
}

// =====================================================================
// matchOne / matchGlobstar
// =====================================================================

/// `matchOne` 相当。`pat[pi..]` が `file[fi..fl)` に一致するか。
/// (`fl` は slice 済み相当の file 長。globstar 不在前提)
fn match_one(file: &[Units], pat: &[Seg], fi: usize, pi: usize, fl: usize) -> bool {
    let mut ctx = Ctx::new(&[]);
    match_one_ctx(&mut ctx, file, pat, fi, pi, fl)
}

/// file part 1 個と pattern seg 1 個の一致。
fn seg_match_one<'a>(ctx: &mut Ctx<'a>, f: &'a [u16], p: &'a Seg) -> bool {
    match p {
        Seg::Lit(lit) => f == lit.as_slice(),
        Seg::Magic(m) => {
            ctx.f = f;
            ctx.uflag = m.uflag;
            register_seqs(&m.seq, &mut ctx.seqs);
            ctx.memo.clear();
            ctx.active.clear();
            ctx.conts.clear();
            seq_match(ctx, m.seq.id, 0, 0, None, false)
        }
        Seg::Globstar => false,
    }
}

fn match_one_ctx<'a>(
    ctx: &mut Ctx<'a>,
    file: &'a [Units],
    pat: &'a [Seg],
    mut fi: usize,
    mut pi: usize,
    fl: usize,
) -> bool {
    while fi < fl && pi < pat.len() {
        if matches!(pat[pi], Seg::Globstar) {
            return false;
        }
        if !seg_match_one(ctx, &file[fi], &pat[pi]) {
            return false;
        }
        fi += 1;
        pi += 1;
    }
    if fi == fl && pi == pat.len() {
        // 同時に尽きた = 完全一致。
        return true;
    }
    if fi == fl {
        // file 側が尽きた(partial は常に false)。
        return false;
    }
    // pattern が尽きたが file が残る: file の末尾が空 part(末尾 `/`)
    // 1 つだけのときのみ許容する(`a/*` が `a/b/` に一致)。
    fi == fl - 1 && file[fi].is_empty()
}

/// `matchGlobstar` 相当。`pat` は `**` を 1 個以上含む。
fn match_globstar<'a>(file: &'a [Units], pat: &'a [Seg], firstgs: usize) -> bool {
    let lastgs = pat
        .iter()
        .rposition(|s| matches!(s, Seg::Globstar))
        .unwrap();
    let head = &pat[..firstgs];
    // `**` 単独(または隣接 globstar)では body は空。JS の slice は
    // start > end を clamp するので同様にする。
    let body = if firstgs < lastgs {
        &pat[firstgs + 1..lastgs]
    } else {
        &[]
    };
    let tail = &pat[lastgs + 1..];
    let mut ctx = Ctx::new(&[]);
    let mut fi = 0usize;
    // head は `file[0..head.len]` に一致しなければならない。
    if !head.is_empty() {
        let fl = head.len().min(file.len());
        if !match_one_ctx(&mut ctx, file, head, 0, 0, fl) {
            return false;
        }
        fi += head.len();
    }
    // tail は file 末尾に一致しなければならない。
    let mut file_tail_match = 0usize;
    if !tail.is_empty() {
        if tail.len() + fi > file.len() {
            return false;
        }
        let mut tail_start = file.len() - tail.len();
        if match_one_ctx(&mut ctx, file, tail, tail_start, 0, file.len()) {
            file_tail_match = tail.len();
        } else {
            // `a/**/*` が `a/b/` に一致する affordance:
            // file 末尾が '' かつ余裕があるなら '' を除いて再試行。
            if !file.last().is_some_and(|p| p.is_empty()) || fi + tail.len() == file.len() {
                return false;
            }
            tail_start -= 1;
            if !match_one_ctx(&mut ctx, file, tail, tail_start, 0, file.len()) {
                return false;
            }
            file_tail_match = tail.len() + 1;
        }
    }
    if body.is_empty() {
        // globstar が 1 つだけ: 中間の part は消費されるが
        // `.`/`..`/dot 始まりは不可。また少なくとも 1 part を
        // 消費するか tail が一致していなければならない。
        let mut saw_some = file_tail_match > 0;
        for p in &file[fi..file.len() - file_tail_match] {
            saw_some = true;
            if is_dot(p) || is_dotdot(p) || p.first() == Some(&DOT) {
                return false;
            }
        }
        return saw_some;
    }
    // body を `**` で section に分割し、各 section の最遅開始位置
    // (`after`)を算出する。`sums[k]` = section k より前の非 `**` part 数。
    let file_length = file.len() - file_tail_match;
    let mut sections: Vec<&[Seg]> = Vec::new();
    let mut sums: Vec<usize> = vec![0];
    let mut start = 0usize;
    for (i, b) in body.iter().enumerate() {
        if matches!(b, Seg::Globstar) {
            let s = &body[start..i];
            sums.push(sums.last().unwrap() + s.len());
            sections.push(s);
            start = i + 1;
        }
    }
    sections.push(&body[start..]);
    let n = sections.len();
    // `after[j] = fileLength - (sums[n-1-j] + len_j)`(負になり得る)。
    let after: Vec<isize> = (0..n)
        .map(|j| file_length as isize - (sums[n - 1 - j] + sections[j].len()) as isize)
        .collect();
    // 再帰的に各 section の位置を探索。
    matches!(
        globstar_sections(
            &mut ctx,
            file,
            &sections,
            &after,
            fi,
            0,
            0,
            file_tail_match > 0
        ),
        GsResult::Yes
    )
}

/// `#matchGlobStarBodySections` の tri-state 結果。
enum GsResult {
    /// 一致。
    Yes,
    /// 不一致(この alignment では不可。dot part 検出など)。
    /// JS の `false` に相当(親は別位置で walk を継続する)。
    No,
    /// 走査し尽くした。JS の `null` に相当(探索全体を打ち切る)。
    Null,
}

/// `file[i]` が globstar で消費できない part(`.`/`..`/dot 始まり)か。
fn bad_gs_part(p: &[u16]) -> bool {
    is_dot(p) || is_dotdot(p) || p.first() == Some(&DOT)
}

/// `#matchGlobStarBodySections` 相当の再帰走査。
#[allow(clippy::too_many_arguments)]
fn globstar_sections<'a>(
    ctx: &mut Ctx<'a>,
    file: &'a [Units],
    sections: &[&'a [Seg]],
    after: &[isize],
    mut fi: usize,
    bi: usize,
    depth: usize,
    saw_tail: bool,
) -> GsResult {
    const MAX_GLOBSTAR_RECURSION: usize = 200;
    let Some(&body) = sections.get(bi) else {
        // 全 section を消費: 残りの file part に dot 系が無いか確認
        // (tail に一致した領域も含めて file 末尾まで)。
        // part が 1 つでも残れば sawTail = true。
        let mut saw = saw_tail;
        for p in &file[fi.min(file.len())..] {
            saw = true;
            if bad_gs_part(p) {
                return GsResult::No;
            }
        }
        return if saw { GsResult::Yes } else { GsResult::No };
    };
    while (fi as isize) <= after[bi] {
        // `file[fi..fi+body.len]` に body が一致するか。
        let fl = (fi + body.len()).min(file.len());
        if match_one_ctx(ctx, file, body, fi, 0, fl) && depth < MAX_GLOBSTAR_RECURSION {
            let sub = globstar_sections(
                ctx,
                file,
                sections,
                after,
                fi + body.len(),
                bi + 1,
                depth + 1,
                saw_tail,
            );
            if !matches!(sub, GsResult::No) {
                return sub;
            }
        }
        if fi >= file.len() || bad_gs_part(&file[fi]) {
            return GsResult::No;
        }
        fi += 1;
    }
    GsResult::Null
}

#[cfg(test)]
mod tests {
    use super::matches_glob;

    /// Node `path.posix.matchesGlob` との差分 probe で見つかった
    /// representative な case(詳細な差分集合は tests/fixtures/glob.json)。
    #[test]
    fn matches_glob_smoke() {
        for (p, g, want) in [
            // extglob 反復(continuation frame)
            ("aa", "*(a|b)", true),
            ("abcabc", "*(a|b|c)", true),
            ("x", "+(a|b)", false),
            // `!` extglob: fillNegs で後続 part が alternative に copyIn。
            // `!(a|b)*` の `*` は alternative 内で starNoEmpty になる。
            ("a", "!(a|b)*", true),
            ("b", "!(a|b)*", true),
            ("a", "!(a|b)", false),
            ("c", "!(a|b)", true),
            ("xay", "x!(a)y", false),
            ("xby", "x!(a)y", true),
            // globstar
            ("a/x/y/z/b", "a/**/x/**/b", true),
            ("a/.h/x", "a/**/x", false),
            ("a/b/", "a/*", true),
            // dot guard
            (".x", "?x", false),
            (".x", "[.]x", true),
            // path-side `..` / `.` 最適化
            ("a/x/../b", "a/b", true),
            ("a/./b", "a/b", true),
            // pattern 側 `\` → `/` 正規化(path 側はしない)
            ("a/b", "a\\b", true),
            ("a*b", "a\\*b", false),
            // UTF-16: `?` は 1 code unit(非 u flag)では surrogate pair を割る。
            ("💩", "?", false),
            ("あ", "?", true),
            // literal PUA(U+E000/U+E001)は内部 sentinel と衝突しない。
            ("a\u{E000}b", "a\u{E000}b", true),
            ("a{b", "a\u{E000}b", false),
            ("a\u{E000}b", "a{b", false),
            ("a\u{E001}b", "a\u{E001}b", true),
            // `{}` literal 化 sentinel の先頭経路。
            ("{}x", "{}x", true),
            // `{a},b}` group 取り込み sentinel 経路(Node 実測:
            // "a}" / "b" が生成され、"a,b}" や literal "{a},b}" は
            // 不一致)。
            ("a}", "{a},b}", true),
            ("b", "{a},b}", true),
            ("a,b}", "{a},b}", false),
            ("{a},b}", "{a},b}", false),
            // 巨大数値 brace range は JS number(f64)意味論で展開する。
            // i64 overflow は発生せず、`String(i)` 丸め結果が生成される。
            (
                "9223372036854776000",
                "{9223372036854775808..9223372036854775809}",
                true,
            ),
            (
                "9223372036854775808",
                "{9223372036854775808..9223372036854775809}",
                false,
            ),
            (
                "-9223372036854775808",
                "{9223372036854775808..9223372036854775809}",
                false,
            ),
            // 1e21 超の端点は exponential(`"1e+21"`)で生成される。
            (
                "1e+21",
                "{1000000000000000000000..1000000000000000000002}",
                true,
            ),
            (
                "1000000000000000000000",
                "{1000000000000000000000..1000000000000000000002}",
                false,
            ),
            // reverse / step / padding の従来意味論を維持する。
            ("5", "{1..9..4}", true),
            ("9", "{1..9..4}", true),
            ("b", "{a..z..4}", false),
            ("e", "{a..z..4}", true),
            ("05", "{01..09..4}", true),
            ("-2", "{0..-5..2}", true),
            ("-4", "{0..-5..2}", true),
            // 非整数端点は sequence ではなく literal。
            ("{1.5..5}", "{1.5..5}", true),
            ("x", "{1.5..5}", false),
        ] {
            assert_eq!(matches_glob(p, g).unwrap(), want, "p={p:?} g={g:?}");
        }
    }

    /// pattern > 65536 UTF-16 unit は `TypeError("pattern is too long")`
    /// 相当の error(Node `path.posix.matchesGlob` と同じ)。65536 ちょうどは
    /// 合法、path 側の長さは制限されない。
    #[test]
    fn overlong_pattern_errors() {
        use super::PatternTooLongError;
        let ok = "a".repeat(65536);
        assert!(matches_glob("x", &ok).is_ok());
        let long = "a".repeat(65537);
        let err = matches_glob("x", &long).unwrap_err();
        assert!(err.downcast_ref::<PatternTooLongError>().is_some());
        assert_eq!(err.to_string(), "pattern is too long");
        // astral char は 2 unit: 65536/2 + 1 = over-limit。
        let astral = "💩".repeat(32769);
        assert!(matches_glob("x", &astral).is_err());
        // path の長さは検査されない(pattern 側のみ上限)。
        let long_path = "b".repeat(70000);
        assert!(matches_glob(&long_path, "*").unwrap());
        // 同じ文字列を pattern に使うと上限で error になる対照。
        assert!(matches_glob(&long_path, &long_path).is_err());
    }
}
