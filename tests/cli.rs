use serde_json::Value;
use std::{
    io::Write,
    process::{Command, Stdio},
};

fn cli(args: &[&str], input: &[u8]) -> (bool, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_t3dnyere"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let output = child.wait_with_output().unwrap();
    (
        output.status.success(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}
#[test]
fn cli_json_source_and_binary_stdin() {
    let doc = include_bytes!("../examples/bracket.t3d.json");
    let (ok, result) = cli(&["validate", "-"], doc);
    assert!(ok);
    assert_eq!(result["runtime_bytes"], 116);
    let bytes = t3dnyere::parse_document(doc)
        .unwrap()
        .compile()
        .unwrap()
        .to_bytes();
    let (ok, result) = cli(&["sample", "-", "0", "0", "0"], &bytes);
    assert!(ok);
    assert_eq!(result["inside"], true);
    let (ok, result) = cli(&["sample", "-", "NaN", "0", "0"], &bytes);
    assert!(!ok);
    assert_eq!(result["ok"], false);
    let (ok, result) = cli(&["schema"], &[]);
    assert!(ok);
    assert_eq!(result["properties"]["nodes"]["maxItems"], 4096);
}
#[test]
fn cli_bad_arguments_and_source_are_machine_readable() {
    for (args, input) in [
        (vec!["unknown"], &b""[..]),
        (vec!["validate", "-"], &b"not json"[..]),
    ] {
        let (ok, result) = cli(&args, input);
        assert!(!ok);
        assert_eq!(result["ok"], false);
        assert!(result["error"].is_string());
    }
}
