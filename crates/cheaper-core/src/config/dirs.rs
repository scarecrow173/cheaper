//! Port of `packages/core/src/config/dirs.ts`.

use std::path::{Path, PathBuf};

/// Port of `ConfigDirs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDirs {
    pub user_config_dir: PathBuf,
    pub project_dir: PathBuf,
    pub builtin_dir: PathBuf,
}

/// Environment lookup injected for test isolation (mirrors the TS `env` opt).
/// `None` falls back to `std::env::var`, which is case-insensitive on Windows
/// like `process.env`.
pub type EnvLookup<'a> = &'a dyn Fn(&str) -> Option<String>;

/// Options for [`resolve_user_config_dir`]. `homedir` mirrors the TS opt;
/// when absent the home directory is resolved from `USERPROFILE` → `HOME` →
/// OS homedir, in that order.
#[derive(Default, Clone, Copy)]
pub struct UserConfigDirOptions<'a> {
    pub env: Option<EnvLookup<'a>>,
    pub homedir: Option<&'a Path>,
}

fn process_env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// Resolve the user-scope cheaper config dir:
/// `CHEAPER_CONFIG_DIR` → `XDG_CONFIG_HOME/cheaper` → `<home>/.config/cheaper`.
///
/// `CHEAPER_CONFIG_DIR`/`XDG_CONFIG_HOME` use JS truthiness (empty string is
/// ignored). `USERPROFILE`/`HOME` use `??` semantics (a present-but-empty
/// value still counts), matching `dirs.ts`.
pub fn resolve_user_config_dir(opts: UserConfigDirOptions<'_>) -> PathBuf {
    let env = opts.env.unwrap_or(&process_env);
    if let Some(dir) = env("CHEAPER_CONFIG_DIR").filter(|v| !v.is_empty()) {
        return PathBuf::from(dir);
    }
    if let Some(xdg) = env("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return Path::new(&xdg).join("cheaper");
    }
    let home = opts
        .homedir
        .map(PathBuf::from)
        .or_else(|| env("USERPROFILE").map(PathBuf::from))
        .or_else(|| env("HOME").map(PathBuf::from))
        .or_else(std::env::home_dir)
        .unwrap_or_default();
    home.join(".config").join("cheaper")
}

/// Options for [`resolve_config_dirs`].
#[derive(Clone, Copy)]
pub struct ConfigDirsOptions<'a> {
    pub cwd: &'a Path,
    pub builtin_dir: &'a Path,
    pub env: Option<EnvLookup<'a>>,
    pub homedir: Option<&'a Path>,
}

/// Port of `resolveConfigDirs`: `<cwd>/.cheaper` + injected `builtinDir`.
pub fn resolve_config_dirs(opts: &ConfigDirsOptions<'_>) -> ConfigDirs {
    ConfigDirs {
        user_config_dir: resolve_user_config_dir(UserConfigDirOptions {
            env: opts.env,
            homedir: opts.homedir,
        }),
        project_dir: opts.cwd.join(".cheaper"),
        builtin_dir: opts.builtin_dir.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        let map: HashMap<&str, &str> = pairs.iter().copied().collect();
        move |k: &str| map.get(k).map(|v| v.to_string())
    }

    fn opts<'a>(
        env: &'a [(&'a str, &'a str)],
        homedir: Option<&'a Path>,
    ) -> (impl Fn(&str) -> Option<String> + 'a, Option<&'a Path>) {
        (self::env(env), homedir)
    }

    const HOME: &str = "/home/u";

    #[test]
    fn prefers_cheaper_config_dir() {
        let (env, homedir) = opts(
            &[
                ("CHEAPER_CONFIG_DIR", "/custom"),
                ("XDG_CONFIG_HOME", "/xdg"),
            ],
            None,
        );
        let d = resolve_user_config_dir(UserConfigDirOptions {
            env: Some(&env),
            homedir,
        });
        assert_eq!(d, PathBuf::from("/custom"));
    }

    #[test]
    fn falls_back_to_xdg_config_home() {
        let (env, homedir) = opts(&[("XDG_CONFIG_HOME", "/xdg")], Some(Path::new(HOME)));
        let d = resolve_user_config_dir(UserConfigDirOptions {
            env: Some(&env),
            homedir,
        });
        assert_eq!(d, Path::new("/xdg").join("cheaper"));
    }

    #[test]
    fn falls_back_to_home_dot_config() {
        let (env, homedir) = opts(&[], Some(Path::new(HOME)));
        let d = resolve_user_config_dir(UserConfigDirOptions {
            env: Some(&env),
            homedir,
        });
        assert_eq!(d, Path::new(HOME).join(".config").join("cheaper"));
    }

    #[test]
    fn ignores_empty_cheaper_config_dir_and_xdg_config_home() {
        let (env, homedir) = opts(
            &[("CHEAPER_CONFIG_DIR", ""), ("XDG_CONFIG_HOME", "")],
            Some(Path::new(HOME)),
        );
        let d = resolve_user_config_dir(UserConfigDirOptions {
            env: Some(&env),
            homedir,
        });
        assert_eq!(d, Path::new(HOME).join(".config").join("cheaper"));
    }

    #[test]
    fn resolves_home_from_userprofile_then_home_env() {
        let (env, _) = opts(&[("USERPROFILE", "/up"), ("HOME", "/h")], None);
        let d = resolve_user_config_dir(UserConfigDirOptions {
            env: Some(&env),
            homedir: None,
        });
        assert_eq!(d, Path::new("/up").join(".config").join("cheaper"));

        let (env, _) = opts(&[("HOME", "/h")], None);
        let d = resolve_user_config_dir(UserConfigDirOptions {
            env: Some(&env),
            homedir: None,
        });
        assert_eq!(d, Path::new("/h").join(".config").join("cheaper"));
    }

    #[test]
    fn config_dirs_sets_project_dir_and_builtin_dir() {
        let (env, homedir) = opts(&[], Some(Path::new(HOME)));
        let d = resolve_config_dirs(&ConfigDirsOptions {
            cwd: Path::new("/work/proj"),
            builtin_dir: Path::new("/builtin"),
            env: Some(&env),
            homedir,
        });
        assert_eq!(
            d.user_config_dir,
            Path::new(HOME).join(".config").join("cheaper")
        );
        assert_eq!(d.project_dir, Path::new("/work/proj").join(".cheaper"));
        assert_eq!(d.builtin_dir, PathBuf::from("/builtin"));
    }

    #[test]
    fn config_dirs_prefers_cheaper_config_dir() {
        let (env, homedir) = opts(
            &[
                ("CHEAPER_CONFIG_DIR", "/custom"),
                ("XDG_CONFIG_HOME", "/xdg"),
            ],
            Some(Path::new(HOME)),
        );
        let d = resolve_config_dirs(&ConfigDirsOptions {
            cwd: Path::new("/work/proj"),
            builtin_dir: Path::new("/builtin"),
            env: Some(&env),
            homedir,
        });
        assert_eq!(d.user_config_dir, PathBuf::from("/custom"));
    }

    #[test]
    fn uses_process_env_and_os_homedir_defaults() {
        // With no injected env/homedir the result must still end with
        // `.config/cheaper` unless CHEAPER_CONFIG_DIR/XDG_CONFIG_HOME are set
        // in the real environment.
        let d = resolve_config_dirs(&ConfigDirsOptions {
            cwd: Path::new("/w"),
            builtin_dir: Path::new("/b"),
            env: Some(&|_| None),
            homedir: None,
        });
        assert!(d
            .user_config_dir
            .ends_with(Path::new(".config").join("cheaper")));
    }
}
