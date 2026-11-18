---
name: verify-exec-policy
description: Exhaustive fail-closed verification of the cheaper exec-policy decision boundary — every (name, fixed) grant lookup, every argv pattern over a bounded token alphabet against every arg list, every deny-list spelling variant, and every command.grants/process.tasks folding combination is checked against the real resolution code in cheaper-core. Run this before merging any change to crates/cheaper-core/src/policy/exec.rs (grant resolution, argv gating, exec-input folding, route projection) or crates/cheaper-core/src/policy/security.rs (ExecDenyList, executable_stem). A pass covers the bounded sweep only, not the whole system.
---

# Verify exec-policy fail-closed invariants

**Property (plain words):** nothing executes unless a declared grant
with the right `fixed` flag matches the requested name AND the args
satisfy the grant's argv pattern; a deny-listed executable is rejected
wherever it is declared, however it is spelled; grant folding never
invents a grant the policy did not declare.

**The harm if it breaks:** an exec boundary hole is a policy bypass —
a worker, route, or workflow runs a command the operator never
granted, or a deny-listed binary (e.g. `ssh`, `curl`) executes because
its spelling slipped past the deny check.

## Run

```bash
mise exec -- cargo test -p cheaper-core --test exhaustive_policy
```

Seven tests, <0.1 s total, deterministic, no network/fs/spawn:

- `every_name_fixed_query_resolves_iff_declared` — every declared-grant
  subset × every (name, fixed) query: `resolve_exec_grant` must return
  the declared grant iff the name was declared with that fixed flag
  (24 queries, including returned-grant equality — no field drift).
- `argv_gate_agrees_with_oracle_on_every_args` — every token pattern
  over a 6-token alphabet up to length 3 (literals, `p*` prefix, bare
  `*` rest, invalid placements `**`/`a* b`-style) is compiled by the
  real `ArgPattern::compile`; compile-accept must equal an independent
  oracle, and `matches` must equal the oracle on every arg list over a
  7-symbol alphabet up to length 3 (>4000 predicate checks).
- `deny_stem_semantics_on_every_spelling` — `ExecDenyList::is_denied`
  truth table over 17 spellings: case variants, `.exe`/`.com` strip
  (once only), `/` and `\` paths, substring non-matches, anchored
  full-match.
- `denied_executable_fails_resolve_in_any_position` — a denied
  executable in each of 3 grant positions, spelled 3 ways
  (`evil`, `C:\bin\EVIL.EXE`, `./lib/evil.com`), must make the whole
  `resolve_exec_policy` fail regardless of scope/fixed shape.
- `folding_produces_exactly_the_declared_union` — every
  tasks×grants subset combination through `build_exec_input`: folded
  keyset must equal the declared union, tasks fold to `fixed:true`
  literal-argv grants, command grants stay non-fixed, and the folded
  doc round-trips through `resolve_exec_policy` with identical keyset.
- `folding_collides_fail_closed` — a name present in both
  `command.grants` and `process.tasks` must always be an error.
- `route_eligibility_and_projection_matrix` — `is_route_eligible` is
  exactly Trusted ∧ !fixed (4-combination truth table);
  `project_grants` keeps {requested eligible} ∪ {fixed} and rejects —
  without mutating the map — any request containing a missing or
  ineligible name.

## Coverage record

- **Verified code:** `crates/cheaper-core/src/policy/exec.rs` —
  `resolve_exec_grant` (name + fixed-flag gate), `ArgPattern::compile`
  / `ArgPattern::matches` (the argv predicate `authorize_args` calls),
  `resolve_exec_policy` (deny application point 1 on declared
  executables, grant map construction), `build_exec_input`
  (task→fixed-grant fold, cross-section collision gate),
  `ResolvedExecPolicy::is_route_eligible` / `project_grants` (route
  projection must not broaden authorization);
  `crates/cheaper-core/src/policy/security.rs` —
  `ExecDenyList::compile` / `is_denied` (anchored case-insensitive
  regex), `executable_stem` (basename → lowercase → strip one
  `.exe`/`.com`).
- **Check:** `crates/cheaper-core/tests/exhaustive_policy.rs` — the
  only file the next agent edits when the invariants themselves
  change.
- **Sync commit:** `78c79a2` — the last commit at which this check was
  confirmed to describe the code above. If `git diff 78c79a2 --
  crates/cheaper-core/src/policy/exec.rs crates/cheaper-core/src/policy/security.rs`
  is non-empty, re-read the diff, confirm the check still covers the
  changed behavior, and only then move the sync commit forward.

## Interpreting a failure

Failures print the counterexample in the code's own terms:
`resolve_exec_grant(alpha, fixed=true) on ["alpha"]`,
`denied "evil" at position 0 resolved ok`,
`collision ["x"]/["x"] accepted`, or
`matches(["p*"], ["z"]) disagrees with oracle`.

- **Real bug** — a grant/deny/folding decision disagrees with the
  declared contract on real resolution output. The bug is in
  `exec.rs`/`security.rs`. Reproduce on the printed case, fix at the
  owning gate (lookup, argv, deny, or fold), rerun.
- **Modeling error** — the check's assumptions drifted from the API:
  `ExecGrant`/`ResolvedExecPolicy` fields changed, `build_exec_input`
  section shapes moved, `ArgPattern` token grammar intentionally
  extended (e.g. mid-token globs), route-projection semantics changed.
  Fix `tests/exhaustive_policy.rs` to match the new contract, rerun,
  then update the sync commit. If a semantic intentionally changed
  (e.g. fixed/non-fixed surfaces merged), update the invariant in the
  doc comment at the top of that file too — and treat any widened
  allowance as a security-review item, not a test fix.

## Known failure modes (proven, not hypothetical)

- Removing the `fixed` flag check in `resolve_exec_grant`
  (`Some(grant) if grant.fixed == fixed` → `Some(grant)`): a non-fixed
  command grant resolves on the fixed task surface →
  `every_name_fixed_query_resolves_iff_declared` fails on the first
  mismatched query.
- Disabling the declared-executable deny gate in `resolve_exec_policy`
  (`if deny.is_denied(&executable)` → `if false && ...`):
  `denied_executable_fails_resolve_in_any_position` reports the
  accepted spelling and position.
- Disabling the cross-section collision gate in `build_exec_input`:
  `folding_collides_fail_closed` reports the accepted colliding name —
  a `process.tasks` name silently shadowed by a `command.grants`
  entry would flip the fixed/non-fixed surface of that name.

## Scope

Covers: grant-name/fixed lookup soundness, argv pattern compile-accept
boundaries and match semantics (the predicate `authorize_args` uses),
declared-executable deny application at resolve time, deny stem
normalization, task/grant folding keyset parity and fixed-flag
preservation, cross-section collision rejection, route eligibility and
non-broadening projection.

Does NOT cover: runtime enforcement in `crates/cheaper-tools`
(`check_exec_execution`, `verify_pinned_executable`, PATH resolution,
pre-spawn deny re-checks — application points 2 and 3) — this check
verifies the *decision* layer, not the *actuation* layer; multi-layer
policy merge in `load.rs`/`merge.rs`; argv patterns or arg lists
longer than length 3 over larger alphabets (the bound exercises every
distinct token kind, rest-position, and prefix boundary); non-ASCII
case folding; or JSON schema validation beyond the shapes the check
constructs.
