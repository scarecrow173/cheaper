//! Exhaustive fail-closed verification of the exec-policy decision
//! boundary.
//!
//! Property (plain words): nothing executes unless a declared grant
//! with the right `fixed` flag matches the requested name AND the args
//! satisfy the grant's argv pattern; a deny-listed executable is
//! rejected wherever it is declared, however it is spelled; grant
//! folding never invents a grant the policy did not declare.
//!
//! The harm if it breaks: an exec boundary hole is a policy-bypass —
//! a worker or workflow runs a command the operator never granted.
//!
//! Method: enumerate the decision space of the REAL resolution code —
//! no model, no mocks. `resolve_exec_grant` is queried for every
//! (name, fixed) pair against every declared-grant subset;
//! `ArgPattern::matches` is checked against an independent oracle for
//! every token pattern over a bounded alphabet times every arg list;
//! `resolve_exec_policy` is driven with deny-matching executables in
//! every declaration position and spelling variant;
//! `build_exec_input` folding is checked for exact keyset parity.
//!
//! Covered code: `src/policy/exec.rs` (`resolve_exec_grant`,
//! `ArgPattern::{compile,matches}`, `resolve_exec_policy`,
//! `build_exec_input`, `is_route_eligible`, `project_grants`),
//! `src/policy/security.rs` (`ExecDenyList::{compile,is_denied}`,
//! `executable_stem`).

use std::collections::BTreeSet;

use serde_json::{json, Map, Value};

use cheaper_core::policy::exec::{
    build_exec_input, resolve_exec_grant, resolve_exec_policy, ArgPattern, ExecDefaults, ExecGrant,
    ExecGrantScope, ExecHeartbeat, ResolvedExecPolicy,
};
use cheaper_core::policy::security::{executable_stem, ExecDenyList};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn grant(executable: &str, fixed: bool, argv: Option<Vec<String>>) -> ExecGrant {
    ExecGrant {
        executable: executable.to_string(),
        fixed,
        argv,
        inputs: None,
        outputs: None,
        cwd: ".".to_string(),
        env: Map::new(),
        timeout: "60s".to_string(),
        kill_grace: "2s".to_string(),
        max_output_bytes: 65536,
        exit_codes: vec![0],
        scope: ExecGrantScope::Trusted,
    }
}

fn policy_with(grants: Vec<(&str, ExecGrant)>) -> ResolvedExecPolicy {
    let defaults = || ExecDefaults {
        max_concurrent: 1,
        timeout: "60s".to_string(),
        max_output_bytes: 65536,
        kill_grace: "2s".to_string(),
        heartbeat: ExecHeartbeat {
            interval: "30s".to_string(),
        },
    };
    ResolvedExecPolicy {
        grants: grants
            .into_iter()
            .map(|(n, g)| (n.to_string(), g))
            .collect(),
        command: defaults(),
        process: defaults(),
    }
}

// ---------------------------------------------------------------------------
// A. grant-lookup soundness — resolve_exec_grant returns a grant iff the
//    name was declared AND the requested `fixed` flag matches. No
//    fixed-flag confusion, no unknown-name grant.
// ---------------------------------------------------------------------------

#[test]
fn every_name_fixed_query_resolves_iff_declared() {
    // declared universe: alpha(!fixed), beta(fixed). Every presence
    // subset is a separate resolved policy.
    let universes = vec![
        vec![],
        vec![("alpha", grant("aa", false, None))],
        vec![("beta", grant("bb", true, Some(vec!["--x".into()])))],
        vec![
            ("alpha", grant("aa", false, None)),
            ("beta", grant("bb", true, Some(vec!["--x".into()]))),
        ],
    ];
    let mut checked = 0usize;
    for universe in &universes {
        let policy = policy_with(universe.clone());
        for name in ["alpha", "beta", "gamma"] {
            for fixed in [false, true] {
                let declared = universe.iter().any(|(n, g)| *n == name && g.fixed == fixed);
                let result = resolve_exec_grant(&policy, name, fixed);
                assert_eq!(
                    result.is_ok(),
                    declared,
                    "resolve_exec_grant({name}, fixed={fixed}) on {:?}",
                    universe.iter().map(|(n, _)| *n).collect::<Vec<_>>()
                );
                if let Ok(g) = result {
                    let declared_grant =
                        universe.iter().find(|(n, _)| n == &name).unwrap().1.clone();
                    assert_eq!(
                        g, declared_grant,
                        "{name}: resolved grant drifted from declaration"
                    );
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 24);
}

// ---------------------------------------------------------------------------
// B. argv gate — ArgPattern::compile accepts exactly the declared token
//    grammar (literals, single trailing prefix `x*`, single trailing
//    rest `*`), and `matches` agrees with an independent oracle on
//    every arg list over the bounded alphabet.
// ---------------------------------------------------------------------------

/// Independent reference implementation of the declared argv contract:
/// - pattern `[]`/`[lits..]` with no Rest requires exact arg equality
///   (position-wise: literal must equal, `pre*` requires arg starts
///   with `pre`).
/// - a single trailing bare `*` (Rest) allows zero-or-more extra args
///   after the fixed prefix.
///
/// Any other `*` usage is invalid and must fail `compile`.
fn oracle_valid(pattern: &[&str]) -> bool {
    pattern.iter().enumerate().all(|(i, t)| {
        let stars = t.matches('*').count();
        match stars {
            0 => true,
            1 if *t == "*" => i == pattern.len() - 1,
            1 => t.ends_with('*') && !t.starts_with('*'),
            _ => false,
        }
    })
}

fn oracle_matches(pattern: &[&str], args: &[&str]) -> bool {
    let has_rest = pattern.last() == Some(&"*");
    let fixed: &[&str] = if has_rest {
        &pattern[..pattern.len() - 1]
    } else {
        pattern
    };
    if has_rest {
        if args.len() < fixed.len() {
            return false;
        }
    } else if args.len() != fixed.len() {
        return false;
    }
    fixed.iter().zip(args.iter()).all(|(tok, arg)| {
        if let Some(prefix) = tok.strip_suffix('*') {
            arg.starts_with(prefix)
        } else {
            *tok == *arg
        }
    })
}

#[test]
fn argv_gate_agrees_with_oracle_on_every_args() {
    // Pattern vocabulary: literals, a trailing-prefix token, a rest
    // marker, plus invalid placements — all lengths 0..=3.
    let toks = ["a", "b", "p*", "*", "**", "a*b"];
    let mut patterns: Vec<Vec<&str>> = vec![vec![]];
    for _len in 1..=3usize {
        let mut cur: Vec<Vec<&str>> = Vec::new();
        for base in &patterns {
            for t in toks {
                let mut p = base.clone();
                p.push(t);
                cur.push(p);
            }
        }
        patterns.extend(cur);
    }
    // Args vocabulary: exact literals, prefix hit/miss, metachars.
    let syms = ["a", "b", "px", "py", "z", ";rm", "-h"];
    let mut arg_lists: Vec<Vec<&str>> = vec![vec![]];
    for _len in 1..=3usize {
        let mut cur: Vec<Vec<&str>> = Vec::new();
        for base in &arg_lists {
            for s in syms {
                let mut a = base.clone();
                a.push(s);
                cur.push(a);
            }
        }
        arg_lists.extend(cur);
    }

    let mut checked = 0usize;
    let mut compiled = 0usize;
    for pattern in &patterns {
        let owned: Vec<String> = pattern.iter().map(|s| s.to_string()).collect();
        let valid = oracle_valid(pattern);
        let result = ArgPattern::compile(&owned);
        assert_eq!(
            result.is_ok(),
            valid,
            "compile({pattern:?}) validity disagrees with oracle"
        );
        if !valid {
            continue;
        }
        compiled += 1;
        let compiled_pat = result.unwrap();
        for args in &arg_lists {
            let owned_args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
            let expect = oracle_matches(pattern, args);
            let got = compiled_pat.matches(&owned_args);
            assert_eq!(
                got, expect,
                "matches({pattern:?}, {args:?}) disagrees with oracle"
            );
            checked += 1;
        }
    }
    // sanity: the sweep covered every arg list against every valid pattern
    assert!(compiled > 30, "too few valid patterns: {compiled}");
    assert!(checked > 4000, "too few arg checks: {checked}");
}

// ---------------------------------------------------------------------------
// C. deny fail-closed — a denied executable spelling fails the whole
//    resolve wherever it is declared; is_denied stem semantics hold on
//    every spelling variant; empty deny never rejects.
// ---------------------------------------------------------------------------

#[test]
fn deny_stem_semantics_on_every_spelling() {
    let deny = ExecDenyList::compile(&["evil".to_string(), "ba.".to_string()]).unwrap();
    // (spelling, denied?) — stem = basename → lowercase → strip .exe/.com once.
    let cases: &[(&str, bool)] = &[
        ("evil", true),
        ("EVIL", true),
        ("EvIl.ExE", true),
        ("evil.com", true),
        ("evil.exe.com", false), // strip once only → stem "evil.exe"
        ("C:\\tools\\evil.exe", true),
        ("/usr/bin/evil", true),
        ("./evil", true),
        ("evilx", false),
        ("xevil", false),
        ("bae", true), // ba. regex: 'b' 'a' any
        ("ban", true),
        ("bad.com", true),
        ("b.an", false), // extra char — full-match anchored
        ("good", false),
        ("git", false),
        ("", false),
    ];
    for (spelling, expect) in cases {
        assert_eq!(
            deny.is_denied(spelling),
            *expect,
            "is_denied({spelling:?}) — stem {:?}",
            executable_stem(spelling)
        );
    }
    assert!(ExecDenyList::default().is_empty());
    assert!(!ExecDenyList::default().is_denied("evil"));
}

#[test]
fn denied_executable_fails_resolve_in_any_position() {
    let deny = ExecDenyList::compile(&["evil".to_string()]).unwrap();
    // Policy with three grants; the denied executable occupies each
    // position in turn, plus both scope/fixed shapes.
    for (pos, denied_exe) in [
        (0usize, "evil"),
        (1, "C:\\bin\\EVIL.EXE"),
        (2, "./lib/evil.com"),
    ] {
        let mut grants = Map::new();
        for i in 0..3 {
            let exe = if i == pos { denied_exe } else { "good" };
            grants.insert(
                format!("g{i}"),
                json!({ "executable": exe, "fixed": i == 1, "scope": "project" }),
            );
        }
        let doc = json!({ "grants": Value::Object(grants) });
        let result = resolve_exec_policy(Some(&doc), &deny);
        assert!(
            result.is_err(),
            "denied {denied_exe:?} at position {pos} resolved ok"
        );
    }
    // Empty deny: same docs resolve.
    let clean = ExecDenyList::compile(&[]).unwrap();
    let mut grants = Map::new();
    grants.insert(
        "g".to_string(),
        json!({ "executable": "evil", "fixed": false }),
    );
    let doc = json!({ "grants": Value::Object(grants) });
    assert!(resolve_exec_policy(Some(&doc), &clean).is_ok());
    // No exec doc at all → empty grants, still ok (deny is vacuous).
    assert!(resolve_exec_policy(None, &deny).unwrap().grants.is_empty());
}

// ---------------------------------------------------------------------------
// D. folding integrity — build_exec_input produces exactly the declared
//    union: task names + grant names, nothing invented; a name in both
//    sections is a fail-closed collision; the folded doc round-trips
//    through resolve_exec_policy unchanged in keyset.
// ---------------------------------------------------------------------------

fn process_section(task_names: &[&str]) -> Value {
    let mut tasks = Map::new();
    for n in task_names {
        tasks.insert(
            n.to_string(),
            json!({ "command": ["t-exe", "--flag"], "inputs": [], "outputs": [] }),
        );
    }
    json!({ "tasks": Value::Object(tasks) })
}

fn command_section(grant_names: &[&str]) -> Value {
    let mut grants = Map::new();
    for n in grant_names {
        grants.insert(n.to_string(), json!({ "executable": "g-exe" }));
    }
    json!({ "grants": Value::Object(grants) })
}

#[test]
fn folding_produces_exactly_the_declared_union() {
    let combos: &[(&[&str], &[&str])] = &[
        (&[], &[]),
        (&["t1"], &[]),
        (&[], &["g1"]),
        (&["t1", "t2"], &["g1"]),
        (&["t1"], &["g1", "g2"]),
        (&["t1", "t2", "t3"], &["g1", "g2"]),
    ];
    let mut checked = 0usize;
    for (tasks, grants) in combos {
        let input = build_exec_input(
            Some(&process_section(tasks)),
            Some(&command_section(grants)),
            &BTreeSet::new(),
        )
        .unwrap_or_else(|e| panic!("fold {tasks:?}+{grants:?}: {e}"));
        let keys: BTreeSet<&str> = input["grants"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let declared: BTreeSet<&str> = tasks.iter().chain(grants.iter()).cloned().collect();
        assert_eq!(
            keys, declared,
            "folded keyset != declared union for {tasks:?}+{grants:?}"
        );
        // tasks fold to fixed:true literal-argv grants; grants keep !fixed.
        for t in *tasks {
            let g = &input["grants"][*t];
            assert_eq!(g["fixed"], json!(true), "{t}: task not folded fixed");
            assert_eq!(g["executable"], json!("t-exe"));
            assert_eq!(g["argv"], json!(["--flag"]));
        }
        for n in *grants {
            let g = &input["grants"][*n];
            assert_ne!(g["fixed"], json!(true), "{n}: grant folded as fixed");
            assert_eq!(g["executable"], json!("g-exe"));
        }
        // folded doc resolves under an empty deny with the same keyset.
        let resolved = resolve_exec_policy(
            Some(&Value::Object(input.clone())),
            &ExecDenyList::default(),
        )
        .unwrap_or_else(|e| panic!("resolve folded {tasks:?}+{grants:?}: {e}"));
        let resolved_keys: BTreeSet<&str> = resolved.grants.keys().map(String::as_str).collect();
        assert_eq!(resolved_keys, declared);
        checked += 1;
    }
    assert_eq!(checked, 6);
}

#[test]
fn folding_collides_fail_closed() {
    for names in [
        (vec!["x"], vec!["x"]),
        (vec!["t1", "x"], vec!["x", "g1"]),
        (vec!["x"], vec!["a", "x"]),
    ] {
        let result = build_exec_input(
            Some(&process_section(&names.0)),
            Some(&command_section(&names.1)),
            &BTreeSet::new(),
        );
        assert!(
            result.is_err(),
            "collision {:?}/{:?} accepted",
            names.0,
            names.1
        );
    }
}

// ---------------------------------------------------------------------------
// E. route eligibility — is_route_eligible is exactly Trusted ∧ !fixed;
//    project_grants keeps {requested eligible} ∪ {fixed} and rejects
//    any request containing missing or ineligible names.
// ---------------------------------------------------------------------------

#[test]
fn route_eligibility_and_projection_matrix() {
    let mk = |fixed: bool, scope: ExecGrantScope| ExecGrant {
        scope,
        ..grant("x", fixed, None)
    };
    assert!(ResolvedExecPolicy::is_route_eligible(&mk(
        false,
        ExecGrantScope::Trusted
    )));
    assert!(!ResolvedExecPolicy::is_route_eligible(&mk(
        false,
        ExecGrantScope::Project
    )));
    assert!(!ResolvedExecPolicy::is_route_eligible(&mk(
        true,
        ExecGrantScope::Trusted
    )));
    assert!(!ResolvedExecPolicy::is_route_eligible(&mk(
        true,
        ExecGrantScope::Project
    )));

    let policy = policy_with(vec![
        ("e1", mk(false, ExecGrantScope::Trusted)),
        ("e2", mk(false, ExecGrantScope::Project)),
        ("e3", mk(true, ExecGrantScope::Trusted)),
    ]);
    // request sets → (expected bool, expected surviving names)
    let cases: &[(&[&str], bool, &[&str])] = &[
        (&["e1"], true, &["e1", "e3"]),
        (&["e1", "e3"], false, &[]), // e3 is fixed → ineligible request
        (&["e2"], false, &[]),       // project scope → ineligible
        (&["e1", "e2"], false, &[]),
        (&["missing"], false, &[]),
        (&[], false, &[]),
    ];
    for (req, ok, surviving) in cases {
        let mut p = policy.clone();
        let names: BTreeSet<String> = req.iter().map(|s| s.to_string()).collect();
        let before: BTreeSet<String> = p.grants.keys().cloned().collect();
        let got = p.project_grants(&names);
        assert_eq!(got, *ok, "project_grants({req:?})");
        if *ok {
            let after: BTreeSet<&str> = p.grants.keys().map(String::as_str).collect();
            assert_eq!(after, surviving.iter().cloned().collect());
        } else {
            let after: BTreeSet<String> = p.grants.keys().cloned().collect();
            assert_eq!(after, before, "rejected projection must not mutate");
        }
    }
}
