use clap::Parser;

/// Unearth your forgotten code debt — TODO scanner with git blame age tracking.
#[derive(Parser, Debug)]
#[command(name = "gundi", version, about, long_about = None)]
pub struct Cli {
    /// Directory to scan (defaults to current directory)
    #[arg(default_value = ".")]
    pub path: String,

    /// Filter by comment types (comma-separated: todo,fixme,hack,bug,xxx)
    #[arg(long = "type", value_delimiter = ',')]
    pub types: Option<Vec<String>>,

    /// Filter by author name (case-insensitive substring match)
    #[arg(long)]
    pub author: Option<String>,

    /// Only show items older than N days
    #[arg(long)]
    pub older_than: Option<i64>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Output as Markdown
    #[arg(long)]
    pub md: bool,

    /// Show summary only (counts by type and author)
    #[arg(long)]
    pub summary: bool,

    /// Exit with code 1 if any item is older than N days (CI gate)
    #[arg(long)]
    pub fail_on: Option<i64>,

    /// Skip git blame (faster, no age/author info)
    #[arg(long)]
    pub no_blame: bool,
}
