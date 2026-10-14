use cheaper_core::policy::load::{load_policy, LoadPolicyOptions};

use crate::common::*;

#[test]
fn load_cases() {
    let f = fixture("policy/load.json");
    for case in cases(&f) {
        let name = case_name(case);
        let Some(c) = case_dir(case) else {
            panic!("{name}: case dir setup failed");
        };
        let opts = &case["opts"];
        let dir = c.dir.to_string_lossy().to_string();
        let builtin = join(&dir, opts["builtinPolicyPath"].as_str().unwrap());
        let user = join(&dir, opts["userConfigDir"].as_str().unwrap());
        let project = join(&dir, opts["projectDir"].as_str().unwrap());
        let invocation = opts
            .get("invocation")
            .and_then(serde_json::Value::as_object);
        let produced = match load_policy(&LoadPolicyOptions {
            builtin_policy_path: &builtin,
            user_config_dir: &user,
            project_dir: &project,
            invocation,
        }) {
            Ok(p) => Ok(serde_json::json!({
                "policy": serde_json::to_value(&p).unwrap_or(serde_json::Value::Null)
            })),
            Err(e) => Err(cheaper_err(&e, &c.needles)),
        };
        check(&name, &case["expected"], produced);
    }
}
