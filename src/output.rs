use std::collections::HashMap;

use colored::Colorize;

use crate::types::{DebtItem, ScanSummary};

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
        let author_display = if author_str.len() > 18 {
            format!("{}...", &author_str[..15])
        } else {
            author_str.to_string()
        };

        let file_display = if item.file.len() > 38 {
            format!("...{}", &item.file[item.file.len() - 35..])
        } else {
            item.file.clone()
        };

        let text_display = if item.text.len() > 50 {
            format!("{}...", &item.text[..47])
        } else {
            item.text.clone()
        };

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
}
