use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use chrono::{NaiveDate, Utc};

use crate::types::DebtItem;

/// Enrich debt items with git blame information.
/// Runs git blame on each unique file and maps results to items.
pub fn enrich_with_blame(items: Vec<DebtItem>, repo_root: &Path) -> Vec<DebtItem> {
    // Group items by file to minimize git blame calls
    let mut by_file: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, item) in items.iter().enumerate() {
        by_file.entry(item.file.clone()).or_default().push(idx);
    }

    let mut enriched = items;
    let today = Utc::now().date_naive();

    for (file, indices) in &by_file {
        let blame_data = run_blame(repo_root, file);
        if let Ok(blame_lines) = blame_data {
            for &idx in indices {
                let line = enriched[idx].line;
                if let Some(info) = blame_lines.get(&line) {
                    enriched[idx].author = Some(info.author.clone());
                    enriched[idx].date = Some(info.date);
                    enriched[idx].days_ago = Some((today - info.date).num_days());
                }
            }
        }
    }

    enriched
}

#[derive(Debug)]
struct BlameInfo {
    author: String,
    date: NaiveDate,
}

/// Run git blame on a file and parse the output.
/// Returns a map of line number -> BlameInfo.
fn run_blame(repo_root: &Path, file: &str) -> Result<HashMap<usize, BlameInfo>, String> {
    let output = Command::new("git")
        .args(["blame", "--porcelain", file])
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("Failed to run git blame: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "git blame failed for {file}: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_porcelain_blame(&stdout)
}

/// Parse porcelain blame output into a line->BlameInfo map.
fn parse_porcelain_blame(output: &str) -> Result<HashMap<usize, BlameInfo>, String> {
    let mut result = HashMap::new();
    let mut current_line: Option<usize> = None;
    let mut current_author: Option<String> = None;
    let mut current_date: Option<NaiveDate> = None;

    for line in output.lines() {
        // Header line: <hash> <orig-line> <final-line> [<num-lines>]
        if line.len() >= 40 && line.chars().take(40).all(|c| c.is_ascii_hexdigit()) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                if let Ok(ln) = parts[2].parse::<usize>() {
                    // Save previous entry if we have one
                    if let (Some(prev_line), Some(author), Some(date)) =
                        (current_line, current_author.take(), current_date.take())
                    {
                        result.insert(prev_line, BlameInfo { author, date });
                    }
                    current_line = Some(ln);
                }
            }
        } else if let Some(rest) = line.strip_prefix("author ") {
            current_author = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("author-time ") {
            if let Ok(timestamp) = rest.parse::<i64>() {
                if let Some(dt) = chrono::DateTime::from_timestamp(timestamp, 0) {
                    current_date = Some(dt.date_naive());
                }
            }
        }
    }

    // Don't forget the last entry
    if let (Some(line_num), Some(author), Some(date)) =
        (current_line, current_author, current_date)
    {
        result.insert(line_num, BlameInfo { author, date });
    }

    Ok(result)
}

/// Check if a directory is inside a git repository.
pub fn is_git_repo(path: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(path)
        .output()
        .is_ok_and(|o| o.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_porcelain_blame() {
        let output = "\
abcdef1234567890abcdef1234567890abcdef12 1 1 1
author Alice
author-mail <alice@example.com>
author-time 1700000000
author-tz +0000
committer Alice
committer-mail <alice@example.com>
committer-time 1700000000
committer-tz +0000
summary Initial commit
filename test.rs
\t// TODO: fix this
";
        let result = parse_porcelain_blame(output).unwrap();
        assert_eq!(result.len(), 1);
        let info = result.get(&1).unwrap();
        assert_eq!(info.author, "Alice");
        assert_eq!(info.date, NaiveDate::from_ymd_opt(2023, 11, 14).unwrap());
    }

    #[test]
    fn test_parse_empty_blame() {
        let result = parse_porcelain_blame("").unwrap();
        assert!(result.is_empty());
    }
}
