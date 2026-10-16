//! `input.rs` / `count.rs` の contract test。

use cheaper_cli::cli::CliStdin;
use cheaper_cli::count::count_max_bytes;
use cheaper_cli::input::read_task_input;
use cheaper_core::errors::ErrorCode;

struct Stdin {
    data: Vec<u8>,
    tty: bool,
    fail: bool,
}
impl CliStdin for Stdin {
    fn is_tty(&self) -> bool {
        self.tty
    }
    fn read_all(&mut self) -> std::io::Result<Vec<u8>> {
        if self.fail {
            return Err(std::io::Error::other("io fail"));
        }
        Ok(std::mem::take(&mut self.data))
    }
}

fn stdin_tty() -> Stdin {
    Stdin {
        data: vec![],
        tty: true,
        fail: false,
    }
}

fn stdin_pipe(data: &str) -> Stdin {
    Stdin {
        data: data.as_bytes().to_vec(),
        tty: false,
        fail: false,
    }
}

#[test]
fn tty_stdin_rejected() {
    let e = read_task_input("-", &mut stdin_tty()).unwrap_err();
    assert_eq!(e.code(), ErrorCode::InvalidRequest);
    assert_eq!(
        e.message(),
        "no task input: stdin is a TTY and --input was not given"
    );
}

#[test]
fn empty_stdin_rejected() {
    let e = read_task_input("-", &mut stdin_pipe("   \n\t")).unwrap_err();
    assert_eq!(e.code(), ErrorCode::InvalidRequest);
    assert_eq!(e.message(), "no task input: stdin was empty");
}

#[test]
fn invalid_json_rejected() {
    let e = read_task_input("-", &mut stdin_pipe("{oops")).unwrap_err();
    assert_eq!(e.code(), ErrorCode::InvalidRequest);
    assert_eq!(e.message(), "task input is not valid JSON");
    assert_eq!(
        e.details()
            .unwrap()
            .get("reason")
            .unwrap()
            .as_str()
            .unwrap(),
        "json-parse"
    );
}

#[test]
fn non_object_rejected() {
    for input in ["[1,2]", "\"x\"", "null", "42"] {
        let e = read_task_input("-", &mut stdin_pipe(input)).unwrap_err();
        assert_eq!(e.code(), ErrorCode::InvalidRequest, "input={input}");
        assert_eq!(e.message(), "task input must be a JSON object");
        assert_eq!(
            e.details()
                .unwrap()
                .get("reason")
                .unwrap()
                .as_str()
                .unwrap(),
            "not-object"
        );
    }
}

#[test]
fn stdin_object_accepted() {
    let v = read_task_input("-", &mut stdin_pipe("{\"files\":[\"a.rs\"]}")).unwrap();
    assert_eq!(v.get("files").unwrap()[0].as_str().unwrap(), "a.rs");
}

#[test]
fn file_input_and_enoent() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("in.json");
    std::fs::write(&p, "{\"q\":1}").unwrap();
    let mut s = stdin_tty();
    let v = read_task_input(p.to_str().unwrap(), &mut s).unwrap();
    assert_eq!(v.get("q").unwrap().as_i64().unwrap(), 1);

    let missing = dir.path().join("nope.json");
    let e = read_task_input(missing.to_str().unwrap(), &mut s).unwrap_err();
    assert_eq!(e.code(), ErrorCode::InvalidRequest);
    assert!(e.message().starts_with("cannot read input file: "));
    let d = e.details().unwrap();
    assert_eq!(
        d.get("input").unwrap().as_str().unwrap(),
        missing.to_str().unwrap()
    );
    assert!(d.get("cause").unwrap().is_string());
}

#[test]
fn count_max_bytes_parsing() {
    let get = |v: Option<&str>| {
        let f = move |_: &str| v.map(String::from);
        count_max_bytes(&f)
    };
    assert_eq!(get(Some("1024")), Some(1024));
    assert_eq!(get(Some(" 2048 ")), Some(2048)); // JS trim + Number
    assert_eq!(get(Some("0")), None); // 正の整数のみ
    assert_eq!(get(Some("-5")), None);
    assert_eq!(get(Some("16MB")), None); // parseInt 的な部分 parse 不可
    assert_eq!(get(Some("1.5")), None);
    assert_eq!(get(Some("abc")), None);
    assert_eq!(get(Some("")), None);
    assert_eq!(get(None), None);
    // safe integer 上限超過
    assert_eq!(get(Some("9007199254740993")), None);
    assert_eq!(get(Some("9007199254740991")), Some(9007199254740991));
}
