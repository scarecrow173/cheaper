use std::collections::HashMap;
use std::path::Path;

use cheaper_core::config::dirs::{resolve_config_dirs, ConfigDirsOptions};

use crate::common::*;

#[test]
fn dirs_cases() {
    let f = fixture("dirs.json");
    for case in cases(&f) {
        let name = case_name(case);
        let env_map: HashMap<String, String> = case
            .get("env")
            .and_then(|v| v.as_object())
            .map(|o| {
                o.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();
        let env = |k: &str| env_map.get(k).cloned();
        let homedir = case.get("homedir").and_then(|v| v.as_str()).map(Path::new);
        let dirs = resolve_config_dirs(&ConfigDirsOptions {
            cwd: Path::new(case["cwd"].as_str().unwrap()),
            builtin_dir: Path::new(case["builtinDir"].as_str().unwrap()),
            env: Some(&env),
            homedir,
        });
        let want = &case["expected"]["ok"];
        assert_eq!(
            slashify(&dirs.user_config_dir.to_string_lossy()),
            slashify(want["userConfigDir"].as_str().unwrap()),
            "{name}: userConfigDir"
        );
        assert_eq!(
            slashify(&dirs.project_dir.to_string_lossy()),
            slashify(want["projectDir"].as_str().unwrap()),
            "{name}: projectDir"
        );
        assert_eq!(
            slashify(&dirs.builtin_dir.to_string_lossy()),
            slashify(want["builtinDir"].as_str().unwrap()),
            "{name}: builtinDir"
        );
    }
}
