//! Port of `packages/core/src/workspace/deny-paths.ts`.
//!
//! Privacy deny list for workspace-relative display paths (SPEC-0010 /
//! DESIGN-0015). Glob patterns are converted to `regex` patterns by the
//! same rules as the TS `denyPatternToRegex` (no lookahead/backreference
//! is emitted, so the `regex` crate covers the full language).

use regex::Regex;

/// `DEFAULT_DENY_PATHS` verbatim (declaration order preserved).
pub const DEFAULT_DENY_PATHS: &[&str] = &[
    ".env",
    ".env.*",
    "*.pem",
    "*.key",
    "*.p12",
    "*.pfx",
    "*.keystore",
    "id_rsa",
    "id_ed25519",
    "id_ecdsa",
    "id_dsa",
    ".netrc",
    ".npmrc",
    ".pypirc",
    "credentials.json",
    "secrets.json",
    "secrets.yaml",
    "secrets.yml",
    ".ssh/**",
    ".aws/**",
    ".gnupg/**",
];

/// Port of `PathDenylist`.
#[derive(Debug)]
pub struct PathDenylist {
    regexes: Vec<Regex>,
    /// Effective pattern list (built-in + policy), in order.
    pub patterns: Vec<String>,
}

impl PathDenylist {
    /// `isDenied(displayPath)`: workspace-relative POSIX display path.
    pub fn is_denied(&self, display_path: &str) -> bool {
        let p = display_path.strip_prefix("./").unwrap_or(display_path);
        self.regexes.iter().any(|re| re.is_match(p))
    }
}

fn escape_reg_exp(s: &str) -> String {
    // JS escapeRegExp charset: . + ^ $ { } ( ) | [ ] \
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if matches!(
            ch,
            '.' | '+' | '^' | '$' | '{' | '}' | '(' | ')' | '|' | '[' | ']' | '\\'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn segment_to_regex(seg: &str) -> String {
    let mut out = String::new();
    for ch in seg.chars() {
        match ch {
            '*' => out.push_str("[^/]*"),
            '?' => out.push_str("[^/]"),
            _ => out.push_str(&escape_reg_exp(&ch.to_string())),
        }
    }
    out
}

fn deny_pattern_to_regex_impl(pattern: &str, case_insensitive: bool) -> Regex {
    let mut anchored = false;
    let mut p = pattern;
    if let Some(rest) = p.strip_prefix('/') {
        anchored = true;
        p = rest;
    }
    let mut dir_itself = false;
    if p == "**" || p.ends_with("/**") {
        dir_itself = true;
        p = if p == "**" { "" } else { &p[..p.len() - 3] };
    }
    let segs: Vec<&str> = p.split('/').filter(|s| !s.is_empty()).collect();
    let body = segs
        .iter()
        .map(|s| {
            if *s == "**" {
                "(?:[^/]+/)*".to_string()
            } else {
                format!("{}/", segment_to_regex(s))
            }
        })
        .collect::<String>();
    // `.replace(/\/$/, "")`: strip exactly one trailing separator.
    let body = body.strip_suffix('/').unwrap_or(&body).to_string();
    let tail = if dir_itself { "(?:/.*)?" } else { "" };
    let prefix = if anchored { "^" } else { "^(?:.*/)?" };
    // Windows FS is case-insensitive; deny comparison matches so casing
    // cannot bypass the denylist (JS `i` flag).
    regex::RegexBuilder::new(&format!("{prefix}{body}{tail}$"))
        .case_insensitive(case_insensitive)
        .build()
        .expect("deny pattern compiles")
}

/// `denyPatternToRegex` for the host platform (win32 → case-insensitive).
/// Test seam: [`deny_pattern_to_regex_for`] takes the flag explicitly.
pub fn deny_pattern_to_regex(pattern: &str) -> Regex {
    deny_pattern_to_regex_impl(pattern, cfg!(windows))
}

/// `denyPatternToRegex` with an explicit case-insensitivity flag (the
/// `process.platform === "win32"` axis of the TS tests).
pub fn deny_pattern_to_regex_for(pattern: &str, case_insensitive: bool) -> Regex {
    deny_pattern_to_regex_impl(pattern, case_insensitive)
}

/// `createPathDenylist(extra)`. `extra` is appended after the built-ins;
/// there is no way to disable a built-in (privacy floor).
pub fn create_path_denylist(extra: &[&str]) -> PathDenylist {
    create_path_denylist_for(extra, cfg!(windows))
}

/// `createPathDenylist` with explicit case sensitivity (test seam for the
/// platform-mocked TS tests).
pub fn create_path_denylist_for(extra: &[&str], case_insensitive: bool) -> PathDenylist {
    let patterns: Vec<String> = DEFAULT_DENY_PATHS
        .iter()
        .map(|s| (*s).to_string())
        .chain(extra.iter().map(|s| (*s).to_string()))
        .collect();
    let regexes = patterns
        .iter()
        .map(|p| deny_pattern_to_regex_impl(p, case_insensitive))
        .collect();
    PathDenylist { regexes, patterns }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_pattern_matrix() {
        let cases: &[(&str, &str, bool)] = &[
            // basename pattern matches at any depth
            (".env", ".env", true),
            (".env", "config/.env", true),
            (".env", ".env.local", false),
            (".env", "envfile", false),
            // wildcard in segment
            (".env.*", ".env.local", true),
            (".env.*", "a/b/.env.production", true),
            (".env.*", ".env", false),
            ("*.pem", "certs/server.pem", true),
            ("*.pem", "certs/pem.txt", false),
            // dir pattern matches the dir itself and its contents
            (".ssh/**", ".ssh", true),
            (".ssh/**", ".ssh/id_rsa", true),
            (".ssh/**", "home/user/.ssh/config", true),
            (".ssh/**", ".sshd_config", false),
            (".ssh/**", "ssh/file", false),
            // root-anchored pattern
            ("/config/secrets.json", "config/secrets.json", true),
            ("/config/secrets.json", "sub/config/secrets.json", false),
            // ** in the middle
            ("a/**/b.txt", "a/x/y/b.txt", true),
            ("a/**/b.txt", "a/b.txt", true),
            // ? wildcard
            ("secret?.txt", "secret1.txt", true),
            ("secret?.txt", "secret12.txt", false),
        ];
        for (pattern, display, expected) in cases {
            assert_eq!(
                deny_pattern_to_regex(pattern).is_match(display),
                *expected,
                "{pattern} vs {display}"
            );
        }
    }

    #[test]
    fn respects_filesystem_casing() {
        for case_insensitive in [true, false] {
            let list = create_path_denylist_for(&["Blocked", "/Private/**"], case_insensitive);
            for file in [".ENV", "certs/KEY.PEM", "blocked", "PRIVATE/data.json"] {
                assert_eq!(list.is_denied(file), case_insensitive, "{file}");
            }
            assert!(list.is_denied("Blocked"));
            assert!(list.is_denied("Private/data.json"));
            assert!(!list.is_denied("sub/Private/data.json"));
            let mut expected: Vec<String> =
                DEFAULT_DENY_PATHS.iter().map(|s| s.to_string()).collect();
            expected.extend(["Blocked".to_string(), "/Private/**".to_string()]);
            assert_eq!(list.patterns, expected);
        }
    }

    #[test]
    fn denies_every_default_representative_at_any_depth() {
        let list = create_path_denylist(&[]);
        for p in [
            ".env",
            "config/.env.local",
            "certs/server.pem",
            "keys/private.key",
            "id_rsa",
            "a/b/id_ed25519",
            ".netrc",
            "pkg/.npmrc",
            "secrets.yaml",
            ".ssh/config",
            "app/.aws/credentials",
        ] {
            assert!(list.is_denied(p), "{p}");
        }
    }

    #[test]
    fn does_not_deny_ordinary_files() {
        let list = create_path_denylist(&[]);
        for p in [
            "src/index.ts",
            "docs/guide.md",
            "environment.ts",
            "README.md",
            "src/secrets-manager.ts",
            "keynote.md",
        ] {
            assert!(!list.is_denied(p), "{p}");
        }
    }

    #[test]
    fn merges_extra_without_removing_builtins() {
        let list = create_path_denylist(&["internal/**"]);
        assert!(list.is_denied("internal/notes.txt"));
        assert!(list.is_denied(".env"));
        assert_eq!(list.patterns.len(), DEFAULT_DENY_PATHS.len() + 1);
    }

    #[test]
    fn normalizes_leading_dot_slash() {
        let list = create_path_denylist(&[]);
        assert!(list.is_denied("./.env"));
    }
}
