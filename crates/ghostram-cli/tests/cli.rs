//! CLI integration tests. They use the real binary but do not require a GPU,
//! Vulkan device, engine, model, root access, or network service.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_ghostram")
}

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "ghostram-cli-test-{}-{nanos}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write(&self, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = self.path.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, bytes).unwrap();
        path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn encode_string(output: &mut Vec<u8>, value: &str) {
    output.extend_from_slice(&u64::try_from(value.len()).unwrap().to_le_bytes());
    output.extend_from_slice(value.as_bytes());
}

fn push_entry(metadata: &mut Vec<u8>, key: &str, value_type: u32, value: &[u8]) {
    encode_string(metadata, key);
    metadata.extend_from_slice(&value_type.to_le_bytes());
    metadata.extend_from_slice(value);
}

fn synthetic_gguf(name: &str) -> Vec<u8> {
    let mut metadata = Vec::new();
    push_entry(&mut metadata, "general.alignment", 4, &32_u32.to_le_bytes());

    let mut architecture = Vec::new();
    encode_string(&mut architecture, "llama");
    push_entry(&mut metadata, "general.architecture", 8, &architecture);

    let mut model_name = Vec::new();
    encode_string(&mut model_name, name);
    push_entry(&mut metadata, "general.name", 8, &model_name);

    push_entry(&mut metadata, "general.file_type", 4, &15_u32.to_le_bytes());

    let mut tokens = Vec::new();
    tokens.extend_from_slice(&8_u32.to_le_bytes());
    tokens.extend_from_slice(&1_u64.to_le_bytes());
    encode_string(&mut tokens, "secret-token-value");
    push_entry(&mut metadata, "tokenizer.ggml.tokens", 9, &tokens);

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GGUF");
    bytes.extend_from_slice(&3_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    bytes.extend_from_slice(&5_u64.to_le_bytes());
    bytes.extend_from_slice(&metadata);
    bytes
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

#[test]
fn help_lists_the_models_scan_command() {
    let output = run(&["--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("ghostram models scan <path>"));
    assert!(stdout.contains("--recursive"));
    assert!(stdout.contains("--hash"));
}

#[test]
fn models_scan_help_documents_safety_boundaries() {
    let output = run(&["models", "scan", "--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("No default directory"));
    assert!(stdout.contains("Symlinks are never followed"));
    assert!(stdout.contains("ghostram.models.scan/1"));
    assert!(stdout.contains("complete file, including tensor payloads"));
}

#[test]
fn models_scan_json_honors_nonrecursive_and_recursive_modes() {
    let directory = TestDirectory::new();
    directory.write("a.gguf", &synthetic_gguf("a"));
    let mut bad = vec![0_u8; 24];
    bad[..4].copy_from_slice(b"NOPE");
    directory.write("b.gguf", &bad);
    directory.write("nested/c.gguf", &synthetic_gguf("c"));

    let root = directory.path().to_str().unwrap();
    let flat = run(&["models", "scan", root, "--json"]);
    assert!(
        flat.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&flat.stderr)
    );
    assert!(flat.stderr.is_empty());
    assert!(!String::from_utf8_lossy(&flat.stdout).contains("secret-token-value"));
    let flat: serde_json::Value =
        serde_json::from_slice(&flat.stdout).expect("stdout must be JSON only");
    assert_eq!(flat["schema_version"], "ghostram.models.scan/1");
    assert_eq!(flat["scan"]["state"], "partial");
    assert_eq!(flat["scan"]["summary"]["candidates"], 2);
    assert_eq!(flat["scan"]["summary"]["parsed"], 1);
    assert_eq!(flat["scan"]["summary"]["malformed"], 1);
    assert_eq!(flat["scan"]["options"]["recursive"], false);
    assert_eq!(
        flat["scan"]["files"][0]["path"].as_str().unwrap(),
        format!("{root}/a.gguf")
    );
    assert_eq!(
        flat["scan"]["files"][1]["path"].as_str().unwrap(),
        format!("{root}/b.gguf")
    );

    let recursive = run(&["models", "scan", root, "--recursive", "--json"]);
    assert!(recursive.status.success());
    let recursive: serde_json::Value = serde_json::from_slice(&recursive.stdout).unwrap();
    assert_eq!(recursive["scan"]["summary"]["candidates"], 3);
    assert_eq!(recursive["scan"]["options"]["recursive"], true);
    assert_eq!(
        recursive["scan"]["files"][2]["path"].as_str().unwrap(),
        format!("{root}/nested/c.gguf")
    );
}

#[test]
fn models_scan_json_hashes_only_when_requested() {
    let directory = TestDirectory::new();
    let bytes = synthetic_gguf("hashed");
    let path = directory.write("model.gguf", &bytes);
    let root = directory.path().to_str().unwrap();

    let plain = run(&["models", "scan", root, "--json"]);
    assert!(plain.status.success());
    let plain: serde_json::Value = serde_json::from_slice(&plain.stdout).unwrap();
    assert_eq!(plain["scan"]["files"][0]["hash"]["status"], "not_requested");
    assert_eq!(plain["scan"]["files"][0]["hash"]["bytes_hashed"], 0);

    let hashed = run(&["models", "scan", root, "--hash", "--json"]);
    assert!(
        hashed.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&hashed.stderr)
    );
    let hashed: serde_json::Value = serde_json::from_slice(&hashed.stdout).unwrap();
    let file = &hashed["scan"]["files"][0];
    let digest = file["hash"]["sha256"].as_str().unwrap();
    assert_eq!(digest.len(), 64);
    assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(file["hash"]["status"], "computed");
    assert_eq!(
        file["hash"]["bytes_hashed"].as_u64().unwrap(),
        fs::metadata(path).unwrap().len()
    );
    assert_eq!(hashed["scan"]["summary"]["hash_computed"], 1);
}

#[test]
fn models_scan_human_output_reports_metadata_and_partial_state() {
    let directory = TestDirectory::new();
    directory.write("model.gguf", &synthetic_gguf("Synthetic Model"));
    directory.write("bad.gguf", b"not a gguf file");
    let root = directory.path().to_str().unwrap();
    let output = run(&["models", "scan", root]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("GhostRAM model scan"));
    assert!(stdout.contains("Schema: ghostram.models.scan/1"));
    assert!(stdout.contains("State: partial"));
    assert!(stdout.contains("candidates: 2"));
    assert!(stdout.contains("architecture: llama (explicit)"));
    assert!(stdout.contains("quantization: Q4_K_M (explicit)"));
    assert!(stdout.contains("tokens: 1 elements of string"));
    assert!(stdout.contains("chat template present: no"));
    assert!(stdout.contains("status: parsed"));
    assert!(stdout.contains("status: malformed"));
    assert!(!stdout.contains("secret-token-value"));
}

#[test]
fn models_scan_empty_directory_succeeds() {
    let directory = TestDirectory::new();
    let root = directory.path().to_str().unwrap();
    let output = run(&["models", "scan", root, "--json"]);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["scan"]["state"], "empty");
    assert_eq!(value["scan"]["summary"]["candidates"], 0);
}

#[test]
fn models_scan_rejects_missing_paths_and_unknown_options() {
    let missing = run(&["models", "scan"]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(missing.stdout.is_empty());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("requires a directory path"));

    let unknown = run(&["models", "scan", "--not-an-option", "."]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("unexpected option"));
}

#[test]
fn models_scan_root_failure_exits_one() {
    let directory = TestDirectory::new();
    let missing = directory.path().join("does-not-exist");
    let output = run(&["models", "scan", missing.to_str().unwrap(), "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("model scan failed"));
}
