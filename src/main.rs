mod blame;
mod cli;
mod filter;
mod output;
mod scanner;
mod types;

use std::path::Path;
use std::process;

use clap::Parser;

use crate::blame::{enrich_with_blame, is_git_repo};
use crate::cli::Cli;
use crate::filter::{Filters, apply_filters, sort_oldest_first};
use crate::output::{
    build_summary, format_json, format_markdown, format_summary, format_summary_json, format_table,
};
use crate::scanner::{hits_to_items, scan_directory};
use crate::types::CommentType;

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(&cli) {
        eprintln!("Error: {e}");
        process::exit(2);
    }
}

fn run(cli: &Cli) -> Result<(), String> {
    let path = Path::new(&cli.path);

    if !path.exists() {
        return Err(format!("Path does not exist: {}", cli.path));
    }
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", cli.path));
    }

    if cli.verbose {
        eprintln!("gundi: scanning {}", path.display());
    }

    // Scan for hits
    let scan = scan_directory(path)?;

    if cli.verbose {
        eprintln!(
            "gundi: scan complete — {} hit(s), {} file(s) skipped",
            scan.hits.len(),
            scan.skipped.len()
        );
        for (file, err) in &scan.skipped {
            eprintln!("gundi: skipped {}: {}", file.display(), err);
        }
    }

    let mut items = hits_to_items(scan.hits);

    // Enrich with git blame if available and not skipped
    if !cli.no_blame && is_git_repo(path) {
        let canonical = path
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize path: {e}"))?;
        if cli.verbose {
            eprintln!("gundi: enriching {} item(s) with git blame", items.len());
        }
        items = enrich_with_blame(items, &canonical);
    }

    // Build filters
    let type_filter = cli.types.as_ref().map(|types| {
        types
            .iter()
            .filter_map(|t| CommentType::from_str(t))
            .collect()
    });

    let filters = Filters {
        types: type_filter,
        author: cli.author.clone(),
        older_than: cli.older_than,
    };

    items = apply_filters(items, &filters);
    sort_oldest_first(&mut items);

    // Check fail-on gate before output
    let should_fail = cli.fail_on.and_then(|max_days| {
        items
            .iter()
            .any(|item| item.days_ago.is_some_and(|d| d >= max_days))
            .then_some(max_days)
    });

    // Output: --quiet suppresses results entirely (only fail-on exit code matters in CI).
    let suppress_output = cli.quiet && cli.fail_on.is_some();

    if !suppress_output {
        if cli.summary {
            let summary = build_summary(&items);
            if cli.json {
                let out = format_summary_json(&summary)?;
                println!("{out}");
            } else {
                println!("{}", format_summary(&summary));
            }
        } else if cli.json {
            let out = format_json(&items)?;
            println!("{out}");
        } else if cli.md {
            println!("{}", format_markdown(&items));
        } else {
            println!("{}", format_table(&items));
        }
    }

    // Exit nonzero if fail-on triggered
    if let Some(max_days) = should_fail {
        let count = items
            .iter()
            .filter(|i| i.days_ago.is_some_and(|d| d >= max_days))
            .count();
        if !cli.quiet {
            eprintln!("CI gate: {} item(s) aged {} days or more", count, max_days);
        }
        process::exit(1);
    }

    Ok(())
}
