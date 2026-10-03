use std::io::{Seek, SeekFrom, Write};
use std::process::{Command, Stdio};

fn helper(input: &[u8]) -> serde_json::Value {
    let mut source = tempfile::tempfile().unwrap();
    source.write_all(input).unwrap();
    source.seek(SeekFrom::Start(0)).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_zeron"))
        .arg("mermaid-render")
        // These would select a remote engine or initialize logging in a
        // regular launch. The helper must enter before either path.
        .env("NOCHES_CONNECTION", "nonexistent-mermaid-test-connection")
        .env("RUST_LOG", "trace")
        .stdin(Stdio::from(source))
        .output()
        .unwrap();
    assert!(result.status.success(), "{:?}", result.stderr);
    serde_json::from_slice(&result.stdout).expect("stdout must contain only the helper reply")
}

fn request(source: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "source": source,
        "palette": {
            "dark": false,
            "font": "Geist",
            "canvas": "#ffffff",
            "node": "#ffffff",
            "group": "#f8f8f8",
            "text": "#111111",
            "label": "#555555",
            "line": "#777777",
            "border": "#cccccc",
            "grid": "#eeeeee",
            "accent_line": "#9999ff",
            "accent_wash": "#ddddff"
        }
    }))
    .unwrap()
}

#[test]
fn application_helper_renders_svg_without_starting_an_engine_or_gui() {
    let reply = helper(&request("flowchart TD; A[Request] --> B[Done]"));
    let svg = reply["Ok"].as_str().expect("diagram should render");
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Request"));
    assert!(!svg.contains("<script"));
}

#[test]
fn application_helper_reports_malformed_and_oversized_source_as_errors() {
    for input in [
        request("not a diagram"),
        request(&"x".repeat(16 * 1024 + 1)),
        vec![b'x'; 128 * 1024 + 1],
    ] {
        let reply = helper(&input);
        assert!(reply["Err"].is_string(), "{reply}");
    }
}
