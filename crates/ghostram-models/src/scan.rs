//! Read-only, symlink-conservative directory traversal and file inspection.

use std::fs::{self, File, Metadata, OpenOptions};
use std::io::Seek;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use ghostram_core::quantity::Bytes;
use ghostram_core::time::format_utc;

use crate::catalog::{
    Failure, FailureKind, FileCatalogEntry, FilenameHints, HashAlgorithm, HashReport, HashStatus,
    ModelsScan, ModelsScanEnvelope, ParseStatus, ScanIssue, ScanIssueKind, ScanOptions, ScanState,
    ScanSummary, SkippedSummary,
};
use crate::error::ScanError;
use crate::filename::hints_from_filename;
use crate::gguf::parse_gguf;
use crate::hash::{HashComputation, HashReadError, hash_reader};

/// Maximum directory depth below the selected root.
pub const MAX_SCAN_DEPTH: u32 = 32;
/// Maximum directory entries observed in one scan.
pub const MAX_SCAN_ENTRIES: u64 = 100_000;
/// Maximum traversal issue records retained in the JSON envelope.
pub const MAX_SCAN_ISSUES: usize = 100;

/// A deterministic test and instrumentation hook. Production scanning does not
/// expose this callback.
#[allow(dead_code)] // Event paths are consumed by test observers, not production.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ScanEvent<'a> {
    /// A candidate was discovered and is about to be opened.
    Discovered(&'a Path),
    /// Parsing completed and hashing, if requested, is about to start.
    AfterParse(&'a Path),
}

#[derive(Debug, Clone)]
pub(crate) struct TraversalLimits {
    pub(crate) max_depth: u32,
    pub(crate) max_entries: u64,
    pub(crate) max_issues: usize,
}

impl Default for TraversalLimits {
    fn default() -> Self {
        Self {
            max_depth: MAX_SCAN_DEPTH,
            max_entries: MAX_SCAN_ENTRIES,
            max_issues: MAX_SCAN_ISSUES,
        }
    }
}

/// Scans one explicitly selected directory for candidate GGUF files.
///
/// Symbolic links are never followed. The root itself must be a real directory.
/// Only `.gguf` files directly in the root are inspected unless `recursive` is
/// set. No default directory, cache, database, or network access is used.
///
/// # Errors
///
/// Returns [`ScanError`] when the selected root is not a readable directory, is a
/// symbolic link, or cannot be represented as UTF-8. Per-file failures are
/// recorded inside the returned envelope and do not abort the scan.
pub fn scan_directory(
    root: &Path,
    options: ScanOptions,
    application_version: &str,
    generated_at: &str,
) -> Result<ModelsScanEnvelope, ScanError> {
    let limits = TraversalLimits::default();
    let mut observer = |_event: ScanEvent<'_>| {};
    scan_directory_with(
        root,
        options,
        application_version,
        generated_at,
        &limits,
        &mut observer,
    )
}

fn scan_directory_with<F>(
    root: &Path,
    options: ScanOptions,
    application_version: &str,
    generated_at: &str,
    limits: &TraversalLimits,
    observer: &mut F,
) -> Result<ModelsScanEnvelope, ScanError>
where
    F: FnMut(ScanEvent<'_>),
{
    let absolute = std::path::absolute(root)
        .map_err(|error| ScanError::new(root, format!("could not resolve path: {error}")))?;
    let root_text = absolute
        .to_str()
        .ok_or_else(|| ScanError::new(&absolute, "scan root is not valid UTF-8"))?
        .to_string();
    let root_metadata = fs::symlink_metadata(&absolute)
        .map_err(|error| ScanError::new(&absolute, format!("could not inspect path: {error}")))?;
    if root_metadata.file_type().is_symlink() {
        return Err(ScanError::new(
            &absolute,
            "scan root is a symbolic link; select the real directory explicitly",
        ));
    }
    if !root_metadata.is_dir() {
        return Err(ScanError::new(&absolute, "scan root is not a directory"));
    }

    let mut walker = Walker::new(options, limits);
    walker.walk(&absolute, 0, true)?;

    let mut files = Vec::with_capacity(walker.candidates.len());
    for path in walker.candidates {
        files.push(inspect_candidate(&path, options, observer));
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));

    let summary = summarize(&files);
    let issues = walker.issues;
    let skipped = walker.skipped;
    let state = scan_state(&files, &skipped, &issues);
    let scan = ModelsScan {
        state,
        roots: vec![root_text],
        options,
        summary,
        skipped,
        issues,
        files,
    };
    Ok(ModelsScanEnvelope::new(
        application_version,
        generated_at,
        scan,
    ))
}

struct Walker<'a> {
    options: ScanOptions,
    limits: &'a TraversalLimits,
    candidates: Vec<PathBuf>,
    skipped: SkippedSummary,
    issues: Vec<ScanIssue>,
    issue_count: u64,
    entry_count: u64,
    stop: bool,
}

impl<'a> Walker<'a> {
    fn new(options: ScanOptions, limits: &'a TraversalLimits) -> Self {
        Self {
            options,
            limits,
            candidates: Vec::new(),
            skipped: SkippedSummary {
                symlinks: 0,
                non_regular_files: 0,
                non_gguf_files: 0,
                non_utf8_paths: 0,
                max_depth_directories: 0,
                entry_limit_reached: false,
                unreadable_directories: 0,
                unreadable_entries: 0,
                issues_truncated: false,
            },
            issues: Vec::new(),
            issue_count: 0,
            entry_count: 0,
            stop: false,
        }
    }

    fn walk(
        &mut self,
        directory: &Path,
        depth: u32,
        fatal_read_error: bool,
    ) -> Result<(), ScanError> {
        if self.stop {
            return Ok(());
        }
        let reader = match fs::read_dir(directory) {
            Ok(reader) => reader,
            Err(error) if fatal_read_error => {
                return Err(ScanError::new(
                    directory,
                    format!("could not read selected directory: {error}"),
                ));
            }
            Err(error) => {
                self.skipped.unreadable_directories =
                    self.skipped.unreadable_directories.saturating_add(1);
                self.add_issue(
                    directory,
                    ScanIssueKind::UnreadableDirectory,
                    format!("could not read directory: {error}"),
                );
                return Ok(());
            }
        };

        let mut entries = Vec::new();
        for entry in reader {
            if self.stop {
                break;
            }
            self.entry_count = self.entry_count.saturating_add(1);
            if self.entry_count > self.limits.max_entries {
                self.skipped.entry_limit_reached = true;
                self.stop = true;
                self.add_issue(
                    directory,
                    ScanIssueKind::EntryLimitReached,
                    format!(
                        "directory entry limit of {} stopped traversal",
                        self.limits.max_entries
                    ),
                );
                break;
            }
            match entry {
                Ok(entry) => entries.push(entry),
                Err(error) => {
                    self.skipped.unreadable_entries =
                        self.skipped.unreadable_entries.saturating_add(1);
                    self.add_issue(
                        directory,
                        ScanIssueKind::UnreadableEntry,
                        format!("could not read directory entry: {error}"),
                    );
                }
            }
        }

        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    self.skipped.unreadable_entries =
                        self.skipped.unreadable_entries.saturating_add(1);
                    self.add_issue(
                        &path,
                        ScanIssueKind::UnreadableEntry,
                        format!("could not read entry type: {error}"),
                    );
                    continue;
                }
            };
            if file_type.is_symlink() {
                self.skipped.symlinks = self.skipped.symlinks.saturating_add(1);
                continue;
            }
            if file_type.is_dir() {
                if !self.options.recursive || self.stop {
                    continue;
                }
                if depth >= self.limits.max_depth {
                    self.skipped.max_depth_directories =
                        self.skipped.max_depth_directories.saturating_add(1);
                    self.add_issue(
                        &path,
                        ScanIssueKind::DepthLimitReached,
                        format!(
                            "directory depth limit of {} prevented descent",
                            self.limits.max_depth
                        ),
                    );
                    continue;
                }
                self.walk(&path, depth.saturating_add(1), false)?;
                continue;
            }
            if file_type.is_file() {
                if !is_gguf_path(&path) {
                    self.skipped.non_gguf_files = self.skipped.non_gguf_files.saturating_add(1);
                } else if path.to_str().is_none() {
                    self.skipped.non_utf8_paths = self.skipped.non_utf8_paths.saturating_add(1);
                    self.add_issue(
                        &path,
                        ScanIssueKind::UnreadableEntry,
                        "candidate path is not valid UTF-8 and was not cataloged".to_string(),
                    );
                } else {
                    self.candidates.push(path);
                }
                continue;
            }
            if is_gguf_path(&path) {
                self.skipped.non_regular_files = self.skipped.non_regular_files.saturating_add(1);
            }
        }
        Ok(())
    }

    fn add_issue(&mut self, path: &Path, kind: ScanIssueKind, message: String) {
        self.issue_count = self.issue_count.saturating_add(1);
        if self.issues.len() >= self.limits.max_issues {
            self.skipped.issues_truncated = true;
            return;
        }
        self.issues.push(ScanIssue {
            path: path.to_string_lossy().into_owned(),
            kind,
            message,
        });
    }
}

fn is_gguf_path(path: &Path) -> bool {
    const SUFFIX: &[u8] = b".gguf";
    match path.file_name().and_then(|name| name.to_str()) {
        Some(name) => {
            name.len() >= SUFFIX.len()
                && name.as_bytes()[name.len() - SUFFIX.len()..].eq_ignore_ascii_case(SUFFIX)
        }
        // Preserve candidate diagnostics for non-UTF-8 names with an ASCII suffix.
        None => path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("gguf")),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileSnapshot {
    length: u64,
    modified: Option<SystemTime>,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

impl FileSnapshot {
    fn from_metadata(metadata: &Metadata) -> Self {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Self {
            length: metadata.len(),
            modified: metadata.modified().ok(),
            #[cfg(unix)]
            device: metadata.dev(),
            #[cfg(unix)]
            inode: metadata.ino(),
        }
    }
}

enum HashAttempt {
    NotRequested,
    Computed(HashComputation),
    Failed(HashReadError),
}

fn inspect_candidate<F>(path: &Path, options: ScanOptions, observer: &mut F) -> FileCatalogEntry
where
    F: FnMut(ScanEvent<'_>),
{
    let path_text = path
        .to_str()
        .expect("walk only stores UTF-8 candidate paths")
        .to_string();
    let filename_hints = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(hints_from_filename)
        .unwrap_or_default();
    observer(ScanEvent::Discovered(path));

    let mut file = match open_candidate(path) {
        Ok(file) => file,
        Err(error) => {
            return unreadable_entry(
                path,
                path_text,
                filename_hints,
                options,
                format!("could not open candidate: {error}"),
            );
        }
    };
    let before = match file.metadata() {
        Ok(metadata) if metadata.is_file() => FileSnapshot::from_metadata(&metadata),
        Ok(_) => {
            return unreadable_entry(
                path,
                path_text,
                filename_hints,
                options,
                "candidate is not a regular file after opening".to_string(),
            );
        }
        Err(error) => {
            return unreadable_entry(
                path,
                path_text,
                filename_hints,
                options,
                format!("could not read candidate metadata: {error}"),
            );
        }
    };

    let parse_result = parse_gguf(&mut file, before.length);
    observer(ScanEvent::AfterParse(path));
    let hash_attempt = if options.hash {
        match file.rewind() {
            Ok(()) => match hash_reader(&mut file) {
                Ok(computation) => HashAttempt::Computed(computation),
                Err(error) => HashAttempt::Failed(error),
            },
            Err(error) => HashAttempt::Failed(HashReadError {
                message: format!("could not rewind candidate before hashing: {error}"),
                bytes_hashed: 0,
            }),
        }
    } else {
        HashAttempt::NotRequested
    };

    let after = file
        .metadata()
        .ok()
        .filter(Metadata::is_file)
        .map(|metadata| FileSnapshot::from_metadata(&metadata));
    let path_after = fs::symlink_metadata(path)
        .ok()
        .filter(|metadata| metadata.is_file())
        .map(|metadata| FileSnapshot::from_metadata(&metadata));
    let hash_length_changed = match &hash_attempt {
        HashAttempt::Computed(computation) => computation.bytes_hashed != before.length,
        HashAttempt::Failed(_) | HashAttempt::NotRequested => false,
    };
    let changed = after != Some(before) || path_after != Some(before) || hash_length_changed;

    let (metadata, parse_failure) = match parse_result {
        Ok(metadata) => (Some(metadata), None),
        Err(error) => (None, Some(error.into_failure())),
    };
    let parse_status = if changed {
        ParseStatus::Changed
    } else if let Some(failure) = &parse_failure {
        parse_status_for_failure(failure.kind)
    } else {
        ParseStatus::Parsed
    };
    let failure = if changed {
        parse_failure.or_else(|| {
            Some(Failure {
                kind: FailureKind::Changed,
                message: "file identity, size, or modification time changed during inspection"
                    .to_string(),
                offset: None,
            })
        })
    } else {
        parse_failure
    };
    let hash = hash_report(changed, options.hash, hash_attempt);
    let metadata = if changed { None } else { metadata };

    FileCatalogEntry {
        path: path_text,
        file_size: Some(Bytes::new(before.length)),
        modified_at: before.modified.map(format_utc),
        parse_status,
        failure,
        metadata,
        hash,
        filename_hints,
    }
}

fn open_candidate(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // A candidate can be replaced after directory inspection. Do not follow a
        // symlink, and do not block forever if it becomes a named pipe before open.
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    options.open(path)
}

fn unreadable_entry(
    path: &Path,
    path_text: String,
    filename_hints: FilenameHints,
    options: ScanOptions,
    message: String,
) -> FileCatalogEntry {
    let observed = fs::symlink_metadata(path)
        .ok()
        .filter(Metadata::is_file)
        .map(|metadata| FileSnapshot::from_metadata(&metadata));
    let hash = if options.hash {
        HashReport {
            algorithm: HashAlgorithm::Sha256,
            status: HashStatus::Failed,
            sha256: None,
            bytes_hashed: Bytes::ZERO,
            reason: Some(message.clone()),
        }
    } else {
        not_requested_hash()
    };
    FileCatalogEntry {
        path: path_text,
        file_size: observed.map(|snapshot| Bytes::new(snapshot.length)),
        modified_at: observed
            .and_then(|snapshot| snapshot.modified)
            .map(format_utc),
        parse_status: ParseStatus::Unreadable,
        failure: Some(Failure {
            kind: FailureKind::Io,
            message,
            offset: None,
        }),
        metadata: None,
        hash,
        filename_hints,
    }
}

fn hash_report(changed: bool, requested: bool, attempt: HashAttempt) -> HashReport {
    if !requested {
        return not_requested_hash();
    }
    let bytes_hashed = match &attempt {
        HashAttempt::Computed(computation) => Bytes::new(computation.bytes_hashed),
        HashAttempt::Failed(error) => Bytes::new(error.bytes_hashed),
        HashAttempt::NotRequested => Bytes::ZERO,
    };
    if changed {
        let reason = match attempt {
            HashAttempt::Computed(_) => {
                "file changed during inspection; digest discarded".to_string()
            }
            HashAttempt::Failed(error) => {
                format!(
                    "file changed during inspection; hash read also failed: {}",
                    error.message
                )
            }
            HashAttempt::NotRequested => "file changed during inspection".to_string(),
        };
        return HashReport {
            algorithm: HashAlgorithm::Sha256,
            status: HashStatus::Changed,
            sha256: None,
            bytes_hashed,
            reason: Some(reason),
        };
    }
    match attempt {
        HashAttempt::Computed(computation) => HashReport {
            algorithm: HashAlgorithm::Sha256,
            status: HashStatus::Computed,
            sha256: Some(computation.digest_hex),
            bytes_hashed,
            reason: None,
        },
        HashAttempt::Failed(error) => HashReport {
            algorithm: HashAlgorithm::Sha256,
            status: HashStatus::Failed,
            sha256: None,
            bytes_hashed,
            reason: Some(error.message),
        },
        HashAttempt::NotRequested => not_requested_hash(),
    }
}

fn not_requested_hash() -> HashReport {
    HashReport {
        algorithm: HashAlgorithm::Sha256,
        status: HashStatus::NotRequested,
        sha256: None,
        bytes_hashed: Bytes::ZERO,
        reason: None,
    }
}

const fn parse_status_for_failure(kind: FailureKind) -> ParseStatus {
    match kind {
        FailureKind::UnsupportedVersion
        | FailureKind::UnsupportedEndianness
        | FailureKind::UnsupportedMetadataType
        | FailureKind::LimitExceeded => ParseStatus::Unsupported,
        FailureKind::Io => ParseStatus::Unreadable,
        FailureKind::Changed => ParseStatus::Changed,
        FailureKind::InvalidMagic
        | FailureKind::InvalidAlignment
        | FailureKind::DuplicateMetadataKey
        | FailureKind::DuplicateTensorName
        | FailureKind::InvalidTensor
        | FailureKind::Truncated
        | FailureKind::Malformed => ParseStatus::Malformed,
    }
}

fn summarize(files: &[FileCatalogEntry]) -> ScanSummary {
    let mut summary = ScanSummary {
        candidates: u64::try_from(files.len()).unwrap_or(u64::MAX),
        parsed: 0,
        parsed_with_warnings: 0,
        incomplete_metadata: 0,
        unsupported: 0,
        malformed: 0,
        unreadable: 0,
        changed: 0,
        hash_computed: 0,
        hash_failed: 0,
        hash_changed: 0,
        bytes_hashed: Bytes::ZERO,
    };
    let mut bytes_hashed = 0_u64;
    for file in files {
        match file.parse_status {
            ParseStatus::Parsed => {
                summary.parsed = summary.parsed.saturating_add(1);
                if file
                    .metadata
                    .as_ref()
                    .is_some_and(|metadata| !metadata.warnings.is_empty())
                {
                    summary.parsed_with_warnings = summary.parsed_with_warnings.saturating_add(1);
                }
                if file.metadata.as_ref().is_some_and(|metadata| {
                    metadata.metadata_state == crate::catalog::MetadataState::Incomplete
                }) {
                    summary.incomplete_metadata = summary.incomplete_metadata.saturating_add(1);
                }
            }
            ParseStatus::Unsupported => {
                summary.unsupported = summary.unsupported.saturating_add(1);
            }
            ParseStatus::Malformed => {
                summary.malformed = summary.malformed.saturating_add(1);
            }
            ParseStatus::Unreadable => {
                summary.unreadable = summary.unreadable.saturating_add(1);
            }
            ParseStatus::Changed => {
                summary.changed = summary.changed.saturating_add(1);
            }
        }
        match file.hash.status {
            HashStatus::Computed => {
                summary.hash_computed = summary.hash_computed.saturating_add(1);
            }
            HashStatus::Failed => {
                summary.hash_failed = summary.hash_failed.saturating_add(1);
            }
            HashStatus::Changed => {
                summary.hash_changed = summary.hash_changed.saturating_add(1);
            }
            HashStatus::NotRequested => {}
        }
        bytes_hashed = bytes_hashed.saturating_add(file.hash.bytes_hashed.as_u64());
    }
    summary.bytes_hashed = Bytes::new(bytes_hashed);
    summary
}

fn scan_state(
    files: &[FileCatalogEntry],
    skipped: &SkippedSummary,
    issues: &[ScanIssue],
) -> ScanState {
    let traversal_incomplete = skipped.entry_limit_reached
        || skipped.max_depth_directories > 0
        || skipped.unreadable_directories > 0
        || skipped.unreadable_entries > 0
        || skipped.non_utf8_paths > 0
        || skipped.issues_truncated
        || !issues.is_empty();
    if traversal_incomplete
        || files
            .iter()
            .any(|file| file.parse_status != ParseStatus::Parsed)
    {
        ScanState::Partial
    } else if files.is_empty() {
        ScanState::Empty
    } else {
        ScanState::Complete
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{ScanEvent, TraversalLimits, scan_directory, scan_directory_with};
    use crate::catalog::{
        FailureKind, HashStatus, MetadataState, ModelsScanEnvelope, ParseStatus, ScanOptions,
        ScanState,
    };
    use crate::test_support::GgufBuilder;

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos();
            let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "ghostram-models-test-{}-{nanos}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("create test directory");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.path.join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create parent directory");
            }
            fs::write(&path, bytes).expect("write test fixture");
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn valid_fixture(name: &str) -> Vec<u8> {
        let mut builder = GgufBuilder::new(3);
        builder.u32("general.alignment", 32);
        builder.string("general.architecture", "llama");
        builder.string("general.name", name);
        builder.u32("general.file_type", 15);
        builder.u64("llama.context_length", 4096);
        builder.u64("llama.block_count", 32);
        builder.string_array("tokenizer.ggml.tokens", &["secret-token-value"]);
        builder.finish(32)
    }

    fn scan(root: &Path, options: ScanOptions) -> ModelsScanEnvelope {
        scan_directory(root, options, "0.1.0-test", "2026-10-21T00:00:00Z").expect("scan succeeds")
    }

    #[test]
    fn recognizes_gguf_name_suffixes() {
        assert!(super::is_gguf_path(Path::new("model.gguf")));
        assert!(super::is_gguf_path(Path::new("model.GGUF")));
        assert!(super::is_gguf_path(Path::new(".gguf")));
        assert!(!super::is_gguf_path(Path::new("model.gguf.bak")));
        assert!(!super::is_gguf_path(Path::new("gguf")));
    }

    #[test]
    fn scans_flat_directories_case_insensitively() {
        let temp = TempDir::new();
        temp.write("model.GGUF", &valid_fixture("upper"));
        temp.write("notes.txt", b"not a model");
        temp.write("nested/model.gguf", &valid_fixture("nested"));
        let envelope = scan(temp.path(), ScanOptions::default());
        let scan = &envelope.scan;

        assert_eq!(scan.state, ScanState::Complete);
        assert_eq!(scan.summary.candidates, 1);
        assert_eq!(scan.summary.parsed, 1);
        assert_eq!(scan.skipped.non_gguf_files, 1);
        assert_eq!(scan.files.len(), 1);
        assert_eq!(scan.files[0].parse_status, ParseStatus::Parsed);
        assert_eq!(
            scan.files[0]
                .metadata
                .as_ref()
                .unwrap()
                .general
                .name
                .as_ref()
                .unwrap()
                .value,
            "upper"
        );
        assert_eq!(scan.roots, vec![temp.path().to_str().unwrap().to_string()]);
    }

    #[test]
    fn recursive_flag_controls_subdirectories() {
        let temp = TempDir::new();
        temp.write("root.gguf", &valid_fixture("root"));
        temp.write("nested/child.gguf", &valid_fixture("child"));
        let flat = scan(temp.path(), ScanOptions::default());
        assert_eq!(flat.scan.summary.candidates, 1);
        let recursive = scan(
            temp.path(),
            ScanOptions {
                recursive: true,
                hash: false,
            },
        );
        assert_eq!(recursive.scan.summary.candidates, 2);
        assert_eq!(recursive.scan.state, ScanState::Complete);
    }

    #[test]
    fn sorts_records_by_path() {
        let temp = TempDir::new();
        temp.write("z.gguf", &valid_fixture("z"));
        temp.write("a.gguf", &valid_fixture("a"));
        temp.write("m.gguf", &valid_fixture("m"));
        let envelope = scan(temp.path(), ScanOptions::default());
        let paths: Vec<&str> = envelope
            .scan
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect();
        let mut sorted = paths.clone();
        sorted.sort_unstable();
        assert_eq!(paths, sorted);
    }

    #[test]
    fn reports_empty_directories() {
        let temp = TempDir::new();
        let envelope = scan(temp.path(), ScanOptions::default());
        assert_eq!(envelope.scan.state, ScanState::Empty);
        assert_eq!(envelope.scan.summary.candidates, 0);
        assert!(envelope.scan.files.is_empty());
    }

    #[test]
    fn mixed_valid_and_malformed_files_produce_partial_results() {
        let temp = TempDir::new();
        temp.write("valid.gguf", &valid_fixture("valid"));
        temp.write("bad.gguf", b"GGUF");
        let envelope = scan(temp.path(), ScanOptions::default());
        let scan = &envelope.scan;

        assert_eq!(scan.state, ScanState::Partial);
        assert_eq!(scan.summary.candidates, 2);
        assert_eq!(scan.summary.parsed, 1);
        assert_eq!(scan.summary.malformed, 1);
        assert_eq!(scan.files[0].parse_status, ParseStatus::Malformed);
        assert_eq!(
            scan.files[0].failure.as_ref().unwrap().kind,
            FailureKind::Truncated
        );
        assert_eq!(scan.files[1].parse_status, ParseStatus::Parsed);
    }

    #[test]
    fn filename_hints_are_cataloged_without_grouping_shards() {
        let temp = TempDir::new();
        temp.write(
            "mmproj-model-00001-of-00002.gguf",
            &valid_fixture("projector"),
        );
        let envelope = scan(temp.path(), ScanOptions::default());
        let hints = &envelope.scan.files[0].filename_hints;
        assert_eq!(hints.role, Some(crate::catalog::FilenameRole::Mmproj));
        assert_eq!(
            hints.shard,
            Some(crate::catalog::ShardHint {
                number: 1,
                total: 2
            })
        );
        assert_eq!(envelope.scan.summary.candidates, 1);
    }

    #[test]
    fn hashes_are_optional_and_identical_content_has_identical_digests() {
        let temp = TempDir::new();
        let first = valid_fixture("same");
        temp.write("first.gguf", &first);
        temp.write("nested/second.gguf", &first);
        temp.write("different.gguf", &valid_fixture("different"));

        let not_requested = scan(temp.path(), ScanOptions::default());
        assert!(
            not_requested
                .scan
                .files
                .iter()
                .all(|file| file.hash.status == HashStatus::NotRequested)
        );

        let hashed = scan(
            temp.path(),
            ScanOptions {
                recursive: true,
                hash: true,
            },
        );
        let by_name = |needle: &str| {
            hashed
                .scan
                .files
                .iter()
                .find(|file| file.path.ends_with(needle))
                .unwrap()
        };
        let first_hash = by_name("first.gguf").hash.sha256.as_ref().unwrap();
        let second_hash = by_name("second.gguf").hash.sha256.as_ref().unwrap();
        let different_hash = by_name("different.gguf").hash.sha256.as_ref().unwrap();
        assert_eq!(first_hash.len(), 64);
        assert_eq!(first_hash, second_hash);
        assert_ne!(first_hash, different_hash);
        assert_eq!(hashed.scan.summary.hash_computed, 3);
        assert_eq!(
            hashed.scan.summary.bytes_hashed.as_u64(),
            u64::try_from(first.len() * 2 + valid_fixture("different").len()).unwrap()
        );
    }

    #[test]
    fn hash_reports_the_known_empty_digest_without_a_partial_value() {
        let temp = TempDir::new();
        temp.write("empty.gguf", b"");
        let envelope = scan(
            temp.path(),
            ScanOptions {
                recursive: false,
                hash: true,
            },
        );
        let file = &envelope.scan.files[0];
        assert_eq!(file.parse_status, ParseStatus::Malformed);
        assert_eq!(file.hash.status, HashStatus::Computed);
        assert_eq!(
            file.hash.sha256.as_deref(),
            Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        );
        assert_eq!(file.hash.bytes_hashed.as_u64(), 0);
    }

    #[test]
    fn detects_a_change_after_parsing_and_discards_the_digest() {
        let temp = TempDir::new();
        let path = temp.write("changing.gguf", &valid_fixture("changing"));
        let target = path.clone();
        let envelope = scan_directory_with(
            temp.path(),
            ScanOptions {
                recursive: false,
                hash: true,
            },
            "0.1.0-test",
            "2026-10-21T00:00:00Z",
            &TraversalLimits::default(),
            &mut |event| {
                if let ScanEvent::AfterParse(event_path) = event {
                    if event_path == target.as_path() {
                        fs::write(&target, b"short").expect("mutate fixture");
                    }
                }
            },
        )
        .unwrap();
        let file = &envelope.scan.files[0];
        assert_eq!(file.parse_status, ParseStatus::Changed);
        assert_eq!(file.hash.status, HashStatus::Changed);
        assert_eq!(file.hash.sha256, None);
        assert!(file.metadata.is_none());
        assert_eq!(envelope.scan.summary.changed, 1);
        assert_eq!(envelope.scan.summary.hash_changed, 1);
    }

    #[test]
    fn detects_disappearance_during_inspection() {
        let temp = TempDir::new();
        let path = temp.write("vanishing.gguf", &valid_fixture("vanishing"));
        let target = path.clone();
        let envelope = scan_directory_with(
            temp.path(),
            ScanOptions::default(),
            "0.1.0-test",
            "2026-10-21T00:00:00Z",
            &TraversalLimits::default(),
            &mut |event| {
                if let ScanEvent::AfterParse(event_path) = event {
                    if event_path == target.as_path() {
                        fs::remove_file(&target).expect("remove fixture");
                    }
                }
            },
        )
        .unwrap();
        let file = &envelope.scan.files[0];
        assert_eq!(file.parse_status, ParseStatus::Changed);
        assert_eq!(
            file.failure.as_ref().map(|failure| failure.kind),
            Some(FailureKind::Changed)
        );
    }

    #[test]
    fn reports_unreadable_candidates_without_aborting() {
        let temp = TempDir::new();
        let path = temp.write("removed.gguf", &valid_fixture("removed"));
        temp.write("valid.gguf", &valid_fixture("valid"));
        let target = path.clone();
        let envelope = scan_directory_with(
            temp.path(),
            ScanOptions {
                recursive: false,
                hash: true,
            },
            "0.1.0-test",
            "2026-10-21T00:00:00Z",
            &TraversalLimits::default(),
            &mut |event| {
                if let ScanEvent::Discovered(event_path) = event {
                    if event_path == target.as_path() {
                        fs::remove_file(&target).expect("remove fixture");
                    }
                }
            },
        )
        .unwrap();
        assert_eq!(envelope.scan.summary.unreadable, 1);
        assert_eq!(envelope.scan.summary.parsed, 1);
        assert_eq!(envelope.scan.state, ScanState::Partial);
        let unreadable = envelope
            .scan
            .files
            .iter()
            .find(|file| file.path.ends_with("removed.gguf"))
            .unwrap();
        assert_eq!(unreadable.parse_status, ParseStatus::Unreadable);
        assert_eq!(unreadable.failure.as_ref().unwrap().kind, FailureKind::Io);
        assert_eq!(unreadable.hash.status, HashStatus::Failed);
        assert_eq!(unreadable.hash.sha256, None);
    }

    #[test]
    fn enforces_entry_and_depth_limits() {
        let temp = TempDir::new();
        temp.write("a.gguf", &valid_fixture("a"));
        temp.write("b.gguf", &valid_fixture("b"));
        temp.write("c.gguf", &valid_fixture("c"));
        let limits = TraversalLimits {
            max_depth: 32,
            max_entries: 2,
            max_issues: 100,
        };
        let mut observer = |_event: ScanEvent<'_>| {};
        let envelope = scan_directory_with(
            temp.path(),
            ScanOptions::default(),
            "0.1.0-test",
            "2026-10-21T00:00:00Z",
            &limits,
            &mut observer,
        )
        .unwrap();
        assert_eq!(envelope.scan.state, ScanState::Partial);
        assert_eq!(envelope.scan.summary.candidates, 2);
        assert!(envelope.scan.skipped.entry_limit_reached);
        assert!(
            envelope
                .scan
                .issues
                .iter()
                .any(|issue| issue.kind == crate::catalog::ScanIssueKind::EntryLimitReached)
        );

        let temp = TempDir::new();
        temp.write("nested/model.gguf", &valid_fixture("nested"));
        let limits = TraversalLimits {
            max_depth: 0,
            max_entries: 100,
            max_issues: 100,
        };
        let envelope = scan_directory_with(
            temp.path(),
            ScanOptions {
                recursive: true,
                hash: false,
            },
            "0.1.0-test",
            "2026-10-21T00:00:00Z",
            &limits,
            &mut observer,
        )
        .unwrap();
        assert_eq!(envelope.scan.state, ScanState::Partial);
        assert_eq!(envelope.scan.skipped.max_depth_directories, 1);
        assert_eq!(envelope.scan.summary.candidates, 0);
    }

    #[test]
    fn rejects_root_symlinks_and_non_directories() {
        let temp = TempDir::new();
        let target = temp.path().join("real");
        fs::create_dir(&target).unwrap();
        let link = temp.path().join("link");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();
        #[cfg(unix)]
        {
            let error = scan_directory(
                &link,
                ScanOptions::default(),
                "0.1.0-test",
                "2026-10-21T00:00:00Z",
            )
            .unwrap_err();
            assert!(error.message().contains("symbolic link"));
        }

        let file = temp.write("not-a-directory", b"x");
        let error = scan_directory(
            &file,
            ScanOptions::default(),
            "0.1.0-test",
            "2026-10-21T00:00:00Z",
        )
        .unwrap_err();
        assert!(error.message().contains("not a directory"));
    }

    #[cfg(unix)]
    #[test]
    fn skips_file_and_directory_symlinks() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new();
        temp.write("real.gguf", &valid_fixture("real"));
        let link_file = temp.path().join("linked.gguf");
        symlink(temp.path().join("real.gguf"), &link_file).unwrap();
        let real_directory = temp.path().join("nested");
        fs::create_dir(&real_directory).unwrap();
        fs::write(real_directory.join("inside.gguf"), valid_fixture("inside")).unwrap();
        let link_directory = temp.path().join("linked-directory");
        symlink(&real_directory, &link_directory).unwrap();

        let envelope = scan(
            temp.path(),
            ScanOptions {
                recursive: true,
                hash: false,
            },
        );
        assert_eq!(envelope.scan.summary.candidates, 2);
        assert_eq!(envelope.scan.skipped.symlinks, 2);
        assert_eq!(envelope.scan.state, ScanState::Complete);
        assert!(
            envelope
                .scan
                .files
                .iter()
                .all(|file| !file.path.contains("linked"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn reports_non_utf8_candidate_paths() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let temp = TempDir::new();
        let name = OsString::from_vec(b"bad\xff.gguf".to_vec());
        let path = temp.path().join(name);
        fs::write(&path, valid_fixture("bad-name")).unwrap();
        let envelope = scan(temp.path(), ScanOptions::default());
        assert_eq!(envelope.scan.skipped.non_utf8_paths, 1);
        assert_eq!(envelope.scan.summary.candidates, 0);
        assert_eq!(envelope.scan.state, ScanState::Partial);
        assert!(!envelope.scan.issues.is_empty());
    }

    #[test]
    fn serializes_a_valid_contract_without_token_values() {
        let temp = TempDir::new();
        temp.write("model.gguf", &valid_fixture("Synthetic Model"));
        let envelope = scan(
            temp.path(),
            ScanOptions {
                recursive: false,
                hash: true,
            },
        );
        envelope.validate_schema().unwrap();
        let encoded = serde_json::to_string_pretty(&envelope).unwrap();
        assert!(encoded.contains("\"schema_version\": \"ghostram.models.scan/1\""));
        assert!(encoded.contains("\"parse_status\": \"parsed\""));
        assert!(encoded.contains("\"metadata_state\": \"available\""));
        assert!(encoded.contains("\"algorithm\": \"sha256\""));
        assert!(encoded.contains("\"status\": \"computed\""));
        assert!(!encoded.contains("secret-token-value"));
        let decoded: ModelsScanEnvelope = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, envelope);
        assert_eq!(
            decoded.scan.files[0]
                .metadata
                .as_ref()
                .unwrap()
                .metadata_state,
            MetadataState::Available
        );
    }

    #[test]
    fn summary_counts_match_file_records() {
        let temp = TempDir::new();
        temp.write("valid.gguf", &valid_fixture("valid"));
        temp.write("bad.gguf", b"not gguf");
        temp.write("future.gguf", &{
            let mut bytes = valid_fixture("future");
            bytes[4..8].copy_from_slice(&9_u32.to_le_bytes());
            bytes
        });
        let envelope = scan(
            temp.path(),
            ScanOptions {
                recursive: false,
                hash: true,
            },
        );
        let scan = &envelope.scan;
        assert_eq!(scan.summary.candidates, 3);
        assert_eq!(
            scan.summary.parsed
                + scan.summary.unsupported
                + scan.summary.malformed
                + scan.summary.unreadable
                + scan.summary.changed,
            scan.summary.candidates
        );
        assert_eq!(scan.summary.parsed, 1);
        assert_eq!(scan.summary.malformed, 1);
        assert_eq!(scan.summary.unsupported, 1);
        assert_eq!(scan.summary.hash_computed, 3);
    }

    #[test]
    fn file_size_and_modified_time_are_observed() {
        let temp = TempDir::new();
        let bytes = valid_fixture("observed");
        temp.write("observed.gguf", &bytes);
        let envelope = scan(temp.path(), ScanOptions::default());
        let file = &envelope.scan.files[0];
        assert_eq!(
            file.file_size,
            Some(ghostram_core::quantity::Bytes::new(
                u64::try_from(bytes.len()).unwrap()
            ))
        );
        assert!(file.modified_at.is_some());
    }
}
