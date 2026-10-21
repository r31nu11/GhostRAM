//! Conservative filename hints for GGUF catalog records.

use crate::catalog::{FilenameHints, FilenameRole, ShardHint};

/// Extracts only documented filename conventions. These hints are not model
/// identity and never combine separate shards.
#[must_use]
pub fn hints_from_filename(filename: &str) -> FilenameHints {
    let Some(stem) = strip_gguf_suffix(filename) else {
        return FilenameHints::default();
    };
    let lower = stem.to_ascii_lowercase();
    let (base, shard) =
        split_shard(stem, &lower).map_or((stem, None), |(base, shard)| (base, Some(shard)));
    let base_lower = base.to_ascii_lowercase();

    let role = if base_lower.starts_with("mmproj-") {
        Some(FilenameRole::Mmproj)
    } else if base_lower.starts_with("mtp-") {
        Some(FilenameRole::Mtp)
    } else if base_lower.ends_with("-lora") {
        Some(FilenameRole::Lora)
    } else if base_lower.ends_with("-vocab") {
        Some(FilenameRole::Vocab)
    } else {
        None
    };

    FilenameHints { role, shard }
}

fn strip_gguf_suffix(filename: &str) -> Option<&str> {
    let suffix = ".gguf";
    if filename.len() < suffix.len() {
        return None;
    }
    let split = filename.len() - suffix.len();
    if filename
        .get(split..)
        .is_some_and(|end| end.eq_ignore_ascii_case(suffix))
    {
        filename.get(..split)
    } else {
        None
    }
}

fn split_shard<'a>(stem: &'a str, lower: &str) -> Option<(&'a str, ShardHint)> {
    let (left, total_text) = lower.rsplit_once("-of-")?;
    let total = parse_five_digits(total_text)?;
    if total == 0 {
        return None;
    }
    let (base_lower, number_text) = left.rsplit_once('-')?;
    let number = parse_five_digits(number_text)?;
    if number == 0 || number > total {
        return None;
    }
    let base = stem.get(..base_lower.len())?;
    Some((base, ShardHint { number, total }))
}

fn parse_five_digits(text: &str) -> Option<u32> {
    if text.len() != 5 || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::hints_from_filename;
    use crate::catalog::{FilenameRole, ShardHint};

    #[test]
    fn recognizes_sidecars_and_shards() {
        let hints = hints_from_filename("mmproj-model-f16-00001-of-00002.gguf");
        assert_eq!(hints.role, Some(FilenameRole::Mmproj));
        assert_eq!(
            hints.shard,
            Some(ShardHint {
                number: 1,
                total: 2
            })
        );
    }

    #[test]
    fn recognizes_mtp_and_adapter_names() {
        assert_eq!(
            hints_from_filename("mtp-model.gguf").role,
            Some(FilenameRole::Mtp)
        );
        assert_eq!(
            hints_from_filename("model-LoRA.gguf").role,
            Some(FilenameRole::Lora)
        );
        assert_eq!(
            hints_from_filename("model-vocab.gguf").role,
            Some(FilenameRole::Vocab)
        );
    }

    #[test]
    fn leaves_ordinary_names_unclassified() {
        let hints = hints_from_filename("llama-3.2-3b-instruct-q4_k_m.gguf");
        assert_eq!(hints, crate::catalog::FilenameHints::default());
    }

    #[test]
    fn ignores_malformed_or_impossible_shard_suffixes() {
        for name in [
            "model-00000-of-00002.gguf",
            "model-00003-of-00002.gguf",
            "model-1-of-2.gguf",
            "model-00001-of-00000.gguf",
            "model-00001-of-00002.bin",
        ] {
            assert_eq!(
                hints_from_filename(name).shard,
                None,
                "unexpected shard hint for {name}"
            );
        }
    }

    #[test]
    fn extension_matching_is_ascii_case_insensitive() {
        let hints = hints_from_filename("MMPROJ-model.GGUF");
        assert_eq!(hints.role, Some(FilenameRole::Mmproj));
    }
}
