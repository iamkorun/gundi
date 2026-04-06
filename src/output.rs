use std::collections::HashMap;

use colored::Colorize;

use crate::types::{DebtItem, ScanSummary};

/// Truncate a string to at most `max_chars` characters, appending `…` if truncated.
/// Operates on Unicode scalar values, not bytes — safe for multi-byte input.
fn truncate_chars(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        return s.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let truncated: String = s.chars().take(keep).collect();
    format!("{}…", truncated)
}

/// Truncate a string from the start, keeping the last `max_chars` characters.
/// Useful for file paths where the tail (filename) matters more than the head.
fn truncate_chars_end(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        return s.to_string();
    }
    let skip = char_count - max_chars.saturating_sub(1);
    let tail: String = s.chars().skip(skip).collect();
    format!("…{}", tail)
}

/// Format items as a colored terminal table.
pub fn format_table(items: &[DebtItem]) -> String {
    if items.is_empty() {
        return "No code debt found. Clean codebase!".to_string();
    }

    let mut lines = Vec::new();

    // Header
    lines.push(format!(
        "{:<6} {:<40} {:<6} {:<20} {:<12} {}",
        "TYPE", "FILE", "LINE", "AUTHOR", "AGE", "TEXT"
    ));
    lines.push("-".repeat(110));

    for item in items {
        let age_str = match item.days_ago {
            Some(d) => format!("{}d ago", d),
            None => "unknown".to_string(),
        };

        let author_str = item.author.as_deref().unwrap_or("unknown");
        let author_display = truncate_chars(author_str, 18);
        let file_display = truncate_chars_end(&item.file, 38);
        let text_display = truncate_chars(&item.text, 50);

        let row = format!(
            "{:<6} {:<40} {:<6} {:<20} {:<12} {}",
            item.comment_type.label(),
            file_display,
            item.line,
            author_display,
            age_str,
            text_display,
        );

        let colored_row = match item.days_ago {
            Some(d) if d > 90 => row.red().to_string(),
            Some(d) if d > 30 => row.yellow().to_string(),
            _ => row,
        };

        lines.push(colored_row);
    }

    lines.push(String::new());
    lines.push(format!("Total: {} items", items.len()));

    lines.join("\n")
}

/// Format items as JSON.
pub fn format_json(items: &[DebtItem]) -> Result<String, String> {
    serde_json::to_string_pretty(items).map_err(|e| format!("JSON serialization failed: {e}"))
}

/// Format items as a Markdown table.
pub fn format_markdown(items: &[DebtItem]) -> String {
    if items.is_empty() {
        return "No code debt found.".to_string();
    }

    let mut lines = Vec::new();
    lines.push("| Type | File | Line | Author | Age | Text |".to_string());
    lines.push("|------|------|------|--------|-----|------|".to_string());

    for item in items {
        let age_str = match item.days_ago {
            Some(d) => format!("{}d", d),
            None => "?".to_string(),
        };
        let author = item.author.as_deref().unwrap_or("unknown");

        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} |",
            item.comment_type.label(),
            item.file,
            item.line,
            author,
            age_str,
            item.text,
        ));
    }

    lines.push(String::new());
    lines.push(format!("**Total: {} items**", items.len()));

    lines.join("\n")
}

/// Build a scan summary from items.
pub fn build_summary(items: &[DebtItem]) -> ScanSummary {
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    let mut author_counts: HashMap<String, usize> = HashMap::new();
    let mut oldest: Option<i64> = None;
    let mut newest: Option<i64> = None;

    for item in items {
        *type_counts
            .entry(item.comment_type.label().to_string())
            .or_default() += 1;
        let author = item.author.as_deref().unwrap_or("unknown").to_string();
        *author_counts.entry(author).or_default() += 1;

        if let Some(days) = item.days_ago {
            oldest = Some(oldest.map_or(days, |o: i64| o.max(days)));
            newest = Some(newest.map_or(days, |n: i64| n.min(days)));
        }
    }

    let mut by_type: Vec<(String, usize)> = type_counts.into_iter().collect();
    by_type.sort_by(|a, b| b.1.cmp(&a.1));

    let mut by_author: Vec<(String, usize)> = author_counts.into_iter().collect();
    by_author.sort_by(|a, b| b.1.cmp(&a.1));

    ScanSummary {
        total: items.len(),
        by_type,
        by_author,
        oldest_days: oldest,
        newest_days: newest,
    }
}

/// Format a summary for terminal display.
pub fn format_summary(summary: &ScanSummary) -> String {
    let mut lines = Vec::new();

    lines.push(format!("Code Debt Summary: {} total items", summary.total));
    lines.push(String::new());

    if !summary.by_type.is_empty() {
        lines.push("By Type:".to_string());
        for (t, count) in &summary.by_type {
            lines.push(format!("  {:<8} {}", t, count));
        }
        lines.push(String::new());
    }

    if !summary.by_author.is_empty() {
        lines.push("By Author:".to_string());
        for (author, count) in &summary.by_author {
            lines.push(format!("  {:<20} {}", author, count));
        }
        lines.push(String::new());
    }

    if let (Some(oldest), Some(newest)) = (summary.oldest_days, summary.newest_days) {
        lines.push(format!("Age range: {}d - {}d", newest, oldest));
    }

    lines.join("\n")
}

/// Format a summary as JSON.
pub fn format_summary_json(summary: &ScanSummary) -> Result<String, String> {
    serde_json::to_string_pretty(summary).map_err(|e| format!("JSON serialization failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CommentType;
    use chrono::NaiveDate;

    fn make_item(ctype: CommentType, author: &str, days: i64) -> DebtItem {
        DebtItem {
            file: "src/main.rs".to_string(),
            line: 1,
            comment_type: ctype,
            text: "fix this".to_string(),
            author: Some(author.to_string()),
            date: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            days_ago: Some(days),
        }
    }

    #[test]
    fn test_format_table_empty() {
        let result = format_table(&[]);
        assert!(result.contains("No code debt found"));
    }

    #[test]
    fn test_format_table_with_items() {
        let items = vec![make_item(CommentType::Todo, "Alice", 10)];
        let result = format_table(&items);
        assert!(result.contains("TODO"));
        assert!(result.contains("src/main.rs"));
        assert!(result.contains("Total: 1 items"));
    }

    #[test]
    fn test_format_json() {
        let items = vec![make_item(CommentType::Todo, "Alice", 10)];
        let result = format_json(&items).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["comment_type"], "TODO");
    }

    #[test]
    fn test_format_markdown() {
        let items = vec![make_item(CommentType::Fixme, "Bob", 50)];
        let result = format_markdown(&items);
        assert!(result.contains("| FIXME |"));
        assert!(result.contains("Bob"));
        assert!(result.contains("**Total: 1 items**"));
    }

    #[test]
    fn test_format_markdown_empty() {
        let result = format_markdown(&[]);
        assert!(result.contains("No code debt found"));
    }

    #[test]
    fn test_build_summary() {
        let items = vec![
            make_item(CommentType::Todo, "Alice", 10),
            make_item(CommentType::Todo, "Bob", 100),
            make_item(CommentType::Fixme, "Alice", 50),
        ];

        let summary = build_summary(&items);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.oldest_days, Some(100));
        assert_eq!(summary.newest_days, Some(10));
        assert_eq!(summary.by_type[0].0, "TODO");
        assert_eq!(summary.by_type[0].1, 2);
    }

    #[test]
    fn test_format_summary() {
        let items = vec![
            make_item(CommentType::Todo, "Alice", 10),
            make_item(CommentType::Fixme, "Bob", 50),
        ];
        let summary = build_summary(&items);
        let output = format_summary(&summary);
        assert!(output.contains("2 total items"));
        assert!(output.contains("By Type:"));
        assert!(output.contains("By Author:"));
    }

    #[test]
    fn test_truncate_chars_short() {
        assert_eq!(truncate_chars("hello", 10), "hello");
        assert_eq!(truncate_chars("", 10), "");
    }

    #[test]
    fn test_truncate_chars_long_ascii() {
        let result = truncate_chars("abcdefghijklmnop", 10);
        assert!(result.ends_with('…'));
        assert_eq!(result.chars().count(), 10);
    }

    #[test]
    fn test_truncate_chars_multi_byte_safe() {
        // 한국어 are 3-byte UTF-8 characters; byte slicing would split them.
        // truncate_chars must operate on chars, not bytes.
        let input = "한국어한국어한국어한국어";
        let result = truncate_chars(input, 5);
        assert_eq!(result.chars().count(), 5);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn test_truncate_chars_end_keeps_tail() {
        // Long file paths should keep the filename (tail) visible.
        let result = truncate_chars_end("very/deeply/nested/path/to/file.rs", 15);
        assert!(result.starts_with('…'));
        assert!(result.ends_with("file.rs"));
        assert_eq!(result.chars().count(), 15);
    }

    #[test]
    fn test_format_table_with_unicode_does_not_panic() {
        // Regression: byte slicing previously panicked on multi-byte boundaries.
        let item = DebtItem {
            file: "src/한국어/very/long/path/to/some/deeply/nested/file.rs".to_string(),
            line: 42,
            comment_type: CommentType::Todo,
            text: "修复这个非常非常非常长的注释 with αβγ and 🦀 emoji to ensure no panic"
                .to_string(),
            author: Some("作者非常长的名字".to_string()),
            date: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            days_ago: Some(100),
        };
        let result = format_table(&[item]);
        assert!(result.contains("TODO"));
        assert!(result.contains("Total: 1 items"));
    }
}
