//! Node `path` semantics re-implemented on strings.
//!
//! `packages/core` treats paths as strings and relies on `path.resolve` /
//! `path.join` / `path.relative` / `path.dirname` / `path.basename` /
//! `path.extname` / `path.isAbsolute` byte-level behavior. Rust `Path`
//! does not normalize `..` lexically and formats differently, so the
//! win32 and posix variants are re-implemented here. The platform
//! selection mirrors Node: `path.*` (the `path` module) follows the host
//! OS, `path.win32` / `path.posix` are always available.
//!
//! Only the surface `packages/core` uses is implemented; `resolve` is
//! specialized to the two-argument `resolve(base, input)` form that the
//! ported code actually calls.

/// `path.sep` for the host platform.
#[cfg(windows)]
pub const SEP: char = '\\';
/// `path.sep` for the host platform.
#[cfg(not(windows))]
pub const SEP: char = '/';

fn is_win32_separator(c: char) -> bool {
    c == '/' || c == '\\'
}

fn is_posix_separator(c: char) -> bool {
    c == '/'
}

/// `path.win32.isAbsolute`. Independent of the host OS, like Node.
///
/// Absolute when: `<drive>:<sep>...`, any UNC/rooted path starting with a
/// separator (`/`, `\\server\share`, `\\?\...`).
pub fn win32_is_absolute(p: &str) -> bool {
    if p.is_empty() {
        return false;
    }
    if is_win32_separator(p.as_bytes()[0] as char) {
        return true;
    }
    // `<drive>:<sep>`
    let b = p.as_bytes();
    b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && is_win32_separator(b[2] as char)
}

/// `path.posix.isAbsolute`. Independent of the host OS, like Node.
#[cfg_attr(windows, allow(dead_code))] // only reached via `is_absolute` off Windows
pub fn posix_is_absolute(p: &str) -> bool {
    p.starts_with('/')
}

/// `path.isAbsolute` for the host platform.
#[cfg(windows)]
pub fn is_absolute(p: &str) -> bool {
    win32_is_absolute(p)
}

/// `path.isAbsolute` for the host platform.
#[cfg(not(windows))]
pub fn is_absolute(p: &str) -> bool {
    posix_is_absolute(p)
}

/// `<drive>:` prefix such as `c:`/`C:` (without a following separator).
fn win32_drive_prefix(p: &str) -> Option<usize> {
    let b = p.as_bytes();
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        Some(2)
    } else {
        None
    }
}

/// Split a win32 path into `(device, is_absolute, tail)`.
/// `device` is `""` for paths without a drive/UNC prefix, `X:` for drive
/// paths, `\\server\share` for UNC, and `\\?\X:` / `\\?\UNC\server\share` /
/// `\\.\X:` for device paths.
fn win32_split_device(path: &str) -> (&str, bool, &str) {
    let b = path.as_bytes();
    if b.len() >= 2 && is_win32_separator(b[0] as char) && is_win32_separator(b[1] as char) {
        // UNC / device path. `\\?\UNC\server\share` first.
        if b.len() >= 8 && path[2..].starts_with("?\\") {
            let rest = &path[4..];
            let rb = rest.as_bytes();
            if rb.len() >= 5
                && rest[..4].eq_ignore_ascii_case("UNC\\")
                && is_win32_separator(rb[3] as char)
            {
                // \\?\UNC\server\share
                if let Some(device_len) = win32_unc_device_len(&rest[4..]) {
                    let end = 4 + 4 + device_len;
                    return (&path[..end], true, &path[end..]);
                }
            }
            // \\?\X: or \\.\X:
            if rb.len() >= 2 && rb[0].is_ascii_alphabetic() && rb[1] == b':' {
                // device = \\?\X:
                return (&path[..6], true, &path[6..]);
            }
            // Other `\\?\...` device paths: no parseable tail.
            return (path, true, "");
        }
        // \\server\share
        if let Some(device_len) = win32_unc_device_len(&path[2..]) {
            let end = 2 + device_len;
            return (&path[..end], true, &path[end..]);
        }
        return (path, true, "");
    }
    if let Some(drive_end) = win32_drive_prefix(path) {
        let absolute = b
            .get(drive_end)
            .is_some_and(|c| is_win32_separator(*c as char));
        return (&path[..drive_end], absolute, &path[drive_end..]);
    }
    ("", is_win32_separator(b[0] as char), path)
}

/// Length of the `server\share` portion of a UNC tail (input does not
/// include the leading separators).
fn win32_unc_device_len(tail: &str) -> Option<usize> {
    let mut segs = tail
        .split(|c: char| is_win32_separator(c))
        .filter(|s| !s.is_empty());
    let server = segs.next()?;
    let share = segs.next()?;
    Some(server.len() + 1 + share.len())
}

/// Node `normalizeString`: split a tail on separators and fold `.`/`..`.
/// `allow_above_root` mirrors Node (`..` kept for relative tails, dropped
/// for absolute ones). Trailing `..` are preserved for relative tails.
fn win32_normalize_tail(tail: &str, allow_above_root: bool) -> String {
    let mut out: Vec<&str> = Vec::new();
    for seg in tail.split(|c: char| is_win32_separator(c)) {
        match seg {
            "" | "." => {}
            ".." => {
                if out.last() == Some(&"..") || out.is_empty() {
                    if allow_above_root {
                        out.push("..");
                    }
                } else {
                    out.pop();
                }
            }
            _ => out.push(seg),
        }
    }
    out.join("\\")
}

/// Node `path.win32.resolve(base, input)` for two arguments.
///
/// Right-to-left semantics: an absolute `input` discards `base`; a rooted
/// (`\foo`) input takes `base`'s drive; a drive-relative input (`C:foo`)
/// resolves against `base` when the drive matches, else against that
/// drive's current directory (`=X:` env) or `X:\`.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn win32_resolve(base: &str, input: &str) -> String {
    let (dev_i, abs_i, tail_i) = win32_split_device(input);
    let (dev_b, _abs_b, tail_b) = win32_split_device(base);
    if dev_i.is_empty() {
        // Rooted (`\foo`): keep base device, tail is the input tail.
        if abs_i {
            let tail = win32_normalize_tail(tail_i, false);
            return format!("{dev_b}\\{tail}");
        }
        // Plain relative: base tail + input tail under the base device.
        let combined = if tail_b.is_empty() {
            tail_i.to_string()
        } else {
            format!("{tail_b}\\{tail_i}")
        };
        let tail = win32_normalize_tail(&combined, dev_b.is_empty());
        return format!("{dev_b}\\{tail}");
    }
    if dev_i.eq_ignore_ascii_case(dev_b) || dev_b.is_empty() {
        let combined = if abs_i {
            tail_i.to_string()
        } else {
            format!("{tail_b}\\{tail_i}")
        };
        let tail = win32_normalize_tail(&combined, !abs_i && dev_i.is_empty());
        if abs_i {
            return format!("{dev_i}\\{tail}");
        }
        // Drive-relative resolved against the base tail.
        return if dev_b.is_empty() {
            tail
        } else {
            format!("{dev_i}\\{tail}")
        };
    }
    // Different device: base is discarded entirely (Node `continue`).
    let tail = win32_normalize_tail(tail_i, !abs_i);
    if abs_i {
        return format!("{dev_i}\\{tail}");
    }
    // Drive-relative with a different drive: resolve against that drive's
    // cwd (`=X:` env) or the drive root, per Node.
    let drive_cwd = std::env::var(format!("={dev_i}")).unwrap_or_else(|_| format!("{dev_i}\\"));
    win32_resolve(&drive_cwd, tail_i)
}

/// Node `path.win32.join(parts...)`.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn win32_join(parts: &[&str]) -> String {
    let non_empty: Vec<&str> = parts.iter().copied().filter(|p| !p.is_empty()).collect();
    if non_empty.is_empty() {
        return ".".to_string();
    }
    let joined = non_empty.join("\\");
    win32_normalize_keep_trailing(&joined)
}

fn win32_normalize_keep_trailing(path: &str) -> String {
    let (dev, abs, tail) = win32_split_device(path);
    let keep_trailing = tail.chars().next_back().is_some_and(is_win32_separator);
    let norm = win32_normalize_tail(tail, !abs && dev.is_empty());
    if norm.is_empty() && !keep_trailing {
        return if abs || !dev.is_empty() {
            format!("{dev}\\")
        } else {
            ".".to_string()
        };
    }
    let mut out = String::from(dev);
    if abs || (!dev.is_empty() && !norm.starts_with("..")) {
        out.push('\\');
    }
    out.push_str(&norm);
    if keep_trailing && !norm.is_empty() {
        out.push('\\');
    }
    if norm.is_empty() && dev.is_empty() && !abs {
        return ".".to_string();
    }
    out
}

/// Node `path.win32.relative(from, to)` for already-normalized absolute
/// paths. Comparison is case-insensitive (Node lowercases both sides);
/// emitted segments keep `to`'s original casing.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn win32_relative(from: &str, to: &str) -> String {
    let (dev_f, _, tail_f) = win32_split_device(from);
    let (dev_t, _, tail_t) = win32_split_device(to);
    if !dev_f.eq_ignore_ascii_case(dev_t) {
        return to.to_string();
    }
    let f: Vec<&str> = tail_f
        .split(|c: char| is_win32_separator(c))
        .filter(|s| !s.is_empty())
        .collect();
    let t: Vec<&str> = tail_t
        .split(|c: char| is_win32_separator(c))
        .filter(|s| !s.is_empty())
        .collect();
    let mut common = 0;
    while common < f.len() && common < t.len() && f[common].eq_ignore_ascii_case(t[common]) {
        common += 1;
    }
    let mut out: Vec<String> = vec!["..".to_string(); f.len() - common];
    out.extend(t[common..].iter().map(|s| (*s).to_string()));
    out.join("\\")
}

fn posix_normalize_tail(tail: &str, allow_above_root: bool) -> String {
    let mut out: Vec<&str> = Vec::new();
    for seg in tail.split(is_posix_separator) {
        match seg {
            "" | "." => {}
            ".." => {
                if out.last() == Some(&"..") || out.is_empty() {
                    if allow_above_root {
                        out.push("..");
                    }
                } else {
                    out.pop();
                }
            }
            _ => out.push(seg),
        }
    }
    out.join("/")
}

/// Node `path.posix.resolve(base, input)`.
#[cfg_attr(windows, allow(dead_code))]
pub fn posix_resolve(base: &str, input: &str) -> String {
    let tail = if input.starts_with('/') {
        posix_normalize_tail(input, false)
    } else {
        posix_normalize_tail(&format!("{base}/{input}"), false)
    };
    format!("/{tail}")
}

/// Node `path.posix.join(parts...)`.
#[cfg_attr(windows, allow(dead_code))]
pub fn posix_join(parts: &[&str]) -> String {
    let non_empty: Vec<&str> = parts.iter().copied().filter(|p| !p.is_empty()).collect();
    if non_empty.is_empty() {
        return ".".to_string();
    }
    let joined = non_empty.join("/");
    let abs = joined.starts_with('/');
    let keep_trailing = joined.ends_with('/');
    let norm = posix_normalize_tail(&joined, !abs);
    if norm.is_empty() {
        return if abs {
            "/".to_string()
        } else {
            ".".to_string()
        };
    }
    let mut out = String::new();
    if abs {
        out.push('/');
    }
    out.push_str(&norm);
    if keep_trailing {
        out.push('/');
    }
    out
}

/// Node `path.posix.relative(from, to)` for normalized absolute paths.
#[cfg_attr(windows, allow(dead_code))]
pub fn posix_relative(from: &str, to: &str) -> String {
    let f: Vec<&str> = from.split('/').filter(|s| !s.is_empty()).collect();
    let t: Vec<&str> = to.split('/').filter(|s| !s.is_empty()).collect();
    let mut common = 0;
    while common < f.len() && common < t.len() && f[common] == t[common] {
        common += 1;
    }
    let mut out: Vec<String> = vec!["..".to_string(); f.len() - common];
    out.extend(t[common..].iter().map(|s| (*s).to_string()));
    out.join("/")
}

/// `path.resolve(base, input)` for the host platform.
#[cfg(windows)]
pub fn resolve(base: &str, input: &str) -> String {
    win32_resolve(base, input)
}

/// `path.resolve(base, input)` for the host platform.
#[cfg(not(windows))]
pub fn resolve(base: &str, input: &str) -> String {
    posix_resolve(base, input)
}

/// `path.join(parts...)` for the host platform.
#[cfg(windows)]
pub fn join(parts: &[&str]) -> String {
    win32_join(parts)
}

/// `path.join(parts...)` for the host platform.
#[cfg(not(windows))]
pub fn join(parts: &[&str]) -> String {
    posix_join(parts)
}

/// `path.relative(from, to)` for the host platform.
#[cfg(windows)]
pub fn relative(from: &str, to: &str) -> String {
    win32_relative(from, to)
}

/// `path.relative(from, to)` for the host platform.
#[cfg(not(windows))]
pub fn relative(from: &str, to: &str) -> String {
    posix_relative(from, to)
}

fn last_sep_index(p: &str, is_sep: fn(char) -> bool) -> Option<usize> {
    p.char_indices()
        .rev()
        .find(|(_, c)| is_sep(*c))
        .map(|(i, _)| i)
}

fn trim_end_separators(p: &str, is_sep: fn(char) -> bool) -> &str {
    p.trim_end_matches(is_sep)
}

/// `path.basename(p[, ext])` for the host platform.
pub fn basename(p: &str, ext: Option<&str>) -> String {
    let is_sep: fn(char) -> bool = if cfg!(windows) {
        is_win32_separator
    } else {
        is_posix_separator
    };
    let trimmed = trim_end_separators(p, is_sep);
    let base = match last_sep_index(trimmed, is_sep) {
        Some(i) => &trimmed[i + 1..],
        None => trimmed,
    };
    if let Some(ext) = ext {
        if base.len() > ext.len() && base.ends_with(ext) {
            return base[..base.len() - ext.len()].to_string();
        }
    }
    base.to_string()
}

/// `path.extname(p)` for the host platform: `.` + everything after the
/// last `.` of the basename, ignoring a leading run of dots. `""` when
/// the basename has no `.` beyond leading dots.
pub fn extname(p: &str) -> String {
    let base = basename(p, None);
    let rest = base.trim_start_matches('.');
    match rest.rfind('.') {
        Some(i) => rest[i..].to_string(),
        None => String::new(),
    }
}

/// `path.dirname(p)` for the host platform.
pub fn dirname(p: &str) -> String {
    let is_sep: fn(char) -> bool = if cfg!(windows) {
        is_win32_separator
    } else {
        is_posix_separator
    };
    let trimmed = trim_end_separators(p, is_sep);
    match last_sep_index(trimmed, is_sep) {
        Some(0) => trimmed[..1].to_string(),
        Some(i) => {
            // Strip a run of separators preceding the final component.
            let head = trim_end_separators(&trimmed[..i], is_sep);
            if head.is_empty() {
                trimmed[..1].to_string()
            } else if cfg!(windows) && win32_drive_prefix(head).is_some() && head.len() == 2 {
                // `C:\x` → `C:\` (drive root preserved).
                format!("{head}{SEP}")
            } else {
                head.to_string()
            }
        }
        None => {
            // Rootless (e.g. "C:foo" → "C:", "foo" → ".").
            if cfg!(windows) && win32_drive_prefix(trimmed).is_some() {
                trimmed[..2].to_string()
            } else {
                ".".to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn win32_resolve_matches_node() {
        assert_eq!(win32_resolve("C:\\root", "a/b.txt"), "C:\\root\\a\\b.txt");
        assert_eq!(win32_resolve("C:\\root", "..\\x"), "C:\\x");
        assert_eq!(win32_resolve("C:\\root", "."), "C:\\root");
        assert_eq!(win32_resolve("C:\\root", "D:\\abs"), "D:\\abs");
        assert_eq!(win32_resolve("C:\\root", "c:/abs"), "c:\\abs");
        assert_eq!(win32_resolve("C:\\root", "\\rooted"), "C:\\rooted");
        // `..` above root is dropped for absolute results.
        assert_eq!(win32_resolve("C:\\root", "..\\..\\x"), "C:\\x");
    }

    #[cfg(windows)]
    #[test]
    fn win32_relative_matches_node() {
        assert_eq!(win32_relative("C:\\a\\b", "C:\\a\\b\\c.txt"), "c.txt");
        assert_eq!(win32_relative("C:\\a\\b", "C:\\a\\b"), "");
        assert_eq!(win32_relative("C:\\a\\b", "C:\\a\\x"), "..\\x");
        // Case-insensitive comparison; output keeps `to` casing.
        assert_eq!(win32_relative("c:\\Work", "C:\\work\\Secret"), "Secret");
    }

    #[test]
    fn extname_matches_node() {
        assert_eq!(extname("a/b/file.txt"), ".txt");
        assert_eq!(extname("a/b/.env"), "");
        assert_eq!(extname("a/b/f."), ".");
        assert_eq!(extname("a/b/f.tar.gz"), ".gz");
    }

    #[test]
    fn basename_extname_roundtrip_stem() {
        let p = if cfg!(windows) {
            "C:\\w\\workers\\writer.md"
        } else {
            "/w/workers/writer.md"
        };
        assert_eq!(basename(p, Some(&extname(p))), "writer");
    }
}
