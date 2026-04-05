use chrono::NaiveDate;
use serde::Serialize;

/// The type of code debt comment found.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CommentType {
    Todo,
    Fixme,
    Hack,
    Bug,
    Xxx,
}

impl CommentType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "TODO" => Some(Self::Todo),
            "FIXME" => Some(Self::Fixme),
            "HACK" => Some(Self::Hack),
            "BUG" => Some(Self::Bug),
            "XXX" => Some(Self::Xxx),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Todo => "TODO",
            Self::Fixme => "FIXME",
            Self::Hack => "HACK",
            Self::Bug => "BUG",
            Self::Xxx => "XXX",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![Self::Todo, Self::Fixme, Self::Hack, Self::Bug, Self::Xxx]
    }
}

impl std::fmt::Display for CommentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// A single code debt hit with blame information.
#[derive(Debug, Clone, Serialize)]
pub struct DebtItem {
    pub file: String,
    pub line: usize,
    pub comment_type: CommentType,
    pub text: String,
    pub author: Option<String>,
    pub date: Option<NaiveDate>,
    pub days_ago: Option<i64>,
}

/// Summary statistics for a scan.
#[derive(Debug, Clone, Serialize)]
pub struct ScanSummary {
    pub total: usize,
    pub by_type: Vec<(String, usize)>,
    pub by_author: Vec<(String, usize)>,
    pub oldest_days: Option<i64>,
    pub newest_days: Option<i64>,
}
