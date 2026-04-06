use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ignore::WalkBuilder;
use rayon::prelude::*;
use regex::Regex;

use crate::types::{CommentType, DebtItem};

/// A match found in a file before blame enrichment.
#[derive(Debug, Clone)]
pub struct RawHit {
    pub file: PathBuf,
    pub line: usize,
    pub comment_type: CommentType,
    pub text: String,
}

/// Result of scanning a directory: hits plus paths that could not be read.
#[derive(Debug, Default)]
pub struct ScanResult {
    pub hits: Vec<RawHit>,
    pub skipped: Vec<(PathBuf, String)>,
}

/// Scan a directory for TODO/FIXME/HACK/BUG/XXX comments.
/// Respects .gitignore via the `ignore` crate.
///
/// Files that fail to read (binaries, permission errors, invalid UTF-8) are
/// collected in `skipped` rather than aborting the whole scan.
pub fn scan_directory(root: &Path) -> Result<ScanResult, String> {
    let pattern = Regex::new(r"(?i)\b(TODO|FIXME|HACK|BUG|XXX)\b[:\s]*(.*)")
        .map_err(|e| format!("Failed to compile regex: {e}"))?;

    let entries: Vec<PathBuf> = WalkBuilder::new(root)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_some_and(|ft| ft.is_file()))
        .map(|entry| entry.into_path())
        .collect();

    let hits = Mutex::new(Vec::new());
    let skipped = Mutex::new(Vec::new());

    entries
        .par_iter()
        .for_each(|path| match scan_file(path, root, &pattern) {
            Ok(file_hits) => {
                if let Ok(mut locked) = hits.lock() {
                    locked.extend(file_hits);
                }
            }
            Err(e) => {
                if let Ok(mut locked) = skipped.lock() {
                    locked.push((path.clone(), e));
                }
            }
        });

    Ok(ScanResult {
        hits: hits.into_inner().unwrap_or_default(),
        skipped: skipped.into_inner().unwrap_or_default(),
    })
}

fn scan_file(path: &Path, root: &Path, pattern: &Regex) -> Result<Vec<RawHit>, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    let mut hits = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        for cap in pattern.captures_iter(line) {
            let Some(tag_match) = cap.get(1) else {
                continue;
            };
            let text = cap
                .get(2)
                .map(|m| m.as_str().trim())
                .unwrap_or("")
                .to_string();

            if let Some(comment_type) = CommentType::from_str(tag_match.as_str()) {
                hits.push(RawHit {
                    file: PathBuf::from(&relative),
                    line: line_num + 1,
                    comment_type,
                    text,
                });
            }
        }
    }

    Ok(hits)
}

/// Convert raw hits to debt items (without blame info).
pub fn hits_to_items(hits: Vec<RawHit>) -> Vec<DebtItem> {
    hits.into_iter()
        .map(|h| DebtItem {
            file: h.file.to_string_lossy().to_string(),
            line: h.line,
            comment_type: h.comment_type,
            text: h.text,
            author: None,
            date: None,
            days_ago: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_finds_todo() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.rs");
        fs::write(
            &file,
            "// TODO: fix this later\nlet x = 1;\n// FIXME: broken\n",
        )
        .unwrap();

        let result = scan_directory(dir.path()).unwrap();
        assert_eq!(result.hits.len(), 2);

        let types: Vec<_> = result.hits.iter().map(|h| h.comment_type.label()).collect();
        assert!(types.contains(&"TODO"));
        assert!(types.contains(&"FIXME"));
    }

    #[test]
    fn test_scan_finds_all_types() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.py");
        fs::write(
            &file,
            "# TODO: one\n# FIXME: two\n# HACK: three\n# BUG: four\n# XXX: five\n",
        )
        .unwrap();

        let result = scan_directory(dir.path()).unwrap();
        assert_eq!(result.hits.len(), 5);
    }

    #[test]
    fn test_scan_empty_directory() {
        let dir = TempDir::new().unwrap();
        let result = scan_directory(dir.path()).unwrap();
        assert!(result.hits.is_empty());
        assert!(result.skipped.is_empty());
    }

    #[test]
    fn test_scan_no_matches() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("clean.rs");
        fs::write(&file, "fn main() {\n    println!(\"hello\");\n}\n").unwrap();

        let result = scan_directory(dir.path()).unwrap();
        assert!(result.hits.is_empty());
    }

    #[test]
    fn test_scan_captures_text() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.rs");
        fs::write(&file, "// TODO: implement error handling\n").unwrap();

        let result = scan_directory(dir.path()).unwrap();
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].text, "implement error handling");
    }

    #[test]
    fn test_scan_skips_binary_files() {
        // Binary content with invalid UTF-8 should be reported in skipped, not crash.
        let dir = TempDir::new().unwrap();
        let bin_file = dir.path().join("data.bin");
        fs::write(&bin_file, [0xFF, 0xFE, 0x00, 0x01, 0x02]).unwrap();

        let text_file = dir.path().join("test.rs");
        fs::write(&text_file, "// TODO: still found\n").unwrap();

        let result = scan_directory(dir.path()).unwrap();
        // Text file's TODO is still found
        assert_eq!(result.hits.len(), 1);
        // Binary file is reported as skipped, not silently dropped
        assert_eq!(result.skipped.len(), 1);
        assert!(result.skipped[0].0.ends_with("data.bin"));
    }

    #[test]
    fn test_scan_unicode_content() {
        // Multi-byte UTF-8 in comments must not panic anywhere downstream.
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("unicode.rs");
        fs::write(
            &file,
            "// TODO: 修复这个问题 — handle 한국어 input αβγ 🦀\n",
        )
        .unwrap();

        let result = scan_directory(dir.path()).unwrap();
        assert_eq!(result.hits.len(), 1);
        assert!(result.hits[0].text.contains("修复"));
        assert!(result.hits[0].text.contains("🦀"));
    }

    #[test]
    fn test_hits_to_items() {
        let hits = vec![RawHit {
            file: PathBuf::from("src/main.rs"),
            line: 10,
            comment_type: CommentType::Todo,
            text: "fix this".to_string(),
        }];

        let items = hits_to_items(hits);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].file, "src/main.rs");
        assert!(items[0].author.is_none());
        assert!(items[0].days_ago.is_none());
    }
}
