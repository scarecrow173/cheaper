use std::fs as stdfs;
use std::path::Path;

use cheaper_core::workspace::deny_paths::create_path_denylist;

use super::*;
use crate::limits::GREP_PATTERN_MAX_CHARS;
use crate::tools::grep_eval::test_support::test_spawn;

fn tmpdir(prefix: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("{prefix}{}", fastrand::u64(..)));
    stdfs::create_dir_all(&dir).expect("tmpdir");
    dir
}

fn root_of(dir: &Path) -> String {
    cheaper_core::workspace::paths::realpath(dir.to_str().unwrap()).unwrap()
}

fn opts<'a>(root: &'a str, o: GrepToolOptions<'a>) -> GrepToolOptions<'a> {
    GrepToolOptions {
        root,
        symlink_policy: SymlinkPolicy::Deny,
        eval_spawn: Some(test_spawn()),
        ..o
    }
}

fn default_options<'a>() -> GrepToolOptions<'a> {
    GrepToolOptions {
        root: "",
        symlink_policy: SymlinkPolicy::Deny,
        denylist: None,
        observe_access: None,
        eval_per_file_ms: None,
        eval_total_ms: None,
        scan_deadline_ms: None,
        chunk_bytes: None,
        max_line_bytes: None,
        batch_max_lines: None,
        batch_max_bytes: None,
        output_max_bytes: None,
        output_line_max_chars: None,
        skip_footer_max_entries: None,
        skip_path_max_chars: None,
        skip_footer_max_bytes: None,
        eval_spawn: None,
        eval_reply_max_bytes: None,
        now: None,
    }
}

fn tool(root: &str) -> GrepTool<'_> {
    create_grep_tool(opts(root, default_options())).expect("tool")
}

fn run(t: &GrepTool<'_>, params: GrepParams<'_>) -> Result<String, ToolError> {
    execute_grep(t, &params, None)
}

fn p(pattern: &str) -> GrepParams<'_> {
    GrepParams {
        pattern,
        ..Default::default()
    }
}

// ---- parity ----

#[test]
fn finds_matches_as_display_line_text() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join("a.txt"), "foo\nbar\nfoo bar\n").unwrap();
    let out = run(&tool(&root), p("foo")).unwrap();
    assert_eq!(out, "a.txt:1: foo\na.txt:3: foo bar");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn honors_ignore_case() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join("a.txt"), "Foo\nfOO\n").unwrap();
    let out = run(
        &tool(&root),
        GrepParams {
            pattern: "foo",
            ignore_case: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out, "a.txt:1: Foo\na.txt:2: fOO");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn filters_by_glob() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join("a.txt"), "hit\n").unwrap();
    stdfs::write(dir.join("a.md"), "hit\n").unwrap();
    let out = run(
        &tool(&root),
        GrepParams {
            pattern: "hit",
            glob: Some("*.txt"),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out, "a.txt:1: hit");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn glob_star_does_not_cross_directories() {
    // `path.posix.matchesGlob("sub/x.txt", "*.txt")` === false
    // (`*` は `/` をまたがない)。`sub/*.txt` は一致する。
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::create_dir_all(dir.join("sub")).unwrap();
    stdfs::write(dir.join("sub").join("x.txt"), "hit\n").unwrap();
    stdfs::write(dir.join("a.txt"), "hit\n").unwrap();
    let out = run(
        &tool(&root),
        GrepParams {
            pattern: "hit",
            glob: Some("*.txt"),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out, "a.txt:1: hit");
    let out = run(
        &tool(&root),
        GrepParams {
            pattern: "hit",
            glob: Some("sub/*.txt"),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out, "sub/x.txt:1: hit");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn searches_subdirectory_via_path() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::create_dir_all(dir.join("sub")).unwrap();
    stdfs::write(dir.join("sub").join("s.txt"), "needle\n").unwrap();
    stdfs::write(dir.join("top.txt"), "needle\n").unwrap();
    let out = run(
        &tool(&root),
        GrepParams {
            pattern: "needle",
            path: Some("sub"),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out, "sub/s.txt:1: needle");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn caps_results_at_max_results() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    let lines: Vec<String> = (0..10).map(|i| format!("m{i}")).collect();
    stdfs::write(dir.join("many.txt"), format!("{}\n", lines.join("\n"))).unwrap();
    let out = run(
        &tool(&root),
        GrepParams {
            pattern: "m\\d",
            max_results: Some(3),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        out,
        "many.txt:1: m0\nmany.txt:2: m1\nmany.txt:3: m2\n[truncated: more matches]"
    );
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn emits_context_lines() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join("c.txt"), "l1\nl2\nhit\nl4\nl5\n").unwrap();
    let out = run(
        &tool(&root),
        GrepParams {
            pattern: "hit",
            context: Some(1),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out, "c.txt-2- l2\nc.txt:3: hit\nc.txt-4- l4");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn emits_separator_between_disjoint_context_blocks() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    let body: Vec<String> = (0..12)
        .map(|i| {
            if i == 2 || i == 9 {
                "hit".to_string()
            } else {
                format!("l{i}")
            }
        })
        .collect();
    stdfs::write(dir.join("sep.txt"), format!("{}\n", body.join("\n"))).unwrap();
    let out = run(
        &tool(&root),
        GrepParams {
            pattern: "hit",
            context: Some(1),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        out,
        "sep.txt-2- l1\nsep.txt:3: hit\nsep.txt-4- l3\n--\nsep.txt-9- l8\nsep.txt:10: hit\nsep.txt-11- l10"
    );
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn keeps_cr_verbatim_for_crlf() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join("crlf.txt"), "a\r\nhit\r\n").unwrap();
    let out = run(&tool(&root), p("hit")).unwrap();
    assert_eq!(out, "crlf.txt:2: hit\r");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn skips_node_modules() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::create_dir_all(dir.join("node_modules")).unwrap();
    stdfs::write(dir.join("node_modules").join("n.txt"), "hit\n").unwrap();
    stdfs::write(dir.join("t.txt"), "hit\n").unwrap();
    let out = run(&tool(&root), p("hit")).unwrap();
    assert_eq!(out, "t.txt:1: hit");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn matches_caret_dollar_on_line1_of_empty_file() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join("empty.txt"), "").unwrap();
    let out = run(&tool(&root), p("^$")).unwrap();
    assert_eq!(out, "empty.txt:1: ");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn matches_caret_dollar_on_trailing_empty_line() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join("a.txt"), "a\n").unwrap();
    let out = run(&tool(&root), p("^$")).unwrap();
    assert_eq!(out, "a.txt:2: ");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn no_trailing_empty_line_without_final_newline() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join("a.txt"), "a").unwrap();
    let out = run(&tool(&root), p("^$")).unwrap();
    assert_eq!(out, "(no matches)");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn no_matches_marker() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join("a.txt"), "zzz\n").unwrap();
    let out = run(&tool(&root), p("qqq")).unwrap();
    assert_eq!(out, "(no matches)");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rejects_invalid_regex() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    let e = run(&tool(&root), p("(((")).expect_err("invalid");
    assert!(e.to_string().contains("invalid grep pattern"), "{e}");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rejects_oversized_pattern() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    let long = "a".repeat(GREP_PATTERN_MAX_CHARS + 1);
    let e = run(
        &tool(&root),
        GrepParams {
            pattern: long.as_str(),
            ..Default::default()
        },
    )
    .expect_err("too long");
    assert!(e.to_string().contains("invalid grep pattern"), "{e}");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn denies_path_outside_workspace() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    let e = run(
        &tool(&root),
        GrepParams {
            pattern: "x",
            path: Some(".."),
            ..Default::default()
        },
    )
    .expect_err("denied");
    assert_eq!(
        e.to_string(),
        "Access denied: path is outside the workspace"
    );
    stdfs::remove_dir_all(&dir).unwrap();
}

// ---- streaming / batch protocol ----

#[test]
fn finds_known_match_in_tail_of_large_file() {
    // 16MB 超 fixture は TS と同じ要領(ただし行数を落として CI を
    // 軽く保つ — streaming 証明には chunk/batch 境界を跨げば足りる)。
    let dir = tmpdir("cheaper-grep-big-");
    let root = root_of(&dir);
    let lines = 60_000usize;
    let mut buf = String::new();
    for i in 1..=lines {
        buf.push_str(&format!("line-{i:07} {}\n", "x".repeat(80)));
    }
    let needle_line = lines - 10;
    buf = buf.replacen(
        &format!("line-{needle_line:07} {}", "x".repeat(80)),
        &format!("line-{needle_line:07} UNIQUE_NEEDLE"),
        1,
    );
    assert!(buf.len() > 4 * 1024 * 1024);
    stdfs::write(dir.join("big.txt"), buf).unwrap();
    let out = run(
        &tool(&root),
        GrepParams {
            pattern: "UNIQUE_NEEDLE",
            glob: Some("big.txt"),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        out,
        format!("big.txt:{needle_line}: line-{needle_line:07} UNIQUE_NEEDLE")
    );
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn matches_lines_spanning_chunk_and_batch_boundaries() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join("x.txt"), "aaaaaaaa\nhit-target\nzz\n").unwrap();
    let t = create_grep_tool(opts(
        &root,
        GrepToolOptions {
            chunk_bytes: Some(5),
            batch_max_lines: Some(1),
            ..default_options()
        },
    ))
    .unwrap();
    let out = run(
        &t,
        GrepParams {
            pattern: "hit-target",
            glob: Some("x.txt"),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out, "x.txt:2: hit-target");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn merges_context_blocks_across_batch_boundaries() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join("m.txt"), "l1\nhit\nl3\nhit\nl5\nhit\nl7\n").unwrap();
    let t = create_grep_tool(opts(
        &root,
        GrepToolOptions {
            batch_max_lines: Some(3),
            ..default_options()
        },
    ))
    .unwrap();
    let out = run(
        &t,
        GrepParams {
            pattern: "hit",
            context: Some(1),
            glob: Some("m.txt"),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        out,
        "m.txt-1- l1\nm.txt:2: hit\nm.txt-3- l3\nm.txt:4: hit\nm.txt-5- l5\nm.txt:6: hit\nm.txt-7- l7"
    );
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn closes_batch_at_oversized_line_preserving_line_numbers() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(
        dir.join("o.txt"),
        "l1\nl2\nl3\nl4\nOVERSIZED-LINE-CONTENT\nhit6\n",
    )
    .unwrap();
    let t = create_grep_tool(opts(
        &root,
        GrepToolOptions {
            max_line_bytes: Some(10),
            batch_max_lines: Some(100),
            ..default_options()
        },
    ))
    .unwrap();
    let out = run(
        &t,
        GrepParams {
            pattern: "hit6",
            glob: Some("o.txt"),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out, "o.txt:6: hit6\n[skipped: oversized-line — o.txt:5]");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn closes_batch_before_exceeding_batch_max_bytes() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join("b.txt"), "aaa\nbbb\ncccc\nddd\n").unwrap();
    // invariant: B=12 >= 3*L=12。"aaa"+"bbb"+"cccc"=10 byte で 1 batch、
    // "ddd" を足すと 13 > 12 なので次 batch に締める。
    let t = create_grep_tool(opts(
        &root,
        GrepToolOptions {
            max_line_bytes: Some(4),
            batch_max_bytes: Some(12),
            batch_max_lines: Some(100),
            ..default_options()
        },
    ))
    .unwrap();
    let out = run(
        &t,
        GrepParams {
            pattern: "ddd",
            glob: Some("b.txt"),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out, "b.txt:4: ddd");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rejects_construction_when_batch_bytes_below_3x_line_bytes() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    let e = create_grep_tool(opts(
        &root,
        GrepToolOptions {
            batch_max_bytes: Some(4),
            max_line_bytes: Some(100),
            ..default_options()
        },
    ))
    .err()
    .expect("invariant");
    assert!(e.to_string().contains("batchMaxBytes"), "{e}");
    let e = create_grep_tool(opts(
        &root,
        GrepToolOptions {
            batch_max_bytes: Some(10),
            ..default_options()
        },
    ))
    .err()
    .expect("invariant");
    assert!(e.to_string().contains("batchMaxBytes"), "{e}");
    // 境界値 B = 3*L は受理される。
    create_grep_tool(opts(
        &root,
        GrepToolOptions {
            max_line_bytes: Some(4),
            batch_max_bytes: Some(12),
            ..default_options()
        },
    ))
    .expect("boundary ok");
    stdfs::remove_dir_all(&dir).unwrap();
}

// ---- skip footer ----

#[test]
fn reports_binary_files_in_footer() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join("b.dat"), [0x68u8, 0x69, 0x00, 0x74]).unwrap();
    stdfs::write(dir.join("t.txt"), "hit\n").unwrap();
    let out = run(&tool(&root), p("hit")).unwrap();
    assert_eq!(out, "t.txt:1: hit\n[skipped: binary — b.dat]");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn reports_oversized_lines_as_path_line() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join("o.txt"), "ok\nthis-is-a-very-long-line\nhit\n").unwrap();
    let t = create_grep_tool(opts(
        &root,
        GrepToolOptions {
            max_line_bytes: Some(10),
            ..default_options()
        },
    ))
    .unwrap();
    let out = run(
        &t,
        GrepParams {
            pattern: "hit",
            glob: Some("o.txt"),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out, "o.txt:3: hit\n[skipped: oversized-line — o.txt:2]");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn distinguishes_oversized_skip_from_no_matches() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join("o.txt"), "a-very-very-long-line\n").unwrap();
    let t = create_grep_tool(opts(
        &root,
        GrepToolOptions {
            max_line_bytes: Some(10),
            ..default_options()
        },
    ))
    .unwrap();
    let out = run(
        &t,
        GrepParams {
            pattern: "hit",
            glob: Some("o.txt"),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out, "(no matches)\n[skipped: oversized-line — o.txt:1]");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn bounds_representative_entries_and_counts_rest() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    for i in 0..25 {
        stdfs::write(dir.join(format!("b{i:02}.dat")), [0x61u8, 0x00, 0x62]).unwrap();
    }
    let out = run(&tool(&root), p("hit")).unwrap();
    assert!(out.contains("[skipped: binary — "), "{out}");
    assert!(out.contains("and 5 more"), "{out}");
    let shown = out.matches(".dat").count();
    assert!(shown <= 20, "{out}");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn truncates_long_paths_at_skip_path_max_chars() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    let long_name = format!("{}.dat", "d".repeat(200));
    stdfs::write(dir.join(&long_name), [0x61u8, 0x00]).unwrap();
    let t = create_grep_tool(opts(
        &root,
        GrepToolOptions {
            skip_path_max_chars: Some(20),
            ..default_options()
        },
    ))
    .unwrap();
    let out = run(&t, p("hit")).unwrap();
    assert_eq!(
        out,
        format!("(no matches)\n[skipped: binary — {}]", "d".repeat(20))
    );
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn emits_skipped_report_truncated_over_footer_cap() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    for i in 0..4 {
        stdfs::write(
            dir.join(format!("long-name-{}-{i}.dat", "n".repeat(40))),
            [0x00u8],
        )
        .unwrap();
    }
    let t = create_grep_tool(opts(
        &root,
        GrepToolOptions {
            skip_footer_max_bytes: Some(40),
            ..default_options()
        },
    ))
    .unwrap();
    let out = run(&t, p("hit")).unwrap();
    assert!(out.contains("[skipped-report: truncated]"), "{out}");
    assert!(!out.contains("[skipped: binary"), "{out}");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn hides_denied_files_from_results_and_footer() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join(".env"), "SECRET=1\n").unwrap();
    stdfs::write(dir.join("normal.txt"), "hit\n").unwrap();
    let denylist = create_path_denylist(&[]);
    let t = create_grep_tool(GrepToolOptions {
        denylist: Some(&denylist),
        ..opts(&root, default_options())
    })
    .unwrap();
    let out = run(&t, p("SECRET")).unwrap();
    assert!(!out.contains(".env"), "{out}");
    assert!(!out.contains("SECRET=1"), "{out}");
    let out2 = run(&t, p("hit")).unwrap();
    assert_eq!(out2, "normal.txt:1: hit");
    stdfs::remove_dir_all(&dir).unwrap();
}

// ---- eval budgets and deadline ----

/// fake clock: 呼出ごとに `step` ms 進める(TS fakeClock 相当)。
/// 1 batch の eval 計測は now() 複数呼出ぶんとして積算される。
fn fake_clock(step: f64) -> Box<dyn Fn() -> f64 + Send + Sync> {
    let t = std::sync::Arc::new(std::sync::Mutex::new(0.0f64));
    Box::new(move || {
        let mut g = t.lock().unwrap();
        *g += step;
        *g
    })
}

#[test]
fn per_file_budget_accumulates_skips_file_and_recovers() {
    // step=200ms → 1 batch の eval 計測(t0→dt で ~4 呼出)≈800ms。
    // perFile=1000 は 2 batch(1600ms)で累積超過する。watch は各 batch
    // で十分な残り(200ms〜)になるよう余裕を取る。
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    let lines: Vec<String> = (0..30).map(|i| format!("l{i}")).collect();
    stdfs::write(dir.join("a.txt"), format!("{}\n", lines.join("\n"))).unwrap();
    stdfs::write(dir.join("b.txt"), "hit\n").unwrap();
    let t = create_grep_tool(opts(
        &root,
        GrepToolOptions {
            batch_max_lines: Some(3),
            eval_per_file_ms: Some(1000),
            now: Some(fake_clock(200.0)),
            ..default_options()
        },
    ))
    .unwrap();
    let out = run(&t, p("hit")).unwrap();
    assert!(out.contains("b.txt:1: hit"), "{out}");
    assert!(out.contains("[skipped: timeout — a.txt]"), "{out}");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn total_budget_stops_scan() {
    // 列挙順に依存しないよう両 file を同形にする。先に走査された
    // file が total 予算を使い切り、走査全体が止まる。
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    let lines: Vec<String> = (0..30).map(|i| format!("l{i}")).collect();
    let body = format!("hit\n{}\n", lines.join("\n"));
    stdfs::write(dir.join("a.txt"), &body).unwrap();
    stdfs::write(dir.join("b.txt"), &body).unwrap();
    let t = create_grep_tool(opts(
        &root,
        GrepToolOptions {
            batch_max_lines: Some(3),
            eval_total_ms: Some(1600),
            now: Some(fake_clock(200.0)),
            ..default_options()
        },
    ))
    .unwrap();
    let out = run(&t, p("hit")).unwrap();
    assert!(
        out.contains("[truncated: evaluation time limit reached]"),
        "{out}"
    );
    let scanned: Vec<&str> = ["a.txt", "b.txt"]
        .iter()
        .copied()
        .filter(|n| out.contains(n))
        .collect();
    assert_eq!(scanned.len(), 1, "{out}");
    assert!(out.contains(&format!("{}:1: hit", scanned[0])), "{out}");
    assert!(
        out.contains(&format!("[skipped: timeout — {}]", scanned[0])),
        "{out}"
    );
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn scan_deadline_returns_partial_results_with_marker() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join("a.txt"), "hit\n").unwrap();
    let t = create_grep_tool(opts(
        &root,
        GrepToolOptions {
            scan_deadline_ms: Some(0),
            ..default_options()
        },
    ))
    .unwrap();
    let out = run(&t, p("hit")).unwrap();
    assert!(out.contains("[truncated: scan deadline reached]"), "{out}");
    assert!(!out.contains("a.txt:1: hit"), "{out}");
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rejects_with_operation_aborted_on_signal() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join("a.txt"), "hit\n").unwrap();
    let flag = AtomicBool::new(true);
    let aborted = || flag.load(Ordering::SeqCst);
    let e = execute_grep(&tool(&root), &p("hit"), Some(&aborted)).expect_err("abort");
    assert_eq!(e.to_string(), "Operation aborted");
    stdfs::remove_dir_all(&dir).unwrap();
}

// ---- output bounds ----

#[test]
fn truncates_long_displayed_line() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    stdfs::write(dir.join("long.txt"), format!("hit{}\n", "a".repeat(100))).unwrap();
    let t = create_grep_tool(opts(
        &root,
        GrepToolOptions {
            output_line_max_chars: Some(10),
            ..default_options()
        },
    ))
    .unwrap();
    let out = run(
        &t,
        GrepParams {
            pattern: "hit",
            glob: Some("long.txt"),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        out,
        format!("long.txt:1: hit{}... [truncated]", "a".repeat(7))
    );
    stdfs::remove_dir_all(&dir).unwrap();
}

#[test]
fn stops_output_at_output_max_bytes() {
    let dir = tmpdir("cheaper-grep-");
    let root = root_of(&dir);
    let lines: Vec<String> = (0..30)
        .map(|i| format!("match-line-{i}-xxxxxxxx"))
        .collect();
    stdfs::write(dir.join("o.txt"), format!("{}\n", lines.join("\n"))).unwrap();
    let t = create_grep_tool(opts(
        &root,
        GrepToolOptions {
            output_max_bytes: Some(100),
            ..default_options()
        },
    ))
    .unwrap();
    let out = run(
        &t,
        GrepParams {
            pattern: "match",
            glob: Some("o.txt"),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(out.contains("[truncated: output limit reached]"), "{out}");
    let body = out.split("\n[truncated").next().unwrap_or("");
    assert!(body.len() <= 100, "{body}");
    stdfs::remove_dir_all(&dir).unwrap();
}
