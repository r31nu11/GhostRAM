//! Filesystem capacity discovery through a bounded `df` invocation.

use std::time::Duration;

use ghostram_core::diagnostic::{CategoryReport, StorageInfo};
use ghostram_core::quantity::Bytes;

use crate::command::run_bounded;
use crate::tools::find_executable;

const DF_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) fn collect_storage() -> (Vec<StorageInfo>, CategoryReport) {
    let Some(df) = find_executable("df") else {
        return (
            Vec::new(),
            CategoryReport::unavailable("df was not found on PATH"),
        );
    };

    let observation = match run_bounded(&df, &["-P", "-B1", "/"], DF_TIMEOUT) {
        Ok(observation) => observation,
        Err(error) => {
            return (
                Vec::new(),
                CategoryReport::unavailable(format!("df could not be started: {error}")),
            );
        }
    };

    if observation.timed_out {
        return (
            Vec::new(),
            CategoryReport::unavailable("df exceeded the 2 second timeout"),
        );
    }
    if !observation.succeeded() {
        let detail = observation
            .first_output_line()
            .unwrap_or_else(|| "no output".to_string());
        return (
            Vec::new(),
            CategoryReport::unavailable(format!("df failed: {detail}")),
        );
    }

    match parse_df(&observation.stdout) {
        Ok(storage) => {
            let report = if storage.is_empty() {
                CategoryReport::unavailable("df returned no filesystem rows")
            } else {
                CategoryReport::available()
            };
            (storage, report)
        }
        Err(error) => (
            Vec::new(),
            CategoryReport::unavailable(format!("df output could not be parsed: {error}")),
        ),
    }
}

fn parse_df(text: &str) -> Result<Vec<StorageInfo>, String> {
    let mut rows = Vec::new();
    for line in text.lines().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 6 {
            return Err(format!("row has fewer than 6 fields ({})", fields.len()));
        }
        let total = fields[1]
            .parse::<u64>()
            .map_err(|_| "total size is not a byte count".to_string())?;
        let used = fields[2]
            .parse::<u64>()
            .map_err(|_| "used size is not a byte count".to_string())?;
        let available = fields[3]
            .parse::<u64>()
            .map_err(|_| "available size is not a byte count".to_string())?;
        rows.push(StorageInfo {
            filesystem: fields[0].to_string(),
            mount_point: fields[5].to_string(),
            total_bytes: Bytes::new(total),
            used_bytes: Bytes::new(used),
            available_bytes: Bytes::new(available),
        });
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::parse_df;
    use ghostram_core::quantity::Bytes;

    #[test]
    fn parses_df_byte_counts() {
        let rows = parse_df(include_str!("../tests/fixtures/df.txt")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].filesystem, "/dev/sdb2");
        assert_eq!(rows[0].mount_point, "/");
        assert_eq!(rows[0].total_bytes, Bytes::new(124_895_825_920));
        assert_eq!(rows[0].used_bytes, Bytes::new(74_701_832_192));
        assert_eq!(rows[0].available_bytes, Bytes::new(43_802_386_432));
    }

    #[test]
    fn rejects_a_truncated_df_row() {
        let error =
            parse_df("Filesystem 1-blocks Used Available Capacity Mounted on\n/dev/sdb2 1 2\n")
                .unwrap_err();
        assert!(error.contains("fewer"));
    }

    #[test]
    fn rejects_non_numeric_sizes() {
        let error = parse_df(
            "Filesystem 1-blocks Used Available Capacity Mounted on\n/dev/sdb2 no 2 3 4% /\n",
        )
        .unwrap_err();
        assert!(error.contains("byte count"));
    }
}
