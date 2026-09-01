#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command as ProcessCommand, Stdio};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

use assert_cmd::Command;
use llmf::cli::HELP;
use serde_json::{Value, json};
use tempfile::TempDir;

struct MockServer {
    url: String,
    requests: Arc<Mutex<Vec<String>>>,
    handle: JoinHandle<()>,
    _server_guard: MutexGuard<'static, ()>,
}

fn local_server_guard() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

impl MockServer {
    fn chat(outputs: &[&str]) -> Self {
        let responses = outputs
            .iter()
            .map(|output| {
                (
                    200,
                    json!({"choices": [{"message": {"content": output}}]}).to_string(),
                )
            })
            .collect();
        Self::responses(responses)
    }

    fn responses(responses: Vec<(u16, String)>) -> Self {
        let server_guard = local_server_guard();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        let handle = std::thread::spawn(move || {
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_request(&mut stream);
                captured.lock().unwrap().push(request);
                let reason = if status == 200 { "OK" } else { "Error" };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.flush();
            }
        });
        Self {
            url: format!("http://{address}/v1/chat/completions"),
            requests,
            handle,
            _server_guard: server_guard,
        }
    }

    fn finish(self) -> Vec<String> {
        let Self {
            requests,
            handle,
            _server_guard,
            ..
        } = self;
        handle.join().unwrap();
        let requests = Arc::try_unwrap(requests).unwrap().into_inner().unwrap();
        drop(_server_guard);
        requests
    }
}

fn read_request(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 4096];
    let header_end = loop {
        let count = stream.read(&mut buffer).unwrap();
        assert!(count > 0, "connection ended before headers");
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(index) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            break index + 4;
        }
    };
    let headers = String::from_utf8_lossy(&bytes[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .unwrap();
    while bytes.len() < header_end + content_length {
        let count = stream.read(&mut buffer).unwrap();
        assert!(count > 0, "connection ended before body");
        bytes.extend_from_slice(&buffer[..count]);
    }
    String::from_utf8(bytes).unwrap()
}

fn request_json(request: &str) -> Value {
    let (_, body) = request.split_once("\r\n\r\n").unwrap();
    serde_json::from_str(body).unwrap()
}

struct RawServer {
    url: String,
    handle: JoinHandle<()>,
    _server_guard: MutexGuard<'static, ()>,
}

fn raw_server(response: Vec<u8>) -> RawServer {
    let server_guard = local_server_guard();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let _ = read_request(&mut stream);
        stream.write_all(&response).unwrap();
        stream.flush().unwrap();
    });
    RawServer {
        url: format!("http://{address}/v1/chat/completions"),
        handle,
        _server_guard: server_guard,
    }
}

fn write_workspace(root: &Path, api_url: &str) {
    write_workspace_with_key(root, api_url, "test-secret");
}

fn write_workspace_with_key(root: &Path, api_url: &str, api_key: &str) {
    let llmf = root.join(".llmf");
    fs::create_dir_all(llmf.join("prompts")).unwrap();
    fs::write(llmf.join("system_prompt.md"), "workspace system prompt").unwrap();
    fs::write(llmf.join("prompts/review.md"), "# Review prompt\n").unwrap();
    let config = json!({
        "api_url": api_url,
        "api_key": api_key,
        "model": "configured-model",
        "request_timeout_seconds": 5
    });
    fs::write(
        llmf.join("config.json"),
        serde_json::to_vec_pretty(&config).unwrap(),
    )
    .unwrap();
}

fn command(cwd: &Path, home: &Path) -> Command {
    let mut command = Command::cargo_bin("llmf").unwrap();
    command.current_dir(cwd).env("HOME", home);
    command
}

#[test]
fn help_is_a_standalone_unix_style_manual_page() {
    let temp = TempDir::new().unwrap();
    let mut command = Command::cargo_bin("llmf").unwrap();

    command
        .current_dir(temp.path())
        .env_remove("HOME")
        .arg("--help")
        .assert()
        .success()
        .stdout(HELP)
        .stderr("");
}

#[test]
fn init_creates_all_defaults() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();

    command(temp.path(), &home)
        .arg("--init")
        .assert()
        .success()
        .stderr("");

    for file in [
        "config.json",
        "system_prompt.md",
        "prompts/analyze.md",
        "prompts/clean-web.md",
        "prompts/commit-message.md",
        "prompts/compare-options.md",
        "prompts/customer-brief.md",
        "prompts/decision-record.md",
        "prompts/extract-failures.md",
        "prompts/handoff.md",
        "prompts/implement.md",
        "prompts/implementation-plan.md",
        "prompts/review.md",
        "prompts/review-diff.md",
        "prompts/safe-to-merge.md",
        "prompts/summarize.md",
        "prompts/translatetoen.md",
        "prompts/translatetojp.md",
        "prompts/translatetozh.md",
    ] {
        assert!(temp.path().join(".llmf").join(file).is_file(), "{file}");
    }
    command(temp.path(), &home)
        .arg("--init")
        .assert()
        .code(1)
        .stdout("")
        .stderr("llmf: .llmf already exists\n");
}

#[test]
fn first_run_initializes_user_workspace_without_api_request() {
    let temp = TempDir::new().unwrap();
    let cwd = temp.path().join("project");
    let home = temp.path().join("home");
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(&home).unwrap();

    command(&cwd, &home)
        .arg("review")
        .write_stdin("input")
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicates::str::contains("llmf: initialized"));
    assert!(home.join(".llmf/config.json").is_file());
    assert!(home.join(".llmf/system_prompt.md").is_file());
    assert!(home.join(".llmf/prompts/review.md").is_file());
}

#[test]
fn invalid_utf8_stdin_fails_without_an_api_request_or_stdout() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_workspace(temp.path(), "http://127.0.0.1:1");

    command(temp.path(), &home)
        .arg("review")
        .write_stdin(vec![b'a', 0xff])
        .assert()
        .code(70)
        .stdout("")
        .stderr("llmf: stdin must be valid UTF-8\n");
}

#[test]
fn empty_stdin_is_sent_as_an_empty_user_message() {
    let server = MockServer::chat(&["ok"]);
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_workspace(temp.path(), &server.url);

    command(temp.path(), &home)
        .arg("review")
        .write_stdin("")
        .assert()
        .success()
        .stdout("ok")
        .stderr("");
    let requests = server.finish();
    assert_eq!(request_json(&requests[0])["messages"][1]["content"], "");
}

#[test]
fn sensitive_api_url_components_are_rejected_without_leaking() {
    for api_url in [
        "https://name:super-secret@example.test/v1/chat/completions",
        "https://example.test/v1/chat/completions?token=super-secret",
        "https://example.test/v1/chat/completions#super-secret",
    ] {
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        fs::create_dir_all(&home).unwrap();
        write_workspace(temp.path(), api_url);

        let output = command(temp.path(), &home)
            .arg("review")
            .write_stdin("input")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(78));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.starts_with("llmf: invalid config: api_url"));
        assert!(!stderr.contains("super-secret"));
        assert!(!stderr.contains("example.test"));
    }
}

#[test]
fn invalid_config_fast_fails_with_code_78_and_clean_stdout() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(temp.path().join(".llmf/prompts")).unwrap();
    fs::write(temp.path().join(".llmf/prompts/review.md"), "prompt").unwrap();
    fs::write(temp.path().join(".llmf/config.json"), "{\"model\":}").unwrap();

    command(temp.path(), &home)
        .arg("review")
        .write_stdin("input")
        .assert()
        .code(78)
        .stdout("")
        .stderr(predicates::str::starts_with("llmf: invalid config:"));
}

#[test]
fn show_uses_project_prompt_then_falls_back_to_user_prompt() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(temp.path().join(".llmf/prompts")).unwrap();
    fs::create_dir_all(home.join(".llmf/prompts")).unwrap();
    fs::write(temp.path().join(".llmf/prompts/review.md"), "project").unwrap();
    fs::write(home.join(".llmf/prompts/review.md"), "user").unwrap();

    command(temp.path(), &home)
        .args(["--show", "review"])
        .assert()
        .success()
        .stdout("project")
        .stderr("");
    fs::remove_file(temp.path().join(".llmf/prompts/review.md")).unwrap();
    command(temp.path(), &home)
        .args(["--show", "review"])
        .assert()
        .success()
        .stdout("user")
        .stderr("");
}

#[test]
fn runtime_uses_project_system_prompt_then_falls_back_to_user() {
    let server = MockServer::chat(&["project", "user"]);
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    write_workspace(temp.path(), &server.url);
    write_workspace(&home, &server.url);
    fs::write(
        temp.path().join(".llmf/system_prompt.md"),
        "project system marker",
    )
    .unwrap();
    fs::write(home.join(".llmf/system_prompt.md"), "user system marker").unwrap();

    command(temp.path(), &home)
        .arg("review")
        .write_stdin("input")
        .assert()
        .success()
        .stdout("project")
        .stderr("");
    fs::remove_file(temp.path().join(".llmf/system_prompt.md")).unwrap();
    command(temp.path(), &home)
        .arg("review")
        .write_stdin("input")
        .assert()
        .success()
        .stdout("user")
        .stderr("");

    let requests = server.finish();
    let project_system = request_json(&requests[0])["messages"][0]["content"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(project_system.contains("project system marker"));
    assert!(!project_system.contains("user system marker"));
    let user_system = request_json(&requests[1])["messages"][0]["content"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(user_system.contains("user system marker"));
}

#[test]
fn missing_system_prompt_returns_66_without_an_api_request() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_workspace(temp.path(), "http://127.0.0.1:1");
    fs::remove_file(temp.path().join(".llmf/system_prompt.md")).unwrap();

    command(temp.path(), &home)
        .arg("review")
        .write_stdin("input")
        .assert()
        .code(66)
        .stdout("")
        .stderr("llmf: system prompt not found\n");
}

#[test]
fn list_merges_deduplicates_and_sorts_effective_prompts() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(temp.path().join(".llmf/prompts")).unwrap();
    fs::create_dir_all(home.join(".llmf/prompts")).unwrap();
    fs::write(temp.path().join(".llmf/prompts/z.md"), "z").unwrap();
    fs::write(temp.path().join(".llmf/prompts/a.md"), "project").unwrap();
    fs::write(home.join(".llmf/prompts/a.md"), "user").unwrap();
    fs::write(home.join(".llmf/prompts/m.md"), "m").unwrap();

    command(temp.path(), &home)
        .arg("--list")
        .assert()
        .success()
        .stdout("a\nm\nz\n")
        .stderr("");
}

#[test]
fn missing_prompt_returns_66_and_invalid_names_return_64() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(home.join(".llmf")).unwrap();

    command(temp.path(), &home)
        .args(["--show", "missing"])
        .assert()
        .code(66)
        .stdout("")
        .stderr("llmf: prompt not found: missing\n");
    command(temp.path(), &home)
        .arg("../review")
        .assert()
        .code(64)
        .stdout("")
        .stderr("llmf: invalid prompt name: ../review\n");
}

#[test]
fn project_without_config_falls_back_to_user_config() {
    let server = MockServer::chat(&["from user config"]);
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(temp.path().join(".llmf/prompts")).unwrap();
    fs::write(
        temp.path().join(".llmf/prompts/review.md"),
        "project prompt",
    )
    .unwrap();
    write_workspace(&home, &server.url);

    command(temp.path(), &home)
        .arg("review")
        .write_stdin("input")
        .assert()
        .success()
        .stdout("from user config")
        .stderr("");
    server.finish();
}

#[test]
fn single_request_returns_model_output_verbatim() {
    let server = MockServer::chat(&["# Result\n\nhello"]);
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_workspace(temp.path(), &server.url);

    command(temp.path(), &home)
        .arg("review")
        .env("MODEL", "wrong")
        .write_stdin("the input")
        .assert()
        .success()
        .stdout("# Result\n\nhello")
        .stderr("");

    let requests = server.finish();
    assert_eq!(requests.len(), 1);
    assert!(
        requests[0]
            .to_ascii_lowercase()
            .contains("authorization: bearer test-secret")
    );
    let first = request_json(&requests[0]);
    assert_eq!(first["model"], "configured-model");
    assert_eq!(first["stream"], false);
    assert_eq!(first["messages"][0]["role"], "system");
    assert!(
        first["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("# Review prompt")
    );
    assert!(
        first["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("workspace system prompt")
    );
    assert_eq!(
        first["messages"][1],
        json!({"role": "user", "content": "the input"})
    );
}

#[test]
fn empty_model_content_is_a_successful_verbatim_response() {
    let server = MockServer::chat(&[""]);
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_workspace(temp.path(), &server.url);

    command(temp.path(), &home)
        .arg("review")
        .write_stdin("input")
        .assert()
        .success()
        .stdout("")
        .stderr("");
    server.finish();
}

#[test]
fn model_output_is_not_normalized_or_given_a_newline() {
    let output = "line one\r\nline two\0世界";
    let server = MockServer::chat(&[output]);
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_workspace(temp.path(), &server.url);

    command(temp.path(), &home)
        .arg("review")
        .write_stdin("input")
        .assert()
        .success()
        .stdout(output)
        .stderr("");
    server.finish();
}

#[test]
fn command_like_model_output_is_never_executed() {
    let output = "SHELL\nprintf started > marker";
    let server = MockServer::chat(&[output]);
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_workspace(temp.path(), &server.url);

    command(temp.path(), &home)
        .arg("review")
        .write_stdin("input")
        .assert()
        .success()
        .stdout(output)
        .stderr("");

    assert!(!temp.path().join("marker").exists());
    let requests = server.finish();
    assert_eq!(requests.len(), 1);
}

#[test]
fn empty_api_key_omits_authorization_header() {
    let server = MockServer::chat(&["ok"]);
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_workspace_with_key(temp.path(), &server.url, "");

    command(temp.path(), &home)
        .arg("review")
        .write_stdin("input")
        .assert()
        .success()
        .stdout("ok")
        .stderr("");
    let requests = server.finish();
    assert!(!requests[0].to_ascii_lowercase().contains("authorization:"));
}

#[test]
fn final_stdout_can_be_used_as_the_next_filters_stdin() {
    let server = MockServer::chat(&["intermediate markdown", "final markdown"]);
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_workspace(temp.path(), &server.url);

    let first = command(temp.path(), &home)
        .arg("review")
        .write_stdin("initial")
        .output()
        .unwrap();
    assert!(first.status.success());
    assert!(first.stderr.is_empty());
    let second = command(temp.path(), &home)
        .arg("review")
        .write_stdin(first.stdout)
        .output()
        .unwrap();
    assert!(second.status.success());
    assert_eq!(second.stdout, b"final markdown");
    assert!(second.stderr.is_empty());

    let requests = server.finish();
    assert_eq!(
        request_json(&requests[1])["messages"][1]["content"],
        "intermediate markdown"
    );
}

#[test]
fn plain_model_output_requires_no_action_protocol() {
    let server = MockServer::chat(&["I think you should run git status."]);
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_workspace(temp.path(), &server.url);

    command(temp.path(), &home)
        .arg("review")
        .write_stdin("input")
        .assert()
        .success()
        .stdout("I think you should run git status.")
        .stderr("");
    server.finish();
}

#[test]
fn every_prompt_returns_plain_markdown_directly() {
    let server = MockServer::chat(&["# Summary\n\nPlain Markdown response."]);
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_workspace(temp.path(), &server.url);
    fs::write(
        temp.path().join(".llmf/prompts/summarize.md"),
        "# Summarize prompt\n",
    )
    .unwrap();

    command(temp.path(), &home)
        .arg("summarize")
        .write_stdin("input")
        .assert()
        .success()
        .stdout("# Summary\n\nPlain Markdown response.")
        .stderr("");
    server.finish();
}

#[test]
fn non_success_http_status_returns_74() {
    let server = MockServer::responses(vec![(500, "{}".into())]);
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_workspace(temp.path(), &server.url);

    command(temp.path(), &home)
        .arg("review")
        .write_stdin("input")
        .assert()
        .code(74)
        .stdout("")
        .stderr("llmf: API returned HTTP 500\n");
    server.finish();
}

#[test]
fn malformed_or_incomplete_api_json_returns_74() {
    for body in [
        "not-json",
        "{}",
        r#"{"choices":[]}"#,
        r#"{"choices":[{"message":{"content":null}}]}"#,
        r#"{"choices":[{"message":{"content":7}}]}"#,
    ] {
        let server = MockServer::responses(vec![(200, body.into())]);
        let temp = TempDir::new().unwrap();
        let home = temp.path().join("home");
        fs::create_dir_all(&home).unwrap();
        write_workspace(temp.path(), &server.url);

        command(temp.path(), &home)
            .arg("review")
            .write_stdin("input")
            .assert()
            .code(74)
            .stdout("")
            .stderr(predicates::str::starts_with("llmf: API"));
        server.finish();
    }
}

#[test]
fn first_choice_is_the_only_choice_used() {
    let server = MockServer::responses(vec![(
        200,
        r#"{"choices":[{"message":{"content":"first"}},{"message":{"content":"second"}}]}"#.into(),
    )]);
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_workspace(temp.path(), &server.url);

    command(temp.path(), &home)
        .arg("review")
        .write_stdin("input")
        .assert()
        .success()
        .stdout("first")
        .stderr("");
    server.finish();
}

#[test]
fn chunked_response_without_content_length_remains_bounded() {
    let body = "x".repeat(10 * 1024 * 1024 + 1);
    let response = format!(
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:X}\r\n{}\r\n0\r\n\r\n",
        body.len(), body
    )
    .into_bytes();
    let server = raw_server(response);
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_workspace(temp.path(), &server.url);

    command(temp.path(), &home)
        .arg("review")
        .write_stdin("input")
        .assert()
        .code(74)
        .stdout("")
        .stderr("llmf: API response body too large\n");
    server.handle.join().unwrap();
}

#[test]
fn ctrl_c_after_a_request_starts_returns_130_without_stdout() {
    let _server_guard = local_server_guard();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (request_started, request_started_rx) = mpsc::channel();
    let (send_response, send_response_rx) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let _ = read_request(&mut stream);
        request_started.send(()).unwrap();
        send_response_rx.recv().unwrap();
        let body = r#"{"choices":[{"message":{"content":"ok"}}]}"#;
        stream
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .unwrap();
        stream.flush().unwrap();
    });
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_workspace(
        temp.path(),
        &format!("http://{address}/v1/chat/completions"),
    );

    let mut child = ProcessCommand::new(assert_cmd::cargo::cargo_bin("llmf"))
        .current_dir(temp.path())
        .env("HOME", &home)
        .arg("review")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(b"input").unwrap();
    drop(child.stdin.take());
    request_started_rx
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGINT) };
    std::thread::sleep(Duration::from_millis(20));
    send_response.send(()).unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(130));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"llmf: interrupted\n");
    server.join().unwrap();
}

#[test]
fn oversized_api_response_returns_74() {
    let body = "x".repeat(10 * 1024 * 1024 + 1);
    let server = MockServer::responses(vec![(200, body)]);
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    write_workspace(temp.path(), &server.url);

    command(temp.path(), &home)
        .arg("review")
        .write_stdin("input")
        .assert()
        .code(74)
        .stdout("")
        .stderr("llmf: API response body too large\n");
    server.finish();
}
