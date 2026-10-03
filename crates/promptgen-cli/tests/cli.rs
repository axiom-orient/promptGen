use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use promptgen_core::image::{CanvasPlacement, ImagePromptRequest};
use promptgen_core::json::parse;

static NEXT: AtomicU64 = AtomicU64::new(1);

#[test]
fn compiles_image_prompt_examples() {
    let root = repository_root();
    for (command, example, marker) in [
        ("image", "image-photo-lut.json", "warm_pastel_filmic"),
        ("image", "image-typography-poster.json", "text_element_1"),
        ("image", "image-pose-transfer.json", "자세 충실도 계약"),
        (
            "image",
            "image-editorial-tier2.json",
            "all depicted people are adults aged 25+",
        ),
    ] {
        let output = Command::new(binary())
            .args([command, "--input"])
            .arg(root.join("examples").join(example))
            .args(["--format", "json"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("\"status\": \"valid"));
        assert!(stdout.contains(marker));
    }
}

#[test]
fn fidelity_attempt_limit_is_bounded_and_scoped_to_codex_execution() {
    let root = repository_root();
    let input = root.join("examples/image-app-icon.json");
    let invalid_limit = Command::new(binary())
        .args(["image", "--input"])
        .arg(&input)
        .args(["--mode", "codex-imagegen", "--max-fidelity-attempts", "5"])
        .output()
        .unwrap();
    assert_eq!(invalid_limit.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&invalid_limit.stderr)
            .contains("--max-fidelity-attempts must be an integer in 1..=4")
    );

    let prompt_only = Command::new(binary())
        .args(["image", "--input"])
        .arg(input)
        .args(["--mode", "prompt-only", "--max-fidelity-attempts", "1"])
        .output()
        .unwrap();
    assert_eq!(prompt_only.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&prompt_only.stderr)
            .contains("--max-fidelity-attempts is valid only with --mode codex-imagegen")
    );
}

#[cfg(unix)]
#[test]
fn codex_binary_digest_mismatch_fails_before_review_or_image_dispatch() {
    use std::os::unix::fs::PermissionsExt;

    let root = test_root("codex-pin-before-dispatch");
    let codex = root.join("codex-shim");
    fs::write(
        &codex,
        b"#!/bin/sh\necho invoked >> \"$CODEX_HOME/invocations\"\n",
    )
    .unwrap();
    fs::set_permissions(&codex, fs::Permissions::from_mode(0o700)).unwrap();
    let codex_home = root.join("codex-home-must-not-be-created");
    let image_output = root.join("output.png");
    let result = root.join("receipt.json");
    let wrong_sha256 = "0".repeat(64);
    let output = Command::new(binary())
        .args(["image", "--input"])
        .arg(repository_root().join("examples/image-app-icon.json"))
        .args(["--mode", "codex-imagegen", "--codex-binary"])
        .arg(&codex)
        .args(["--codex-sha256", wrong_sha256.as_str(), "--codex-home"])
        .arg(&codex_home)
        .arg("--image-output")
        .arg(&image_output)
        .arg("--result")
        .arg(&result)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&output.stderr).contains("CODEX_BINARY_IDENTITY"));
    assert!(!codex_home.exists());
    assert!(!image_output.exists());
    assert!(!result.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn strict_unknown_field_is_exit_three() {
    let root = test_root("unknown");
    let input = root.join("bad.json");
    let source =
        fs::read_to_string(repository_root().join("examples/image-photo-lut.json")).unwrap();
    fs::write(
        &input,
        source.replacen(
            "\"language\": \"ko\",",
            "\"language\": \"ko\",\n  \"unknown\": true,",
            1,
        ),
    )
    .unwrap();
    let output = Command::new(binary())
        .args(["image", "--input"])
        .arg(input)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown field"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn catalog_has_six_representative_outcomes() {
    let output = Command::new(binary())
        .args(["catalog", "--format", "text"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().lines().count(), 6);
}

#[test]
fn output_is_not_overwritten_without_force() {
    let root = test_root("overwrite");
    let destination = root.join("result.json");
    fs::write(&destination, "sentinel").unwrap();
    let output = Command::new(binary())
        .args(["schema", "image", "--result"])
        .arg(&destination)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(fs::read_to_string(&destination).unwrap(), "sentinel");
    let output = Command::new(binary())
        .args(["schema", "image", "--result"])
        .arg(&destination)
        .arg("--force")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        fs::read_to_string(&destination)
            .unwrap()
            .contains("codex-subscription")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn guided_editorial_interview_asks_only_missing_fields_then_compiles() {
    let root = test_root("guided-editorial");
    let first = root.join("first.json");
    fs::write(
        &first,
        r#"{"kind":"image","brief":"30대 성인 모델 1명의 절제된 리조트웨어 세로 화보, 골든아워 해안 테라스.","answers":{}}"#,
    )
    .unwrap();
    let output = Command::new(binary())
        .args(["interview", "--input"])
        .arg(&first)
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("\"status\": \"needs_input\""));
    assert!(text.contains("image.wardrobe"));
    assert!(text.contains("30대 성인 모델 1명"));

    let second = root.join("second.json");
    fs::write(
        &second,
        r#"{"kind":"image","brief":"30대 성인 모델 1명의 절제된 리조트웨어 세로 화보, 골든아워 해안 테라스.","answers":{"image.wardrobe":"세이지색 불투명 리넨 테일러드 재킷, 높은 라운드넥 크림 이너, 발목 길이 와이드 팬츠, 여유로운 핏"}}"#,
    )
    .unwrap();
    let output = Command::new(binary())
        .args(["interview", "--input"])
        .arg(&second)
        .args(["--format", "json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("\"status\": \"ready\""));
    assert!(text.contains("\"render_profile\": \"structured\""));
    assert!(text.contains("세이지색 불투명 리넨"));
    assert!(!text.contains("IMG_EDITORIAL_FLAT_LENGTH"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn mcp_stdio_external_agent_interview_retries_questions_and_returns_prompt() {
    let brief = "30대 성인 모델 1명의 절제된 리조트웨어 세로 화보, 골든아워 해안 테라스.";
    let metadata = r#""_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"promptgen-cli-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}}"#;
    let requests = [
        format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{{{metadata}}}}}"#
        ),
        format!(
            r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{{metadata},"name":"promptgen_interview","arguments":{{"kind":"image","brief":"{brief}","answers":{{}}}}}}}}"#
        ),
        format!(
            r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{{metadata},"name":"promptgen_interview","arguments":{{"kind":"image","brief":"{brief}","answers":{{"image.wardrobe":"세이지색 불투명 리넨 테일러드 재킷, 높은 라운드넥 크림 이너, 발목 길이 와이드 팬츠, 여유로운 핏"}}}}}}}}"#
        ),
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientInfo":{"name":"promptgen-cli-test","version":"1"},"io.modelcontextprotocol/clientCapabilities":{}},"name":"promptgen_interview","arguments":{"kind":"audio","brief":"not an image","answers":{}}}}"#.to_owned(),
    ];

    let mut child = Command::new(binary())
        .args(["mcp", "--transport", "stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start MCP stdio server");
    {
        let stdin = child.stdin.as_mut().expect("MCP stdin");
        for request in requests {
            writeln!(stdin, "{request}").expect("write MCP request");
        }
    }
    let output = child.wait_with_output().expect("wait for MCP stdio server");
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let responses = String::from_utf8(output.stdout).expect("MCP responses are UTF-8");
    let lines = responses.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 4, "responses={responses}");
    let discover = lines
        .iter()
        .find(|line| line.contains("\"id\":1"))
        .expect("discover response");
    let needs_input = lines
        .iter()
        .find(|line| line.contains("\"id\":2"))
        .expect("needs_input response");
    let ready = lines
        .iter()
        .find(|line| line.contains("\"id\":3"))
        .expect("ready response");
    let invalid = lines
        .iter()
        .find(|line| line.contains("\"id\":4"))
        .expect("invalid response");
    assert!(discover.contains("\"supportedVersions\":[\"2026-07-28\"]"));
    assert!(needs_input.contains("\"status\":\"needs_input\""));
    assert!(needs_input.contains("\"id\":\"image.wardrobe\""));
    assert!(ready.contains("\"status\":\"ready\""));
    assert!(ready.contains("\"compilation\":{") && ready.contains("\"prompt\":\""));
    assert!(ready.contains("세이지색 불투명 리넨"));
    assert!(invalid.contains("\"error\":{") && invalid.contains("expected image"));
}

#[test]
fn interview_schema_is_exposed_by_cli() {
    let output = Command::new(binary())
        .args(["schema", "interview"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("promptGen guided interview request"));
    assert!(text.contains("\"answers\""));
}

#[test]
fn serve_refuses_non_loopback_bind() {
    let output = Command::new(binary())
        .args(["serve", "--bind", "0.0.0.0:4173"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&output.stderr).contains("WEB_BIND_NOT_LOOPBACK"));
}

#[test]
fn image_prompt_only_rejects_image_output() {
    let root = repository_root();
    let output = Command::new(binary())
        .args(["image", "--input"])
        .arg(root.join("examples/image-photo-lut.json"))
        .args(["--image-output", "unused.png"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn duplicate_options_and_removed_output_option_are_rejected() {
    let root = repository_root();
    let output = Command::new(binary())
        .args(["image", "--input"])
        .arg(root.join("examples/image-photo-lut.json"))
        .args(["--format", "json", "--format", "text"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("provided more than once"));

    let temp = test_root("duplicate-alias");
    let first = temp.join("first.json");
    let second = temp.join("second.json");
    let output = Command::new(binary())
        .args(["schema", "image", "--result"])
        .arg(&first)
        .arg("--output")
        .arg(&second)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown option \"--output\""));
    assert!(!first.exists());
    assert!(!second.exists());
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn image_timeout_bounds_are_fail_closed() {
    let root = repository_root();
    for (option, value, maximum) in [
        ("--timeout-seconds", "3601", "3600"),
        ("--artifact-wait-seconds", "601", "600"),
        ("--timeout-seconds", "0", "3600"),
    ] {
        let output = Command::new(binary())
            .args(["image", "--input"])
            .arg(root.join("examples/image-photo-lut.json"))
            .args([option, value])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(2),
            "option={option}, value={value}"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(maximum), "stderr={stderr}");
    }
}

#[cfg(unix)]
#[test]
fn codex_mode_rejects_result_image_aliases_before_provider_dispatch() {
    let repository = repository_root();
    let root = test_root("cli-codex-alias");
    let real_parent = root.join("real-parent");
    let alias_parent = root.join("alias-parent");
    fs::create_dir(&real_parent).unwrap();
    std::os::unix::fs::symlink(&real_parent, &alias_parent).unwrap();

    let run = |image_output: &std::path::Path, result: &std::path::Path| {
        Command::new(binary())
            .args(["image", "--input"])
            .arg(repository.join("examples/image-photo-lut.json"))
            .args(["--mode", "codex-imagegen", "--codex-binary"])
            .arg(root.join("provider-must-not-run"))
            .arg("--codex-home")
            .arg(root.join("codex-home-must-not-exist"))
            .arg("--image-output")
            .arg(image_output)
            .arg("--result")
            .arg(result)
            .arg("--force")
            .output()
            .unwrap()
    };

    let image_output = real_parent.join("generated.png");
    let result_output = alias_parent.join("generated.png");
    let output = run(&image_output, &result_output);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--result and --image-output"));
    assert!(!root.join("codex-home-must-not-exist").exists());
    assert!(!image_output.exists());

    let first = real_parent.join("published.png");
    let second = root.join("same-inode.json");
    fs::write(&first, b"sentinel").unwrap();
    fs::hard_link(&first, &second).unwrap();
    let output = run(&first, &second);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--result and --image-output"));
    assert_eq!(fs::read(&first).unwrap(), b"sentinel");
    assert!(!root.join("codex-home-must-not-exist").exists());

    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn codex_mode_cli_publishes_verified_thread_artifact() {
    use std::os::unix::fs::PermissionsExt;

    let repository = repository_root();
    let root = test_root("cli-codex-success");
    let codex_home = root.join("codex-home");
    fs::create_dir_all(&codex_home).unwrap();
    let image_output = root.join("output/generated.png");
    let result_output = root.join("output/receipt.json");
    let fixture = repository.join("crates/promptgen-cli/tests/fixtures/valid-1024x1536.png");
    let script = root.join("fake-codex.sh");
    let input = root.join("input.json");
    let mut request = ImagePromptRequest::from_json(
        parse(
            &fs::read_to_string(repository.join("examples/image-photo-lut.json"))
                .expect("read example"),
        )
        .expect("parse example"),
    )
    .expect("decode example");
    for subject in &mut request.subjects {
        subject.placement = CanvasPlacement::Custom {
            x_percent: 0,
            y_percent: 0,
            width_percent: 100,
            height_percent: 100,
        };
    }
    fs::write(&input, request.to_json().to_pretty_string()).expect("write test input");
    let body = r##"#!/bin/sh
set -eu
if [ "${1:-}" = "--version" ]; then echo 'codex-cli integration-test'; exit 0; fi
model=''
schema=''
result=''
previous=''
for argument in "$@"; do
  case "$previous" in
    --model) model="$argument" ;;
    --output-schema) schema="$argument" ;;
    --output-last-message) result="$argument" ;;
  esac
  previous="$argument"
done
if [ "$model" = 'gpt-5.6-luna' ]; then
  [ -f "$schema" ]
  cat > "$CODEX_HOME/luna-review-input.txt"
  printf '%s\n' '{"approved":true,"summary":"The visual contract is coherent.","additions":["Clarify the subject edge against the background."]}' > "$result"
  exit 0
fi
if [ -n "$result" ]; then
  printf '%s\n' '{"pass":true,"summary":"all hard constraints pass","subject_counts":[{"id":"coffee_cup","expected":1,"observed":1,"evidence":"one cup","instances":[{"x_percent":50,"y_percent":50,"evidence":"cup"}]},{"id":"beans","expected":9,"observed":9,"evidence":"nine beans","instances":[{"x_percent":10,"y_percent":80,"evidence":"1"},{"x_percent":20,"y_percent":80,"evidence":"2"},{"x_percent":30,"y_percent":80,"evidence":"3"},{"x_percent":40,"y_percent":80,"evidence":"4"},{"x_percent":50,"y_percent":80,"evidence":"5"},{"x_percent":60,"y_percent":80,"evidence":"6"},{"x_percent":70,"y_percent":80,"evidence":"7"},{"x_percent":80,"y_percent":80,"evidence":"8"},{"x_percent":90,"y_percent":80,"evidence":"9"}]}],"text_checks":[],"violations":[],"repair_instruction":""}' > "$result"
  printf '%s\n' '{"type":"thread.started","thread_id":"thread-validator"}' '{"type":"turn.completed"}'
  exit 0
fi
mkdir -p "$CODEX_HOME/generated_images/thread-cli"
cp "$FAKE_FIXTURE" "$CODEX_HOME/generated_images/thread-cli/call-cli.png"
printf '%s\n' '{"type":"thread.started","thread_id":"thread-cli"}' '{"type":"turn.completed"}'
"##;
    fs::write(&script, body).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    let output = Command::new(binary())
        .args(["image", "--input"])
        .arg(&input)
        .args(["--mode", "codex-imagegen", "--codex-binary"])
        .arg(&script)
        .arg("--codex-home")
        .arg(&codex_home)
        .arg("--image-output")
        .arg(&image_output)
        .arg("--result")
        .arg(&result_output)
        .args(["--timeout-seconds", "5", "--artifact-wait-seconds", "5"])
        .env("FAKE_FIXTURE", &fixture)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(&image_output).unwrap(),
        fs::read(&fixture).unwrap()
    );
    let receipt = fs::read_to_string(&result_output).unwrap();
    assert!(receipt.contains("\"thread_id\": \"thread-cli\""));
    assert!(receipt.contains("\"image_call_id\": \"call-cli\""));
    assert!(receipt.contains("\"width\": 1024"));
    assert!(receipt.contains("\"height\": 1536"));
    assert!(receipt.contains("\"fidelity_checks\""));
    assert!(receipt.contains("\"model\": \"gpt-5.6-luna\""));
    assert!(receipt.contains("\"prompt_refinement\""));
    assert!(receipt.contains("\"executed_prompt_sha256\""));
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn subscription_edit_snapshots_base_and_binds_receipt() {
    use std::os::unix::fs::PermissionsExt;

    let repository = repository_root();
    let root = test_root("cli-codex-edit");
    let codex_home = root.join("codex-home");
    fs::create_dir_all(&codex_home).unwrap();
    let image_output = root.join("output/generated.png");
    let result_output = root.join("output/receipt.json");
    let fixture = repository.join("crates/promptgen-cli/tests/fixtures/valid-1024x1536.png");
    let script = root.join("fake-codex.sh");
    let input = root.join("input.json");
    let mut request = ImagePromptRequest::from_json(
        parse(
            &fs::read_to_string(repository.join("examples/image-edit-product.json"))
                .expect("read example"),
        )
        .expect("parse example"),
    )
    .expect("decode example");
    for subject in &mut request.subjects {
        subject.placement = CanvasPlacement::Custom {
            x_percent: 0,
            y_percent: 0,
            width_percent: 100,
            height_percent: 100,
        };
    }
    fs::write(&input, request.to_json().to_pretty_string()).expect("write test input");
    let reference = root.join("original.png");
    fs::copy(&fixture, &reference).unwrap();
    let before = fs::read(&reference).unwrap();
    let body = r##"#!/bin/sh
set -eu
if [ "${1:-}" = "--version" ]; then echo 'codex-cli integration-test'; exit 0; fi
model=''
schema=''
result=''
previous=''
image=''
for argument in "$@"; do
  case "$previous" in
    --image) image="$argument" ;;
    --model) model="$argument" ;;
    --output-schema) schema="$argument" ;;
    --output-last-message) result="$argument" ;;
  esac
  previous="$argument"
done
if [ "$model" = 'gpt-5.6-luna' ]; then
  [ -f "$schema" ]
  cat > "$CODEX_HOME/luna-review-input.txt"
  printf '%s\n' '{"approved":true,"summary":"The visual contract is coherent.","additions":["Clarify the subject edge against the background."]}' > "$result"
  exit 0
fi
if [ -n "$result" ]; then
  printf '%s\n' '{"pass":true,"summary":"all hard constraints pass","subject_counts":[{"id":"coffee_cup","expected":1,"observed":1,"evidence":"one cup","instances":[{"x_percent":50,"y_percent":50,"evidence":"cup"}]},{"id":"beans","expected":9,"observed":9,"evidence":"nine beans","instances":[{"x_percent":10,"y_percent":80,"evidence":"1"},{"x_percent":20,"y_percent":80,"evidence":"2"},{"x_percent":30,"y_percent":80,"evidence":"3"},{"x_percent":40,"y_percent":80,"evidence":"4"},{"x_percent":50,"y_percent":80,"evidence":"5"},{"x_percent":60,"y_percent":80,"evidence":"6"},{"x_percent":70,"y_percent":80,"evidence":"7"},{"x_percent":80,"y_percent":80,"evidence":"8"},{"x_percent":90,"y_percent":80,"evidence":"9"}]}],"text_checks":[],"violations":[],"repair_instruction":""}' > "$result"
  printf '%s\n' '{"type":"thread.started","thread_id":"thread-validator"}' '{"type":"turn.completed"}'
  exit 0
fi
cat > "$CODEX_HOME/generation-input.txt"
[ "$(basename "$image")" = "base-reference.png" ]
cmp "$image" "$FAKE_FIXTURE"
mkdir -p "$CODEX_HOME/generated_images/thread-cli"
cp "$FAKE_FIXTURE" "$CODEX_HOME/generated_images/thread-cli/call-cli.png"
printf '%s\n' '{"type":"thread.started","thread_id":"thread-cli"}' '{"type":"turn.completed"}'
"##;
    fs::write(&script, body).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    let output = Command::new(binary())
        .args(["image", "--input"])
        .arg(&input)
        .arg("--reference-image")
        .arg(&reference)
        .args(["--mode", "codex-imagegen", "--codex-binary"])
        .arg(&script)
        .arg("--codex-home")
        .arg(&codex_home)
        .arg("--image-output")
        .arg(&image_output)
        .arg("--result")
        .arg(&result_output)
        .args(["--timeout-seconds", "5", "--artifact-wait-seconds", "5"])
        .env("FAKE_FIXTURE", &fixture)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(&image_output).unwrap(),
        fs::read(&fixture).unwrap()
    );
    let receipt = fs::read_to_string(&result_output).unwrap();
    assert!(receipt.contains("\"thread_id\": \"thread-cli\""));
    assert!(receipt.contains("\"image_call_id\": \"call-cli\""));
    assert!(receipt.contains("\"width\": 1024"));
    assert!(receipt.contains("\"height\": 1536"));
    assert!(receipt.contains("\"fidelity_checks\""));
    assert!(receipt.contains("\"model\": \"gpt-5.6-luna\""));
    assert!(receipt.contains("\"prompt_refinement\""));
    assert!(receipt.contains("\"executed_prompt_sha256\""));
    assert_eq!(fs::read(&reference).unwrap(), before);
    let generation_input = fs::read_to_string(codex_home.join("generation-input.txt")).unwrap();
    assert!(generation_input.contains("referenced_image_paths="));
    assert!(generation_input.contains("base-reference.png"));
    assert!(!generation_input.contains(reference.to_string_lossy().as_ref()));
    assert!(receipt.contains("\"reference_sha256\""));
    assert!(receipt.contains("\"task_mode\": \"edit\""));
    assert!(receipt.contains("chatgpt-subscription"));
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn subscription_edit_requires_base_before_calling_luna() {
    let repository = repository_root();
    let root = test_root("cli-codex-missing-base");
    let fixture = repository.join("crates/promptgen-cli/tests/fixtures/valid-1024x1536.png");
    let reference = root.join("source-reference.png");
    fs::copy(&fixture, &reference).unwrap();
    let codex_home = root.join("codex-home-must-not-be-created");
    let output_path = root.join("output/generated.png");
    let output = Command::new(binary())
        .args(["image", "--input"])
        .arg(repository.join("examples/image-edit-product.json"))
        .args(["--mode", "codex-imagegen", "--image-output"])
        .arg(&output_path)
        .arg("--codex-binary")
        .arg(root.join("codex-must-not-run"))
        .arg("--codex-home")
        .arg(&codex_home)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&output.stderr).contains("CODEX_REFERENCE_INPUTS_REQUIRED"));
    assert!(!codex_home.exists());
    assert!(!output_path.exists());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn subscription_edit_rejects_reference_and_result_aliases_before_provider() {
    let repository = repository_root();
    let root = test_root("cli-codex-alias");
    let (fake_codex, codex_home) = fake_luna_codex(&root);
    let fixture = repository.join("crates/promptgen-cli/tests/fixtures/valid-1024x1536.png");
    let valid_input = repository.join("examples/image-edit-product.json");
    let invalid_input = root.join("invalid-edit.json");
    let invalid_source = fs::read_to_string(&valid_input).unwrap().replacen(
        "\"width\": 1024,",
        "\"width\": 3841,",
        1,
    );
    fs::write(&invalid_input, invalid_source).unwrap();
    let reference = root.join("source-reference.png");
    fs::copy(&fixture, &reference).unwrap();
    let before = fs::read(&reference).unwrap();
    let image_output = root.join("output/generated.png");
    let reference_output = root.join("nested/../source-reference.png");
    let result_output = root.join("nested/../output/generated.png");

    let run = |input: &PathBuf,
               image: &PathBuf,
               reference_path: &PathBuf,
               result: Option<&PathBuf>,
               force: bool| {
        let mut command = Command::new(binary());
        command
            .args(["image", "--input"])
            .arg(input)
            .args(["--mode", "codex-imagegen", "--image-output"])
            .arg(image)
            .args(["--reference-image"])
            .arg(reference_path)
            .arg("--codex-binary")
            .arg(&fake_codex)
            .arg("--codex-home")
            .arg(&codex_home);
        if let Some(result) = result {
            command.args(["--result"]).arg(result);
        }
        if force {
            command.arg("--force");
        }
        command.output().unwrap()
    };

    for force in [false, true] {
        let output = run(&valid_input, &reference_output, &reference, None, force);
        assert_eq!(output.status.code(), Some(2), "force={force}");
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("--reference-image and --image-output"),
            "force={force}, stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read(&reference).unwrap(), before, "force={force}");
        assert!(
            !codex_home.join("luna-review-called").exists(),
            "force={force}"
        );
        assert!(!reference_output.exists(), "force={force}");
    }

    for force in [false, true] {
        let output = run(
            &valid_input,
            &image_output,
            &reference,
            Some(&reference_output),
            force,
        );
        assert_eq!(output.status.code(), Some(2), "force={force}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("--result and --reference-image"),
            "force={force}, stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read(&reference).unwrap(), before, "force={force}");
        assert!(
            !codex_home.join("luna-review-called").exists(),
            "force={force}"
        );
        assert!(!image_output.exists(), "force={force}");
    }

    for force in [false, true] {
        let output = run(
            &valid_input,
            &image_output,
            &reference,
            Some(&result_output),
            force,
        );
        assert_eq!(output.status.code(), Some(2), "force={force}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("--result and --image-output"),
            "force={force}, stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read(&reference).unwrap(), before, "force={force}");
        assert!(
            !codex_home.join("luna-review-called").exists(),
            "force={force}"
        );
        assert!(!image_output.exists(), "force={force}");
    }

    let invalid_image_output = root.join("output/invalid-generated.png");
    let output = run(
        &invalid_input,
        &invalid_image_output,
        &reference,
        Some(&reference_output),
        true,
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--result and --reference-image"),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read(&reference).unwrap(), before);
    assert!(!codex_home.join("luna-review-called").exists());
    assert!(!invalid_image_output.exists());

    let real_parent = root.join("real-parent");
    fs::create_dir(&real_parent).unwrap();
    let alias_parent = root.join("alias-parent");
    std::os::unix::fs::symlink(&real_parent, &alias_parent).unwrap();
    let symlink_reference = real_parent.join("source.png");
    fs::copy(&fixture, &symlink_reference).unwrap();
    let symlink_reference_alias = alias_parent.join("source.png");
    let symlink_image_output = real_parent.join("generated.png");
    let symlink_result_output = alias_parent.join("generated.png");
    let symlink_before = fs::read(&symlink_reference).unwrap();

    let output = run(
        &valid_input,
        &symlink_reference_alias,
        &symlink_reference,
        None,
        true,
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--reference-image and --image-output")
    );
    assert_eq!(fs::read(&symlink_reference).unwrap(), symlink_before);
    assert!(!codex_home.join("luna-review-called").exists());
    assert!(!image_output.exists());

    let symlink_distinct_output = root.join("output/symlink-distinct.png");
    let output = run(
        &valid_input,
        &symlink_distinct_output,
        &symlink_reference,
        Some(&symlink_reference_alias),
        true,
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--result and --reference-image"));
    assert_eq!(fs::read(&symlink_reference).unwrap(), symlink_before);
    assert!(!codex_home.join("luna-review-called").exists());
    assert!(!symlink_distinct_output.exists());

    let output = run(
        &valid_input,
        &symlink_image_output,
        &symlink_reference,
        Some(&symlink_result_output),
        true,
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--result and --image-output"));
    assert_eq!(fs::read(&symlink_reference).unwrap(), symlink_before);
    assert!(!codex_home.join("luna-review-called").exists());
    assert!(!symlink_image_output.exists());

    let symlink_invalid_output = root.join("output/symlink-invalid.png");
    let output = run(
        &invalid_input,
        &symlink_invalid_output,
        &symlink_reference,
        Some(&symlink_reference_alias),
        true,
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--result and --reference-image"));
    assert_eq!(fs::read(&symlink_reference).unwrap(), symlink_before);
    assert!(!codex_home.join("luna-review-called").exists());
    assert!(!symlink_invalid_output.exists());

    fs::remove_dir_all(root).unwrap();
}

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_promptgen"))
}

#[cfg(unix)]
fn fake_luna_codex(root: &Path) -> (PathBuf, PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    let codex_home = root.join("codex-home");
    fs::create_dir_all(&codex_home).unwrap();
    let script = root.join("fake-luna-codex.sh");
    fs::write(
        &script,
        r##"#!/bin/sh
set -eu
model=''
schema=''
result=''
shell_disabled='false'
previous=''
for argument in "$@"; do
  case "$previous" in
    --model) model="$argument" ;;
    --output-schema) schema="$argument" ;;
    --output-last-message) result="$argument" ;;
    --disable) [ "$argument" != 'shell_tool' ] || shell_disabled='true' ;;
  esac
  previous="$argument"
done
[ "$model" = 'gpt-5.6-luna' ]
[ "$shell_disabled" = 'true' ]
[ -f "$schema" ]
[ -n "$result" ]
cat > /dev/null
printf '%s\n' 'called' > "$CODEX_HOME/luna-review-called"
if [ "${FAKE_LUNA_REJECT:-}" = 'true' ]; then
  printf '%s\n' '{"approved":false,"summary":"The canonical prompt contains a material contradiction.","additions":[]}' > "$result"
  exit 0
fi
printf '%s\n' '{"approved":true,"summary":"The visual contract is coherent.","additions":["Clarify the subject edge against the background."]}' > "$result"
"##,
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    (script, codex_home)
}

#[cfg(unix)]
#[test]
fn codex_image_mode_stops_when_luna_rejects_the_prompt() {
    let repository = repository_root();
    let root = test_root("cli-luna-rejected");
    let (fake_codex, codex_home) = fake_luna_codex(&root);
    let image_output = root.join("generated.png");
    let output = Command::new(binary())
        .args(["image", "--input"])
        .arg(repository.join("examples/image-photo-lut.json"))
        .args(["--mode", "codex-imagegen", "--image-output"])
        .arg(&image_output)
        .arg("--codex-binary")
        .arg(&fake_codex)
        .arg("--codex-home")
        .arg(&codex_home)
        .env("FAKE_LUNA_REJECT", "true")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&output.stderr).contains("CODEX_LUNA_REVIEW_REJECTED"));
    assert!(!image_output.exists());
    assert!(!codex_home.join("generated_images").exists());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn luna_refine_mode_returns_a_prompt_bound_review_without_image_execution() {
    let repository = repository_root();
    let root = test_root("cli-luna-refine");
    let (fake_codex, codex_home) = fake_luna_codex(&root);
    let result = root.join("refinement.json");
    let output = Command::new(binary())
        .args(["image", "--input"])
        .arg(repository.join("examples/image-photo-lut.json"))
        .args(["--mode", "luna-refine", "--codex-binary"])
        .arg(&fake_codex)
        .arg("--codex-home")
        .arg(&codex_home)
        .arg("--result")
        .arg(&result)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let receipt = fs::read_to_string(result).unwrap();
    assert!(receipt.contains("\"model\": \"gpt-5.6-luna\""));
    assert!(receipt.contains("\"provider\": \"codex-cli\""));
    assert!(receipt.contains("\"source_prompt_sha256\""));
    assert!(receipt.contains("\"refined_prompt_sha256\""));
    assert!(receipt.contains("Clarify the subject edge"));
    assert!(codex_home.join("luna-review-called").exists());
    assert!(!root.join("generated.png").exists());
    fs::remove_dir_all(root).unwrap();
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn test_root(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "promptgen-cli-test-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn subcommand_help_is_clean_success() {
    for args in [
        vec!["image", "--help"],
        vec!["interview", "--help"],
        vec!["serve", "--help"],
        vec!["schema", "--help"],
        vec!["catalog", "--help"],
        vec!["lut-presets", "--help"],
    ] {
        let output = Command::new(binary())
            .args(&args)
            .output()
            .expect("help command should run");
        assert!(output.status.success(), "args={args:?}");
        assert!(
            output.stderr.is_empty(),
            "args={args:?}, stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!output.stdout.is_empty(), "args={args:?}");
    }
}
