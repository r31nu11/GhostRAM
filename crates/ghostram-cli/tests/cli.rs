//! CLI integration tests. They use the real binary but do not require a GPU,
//! Vulkan device, engine, model, root access, or network service.

use std::process::{Command, Output};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_ghostram")
}

fn run(arguments: &[&str]) -> Output {
    Command::new(binary())
        .args(arguments)
        .output()
        .expect("ghostram binary should start")
}

fn run_without_path(arguments: &[&str]) -> Output {
    Command::new(binary())
        .args(arguments)
        .env("PATH", "/nonexistent")
        .output()
        .expect("ghostram binary should start")
}

#[test]
fn version_is_available() {
    let output = run(&["--version"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("ghostram "));
    assert!(output.stderr.is_empty());
}

#[test]
fn help_lists_the_doctor_command() {
    let output = run(&["--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("USAGE:"));
    assert!(stdout.contains("ghostram doctor"));
}

#[test]
fn invalid_arguments_return_usage_exit_code() {
    let output = run(&["doctor", "--not-a-real-option"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("unexpected argument"));
}

#[test]
fn doctor_json_is_valid_and_clean() {
    let output = run(&["doctor", "--json"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.trim_start().starts_with('{'));
    assert!(!stdout.contains("GhostRAM doctor"));
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(value["schema_version"], "ghostram.doctor/1");
    assert_eq!(value["application_version"], env!("CARGO_PKG_VERSION"));
    assert!(value["generated_at"].as_str().unwrap().ends_with('Z'));
    assert!(value["report"]["categories"]["os"].is_object());
    assert!(value["report"]["system"].is_object());
}

#[test]
fn doctor_completes_when_optional_tools_are_missing() {
    let output = run_without_path(&["doctor"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Overall: partial"));
    assert!(stdout.contains("nvidia_gpu: unavailable"));
    assert!(stdout.contains("no llama.cpp executables found on PATH"));
}

#[test]
fn partial_json_report_is_well_formed() {
    let output = run_without_path(&["doctor", "--json"]);
    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be JSON only");
    assert_eq!(
        value["report"]["categories"]["nvidia_gpu"]["state"],
        "unavailable"
    );
    assert!(
        value["report"]["categories"]["nvidia_gpu"]["reason"]
            .as_str()
            .unwrap()
            .contains("nvidia-smi was not found")
    );
    assert_eq!(
        value["report"]["categories"]["vulkan"]["state"],
        "unavailable"
    );
    assert_eq!(
        value["report"]["categories"]["storage"]["state"],
        "unavailable"
    );
    assert_eq!(
        value["report"]["categories"]["engines"]["state"],
        "available"
    );
    assert!(
        value["report"]["system"]["engines"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[cfg(unix)]
#[test]
fn discovers_a_fake_engine_without_launching_inference() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    let directory: PathBuf = std::env::temp_dir().join(format!(
        "ghostram-fake-engine-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let executable = directory.join("llama-server");
    fs::write(
        &executable,
        "#!/bin/sh\n\
         case \"$1\" in\n\
           --version) printf '%s\\n' 'version: fake-llama 0.0-test'; exit 0 ;;\n\
           --list-devices) printf '%s\\n' 'Available devices:' 'CUDA0: Fake GPU'; exit 0 ;;\n\
           *) printf '%s\\n' 'unexpected argument' >&2; exit 42 ;;\n\
         esac\n",
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();

    let output = Command::new(binary())
        .args(["doctor", "--json"])
        .env("PATH", &directory)
        .output()
        .expect("ghostram binary should start");
    let _ = fs::remove_dir_all(&directory);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be JSON only");
    let engines = value["report"]["system"]["engines"]
        .as_array()
        .expect("engines must be an array");
    assert_eq!(engines.len(), 1);
    assert_eq!(engines[0]["executable"], "llama-server");
    assert_eq!(engines[0]["family"], "llama_cpp");
    assert_eq!(engines[0]["state"], "available");
    assert_eq!(engines[0]["version"], "version: fake-llama 0.0-test");
    assert_eq!(engines[0]["devices"][1], "CUDA0: Fake GPU");
    assert_eq!(
        value["report"]["categories"]["engines"]["state"],
        "available"
    );
}
