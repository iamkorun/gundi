use crate::types::{CommentType, DebtItem};

/// Filter criteria for debt items.
#[derive(Debug, Clone, Default)]
pub struct Filters {
    /// Filter by comment types (e.g., ["TODO", "FIXME"])
    pub types: Option<Vec<CommentType>>,
    /// Filter by author name (case-insensitive substring match)
    pub author: Option<String>,
    /// Only show items older than N days
    pub older_than: Option<i64>,
}

/// Apply filters to a list of debt items, returning a new filtered list.
pub fn apply_filters(items: Vec<DebtItem>, filters: &Filters) -> Vec<DebtItem> {
    items
        .into_iter()
        .filter(|item| {
            // Type filter
            if let Some(ref types) = filters.types
                && !types.contains(&item.comment_type)
            {
                return false;
            }

            // Author filter (case-insensitive substring)
            if let Some(ref author_filter) = filters.author {
                match &item.author {
                    Some(author) => {
                        if !author.to_lowercase().contains(&author_filter.to_lowercase()) {
                            return false;
                        }
                    }
                    None => return false,
                }
            }

            // Age filter
            if let Some(min_days) = filters.older_than {
                match item.days_ago {
                    Some(days) => {
                        if days < min_days {
                            return false;
                        }
                    }
                    None => return false,
                }
            }

            true
        })
        .collect()
}

/// Sort items oldest-first (items without dates go last).
pub fn sort_oldest_first(items: &mut [DebtItem]) {
    items.sort_by(|a, b| {
        let a_days = a.days_ago.unwrap_or(-1);
        let b_days = b.days_ago.unwrap_or(-1);
        b_days.cmp(&a_days)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn make_item(ctype: CommentType, author: Option<&str>, days: Option<i64>) -> DebtItem {
        DebtItem {
            file: "test.rs".to_string(),
            line: 1,
            comment_type: ctype,
            text: "test".to_string(),
            author: author.map(String::from),
            date: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            days_ago: days,
        }
    }

    #[test]
    fn test_filter_by_type() {
        let items = vec![
            make_item(CommentType::Todo, Some("Alice"), Some(10)),
            make_item(CommentType::Fixme, Some("Bob"), Some(20)),
            make_item(CommentType::Hack, Some("Alice"), Some(30)),
        ];

        let filters = Filters {
            types: Some(vec![CommentType::Todo, CommentType::Hack]),
            ..Default::default()
        };

        let result = apply_filters(items, &filters);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].comment_type, CommentType::Todo);
        assert_eq!(result[1].comment_type, CommentType::Hack);
    }

    #[test]
    fn test_filter_by_author() {
        let items = vec![
            make_item(CommentType::Todo, Some("Alice Smith"), Some(10)),
            make_item(CommentType::Fixme, Some("Bob Jones"), Some(20)),
        ];

        let filters = Filters {
            author: Some("alice".to_string()),
            ..Default::default()
        };

        let result = apply_filters(items, &filters);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].author.as_deref(), Some("Alice Smith"));
    }

    #[test]
    fn test_filter_by_age() {
        let items = vec![
            make_item(CommentType::Todo, Some("Alice"), Some(10)),
            make_item(CommentType::Fixme, Some("Bob"), Some(50)),
            make_item(CommentType::Hack, Some("Charlie"), Some(100)),
        ];

        let filters = Filters {
            older_than: Some(30),
            ..Default::default()
        };

        let result = apply_filters(items, &filters);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_combined_filters() {
        let items = vec![
            make_item(CommentType::Todo, Some("Alice"), Some(100)),
            make_item(CommentType::Todo, Some("Bob"), Some(5)),
            make_item(CommentType::Fixme, Some("Alice"), Some(100)),
        ];

        let filters = Filters {
            types: Some(vec![CommentType::Todo]),
            author: Some("alice".to_string()),
            older_than: Some(30),
        };

        let result = apply_filters(items, &filters);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].comment_type, CommentType::Todo);
        assert_eq!(result[0].author.as_deref(), Some("Alice"));
    }

    #[test]
    fn test_no_filters() {
        let items = vec![
            make_item(CommentType::Todo, Some("Alice"), Some(10)),
            make_item(CommentType::Fixme, Some("Bob"), Some(20)),
        ];

        let filters = Filters::default();
        let result = apply_filters(items, &filters);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_sort_oldest_first() {
        let mut items = vec![
            make_item(CommentType::Todo, Some("A"), Some(10)),
            make_item(CommentType::Fixme, Some("B"), Some(100)),
            make_item(CommentType::Hack, Some("C"), Some(50)),
            make_item(CommentType::Bug, Some("D"), None),
        ];

        sort_oldest_first(&mut items);

        assert_eq!(items[0].days_ago, Some(100));
        assert_eq!(items[1].days_ago, Some(50));
        assert_eq!(items[2].days_ago, Some(10));
        assert_eq!(items[3].days_ago, None);
    }
}
