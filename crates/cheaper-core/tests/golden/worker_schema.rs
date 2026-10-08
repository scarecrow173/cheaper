use serde_json::{json, Map, Value};

use cheaper_core::worker::schema::{
    compile_schema, load_schema, parse_schema_ref, SchemaKind, SchemaRef,
};

use crate::common::*;

fn kind(v: &Value) -> SchemaKind {
    match v.as_str() {
        Some("input") => SchemaKind::Input,
        _ => SchemaKind::Output,
    }
}

#[test]
fn schema_cases() {
    let f = fixture("worker/schema.json");
    for case in cases(&f) {
        let name = case_name(case);
        let op = case["op"].as_str().unwrap();
        let produced = match op {
            "parseSchemaRef" => {
                match parse_schema_ref(
                    &case["raw"],
                    kind(&case["kind"]),
                    case["sourcePath"].as_str().unwrap(),
                ) {
                    Ok(r) => {
                        let refv = match r {
                            SchemaRef::File(f) => json!({"file": f}),
                            SchemaRef::Inline(m) => json!({"inline": m}),
                        };
                        Ok(json!({"ref": refv}))
                    }
                    Err(e) => Err(cheaper_err(&e, &[])),
                }
            }
            "loadSchema" | "compile" | "compileValidate" => {
                let Some(c) = case_dir(case) else {
                    panic!("{name}: case dir setup failed");
                };
                let produced: Result<Value, Value> = match op {
                    "loadSchema" => {
                        let r = schema_ref(&case["ref"]);
                        match load_schema(&r, &c.root, kind(&case["kind"])) {
                            Ok(schema) => Ok(json!({"schema": schema})),
                            Err(e) => Err(cheaper_err(&e, &c.needles)),
                        }
                    }
                    "compile" => {
                        let schema = obj(&case["schema"]);
                        match compile_schema(schema, kind(&case["kind"])) {
                            Ok(_) => Ok(json!({"compiled": true})),
                            Err(e) => Err(cheaper_err(&e, &c.needles)),
                        }
                    }
                    _ => {
                        let schema = obj(&case["schema"]);
                        match compile_schema(schema, kind(&case["kind"])) {
                            Err(e) => Err(cheaper_err(&e, &c.needles)),
                            Ok(compiled) => match compiled.validate(&case["data"]) {
                                Ok(()) => Ok(json!({"ok": true})),
                                Err(errors) => Ok(json!({
                                    "ok": false,
                                    "errors": errors.iter().map(|e| json!({"instancePath": e.instance_path})).collect::<Vec<_>>(),
                                })),
                            },
                        }
                    }
                };
                check(&name, &case["expected"], produced);
                continue;
            }
            other => panic!("unknown op {other} in {name}"),
        };
        check(&name, &case["expected"], produced);
    }
}

fn schema_ref(v: &Value) -> SchemaRef {
    if let Some(f) = v.get("file").and_then(Value::as_str) {
        SchemaRef::File(f.to_string())
    } else if let Some(i) = v.get("inline").and_then(Value::as_object) {
        SchemaRef::Inline(i.clone())
    } else {
        panic!("bad schema ref {v}")
    }
}

fn obj(v: &Value) -> Map<String, Value> {
    v.as_object().expect("schema object").clone()
}
