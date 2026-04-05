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

/// Scan a directory for TODO/FIXME/HACK/BUG/XXX comments.
/// Respects .gitignore via the `ignore` crate.
pub fn scan_directory(root: &Path) -> Result<Vec<RawHit>, String> {
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

    entries.par_iter().for_each(|path| {
        if let Ok(file_hits) = scan_file(path, root, &pattern) {
            let mut locked = hits.lock().unwrap();
            locked.extend(file_hits);
        }
    });

    Ok(hits.into_inner().unwrap())
}

fn scan_file(path: &Path, root: &Path, pattern: &Regex) -> Result<Vec<RawHit>, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    let mut hits = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        for cap in pattern.captures_iter(line) {
            let tag = cap.get(1).unwrap().as_str();
            let text = cap.get(2).map(|m| m.as_str().trim()).unwrap_or("").to_string();

            if let Some(comment_type) = CommentType::from_str(tag) {
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
        fs::write(&file, "// TODO: fix this later\nlet x = 1;\n// FIXME: broken\n").unwrap();

        let hits = scan_directory(dir.path()).unwrap();
        assert_eq!(hits.len(), 2);

        let types: Vec<_> = hits.iter().map(|h| h.comment_type.label()).collect();
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

        let hits = scan_directory(dir.path()).unwrap();
        assert_eq!(hits.len(), 5);
    }

    #[test]
    fn test_scan_empty_directory() {
        let dir = TempDir::new().unwrap();
        let hits = scan_directory(dir.path()).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn test_scan_no_matches() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("clean.rs");
        fs::write(&file, "fn main() {\n    println!(\"hello\");\n}\n").unwrap();

        let hits = scan_directory(dir.path()).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn test_scan_captures_text() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.rs");
        fs::write(&file, "// TODO: implement error handling\n").unwrap();

        let hits = scan_directory(dir.path()).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].text, "implement error handling");
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
