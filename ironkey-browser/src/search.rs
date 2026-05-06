//! Recursive file search (by name glob and/or content pattern).

use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Options controlling a file search.
#[derive(Debug, Clone, Default)]
pub struct SearchOpts {
    /// Glob pattern for file names (e.g. "*.rs")
    pub name_glob: Option<String>,
    /// Substring/regex to find in file content
    pub content_pattern: Option<String>,
    /// Maximum directory depth (None = unlimited)
    pub max_depth: Option<usize>,
    /// Minimum file size in bytes
    pub min_size: Option<u64>,
    /// Maximum file size in bytes
    pub max_size: Option<u64>,
    /// Whether to follow symlinks
    pub follow_symlinks: bool,
}

/// A single search result.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Path to the matching file
    pub path: PathBuf,
    /// Matching line and its 1-based line number (for content search)
    pub match_line: Option<(usize, String)>,
}

/// Search `root` for files matching `opts`.
///
/// Returns matches lazily collected; may be slow on large filesystems.
pub fn search_directory(root: &Path, opts: &SearchOpts) -> Result<Vec<SearchResult>> {
    let mut results = Vec::new();

    let mut walker = WalkDir::new(root).follow_links(opts.follow_symlinks);
    if let Some(depth) = opts.max_depth {
        walker = walker.max_depth(depth);
    }

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        // Name filter
        if let Some(ref pattern) = opts.name_glob {
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if !glob_match(pattern, &file_name) {
                continue;
            }
        }

        // Size filters
        if let Ok(meta) = entry.metadata() {
            let size = meta.len();
            if let Some(min) = opts.min_size {
                if size < min {
                    continue;
                }
            }
            if let Some(max) = opts.max_size {
                if size > max {
                    continue;
                }
            }
        }

        // Content pattern
        if let Some(ref pattern) = opts.content_pattern {
            if let Ok(content) = std::fs::read_to_string(path) {
                let mut found = false;
                for (line_num, line) in content.lines().enumerate() {
                    if line.contains(pattern.as_str()) {
                        results.push(SearchResult {
                            path: path.to_path_buf(),
                            match_line: Some((line_num + 1, line.to_string())),
                        });
                        found = true;
                        break; // Report first match per file
                    }
                }
                if !found {
                    continue;
                }
            } else {
                // Not a text file; skip
                continue;
            }
        } else {
            results.push(SearchResult {
                path: path.to_path_buf(),
                match_line: None,
            });
        }
    }

    Ok(results)
}

/// Minimal glob matching supporting `*` and `?` wildcards.
fn glob_match(pattern: &str, name: &str) -> bool {
    // Case-insensitive comparison
    let pattern = pattern.to_lowercase();
    let name = name.to_lowercase();
    glob_match_inner(&pattern, &name)
}

fn glob_match_inner(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    let mut dp = vec![vec![false; n.len() + 1]; p.len() + 1];
    dp[0][0] = true;

    // Handle leading '*'
    for i in 1..=p.len() {
        if p[i - 1] == '*' {
            dp[i][0] = dp[i - 1][0];
        }
    }

    for i in 1..=p.len() {
        for j in 1..=n.len() {
            if p[i - 1] == '*' {
                dp[i][j] = dp[i - 1][j] || dp[i][j - 1];
            } else if p[i - 1] == '?' || p[i - 1] == n[j - 1] {
                dp[i][j] = dp[i - 1][j - 1];
            }
        }
    }

    dp[p.len()][n.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_star() {
        assert!(glob_match("*.rs", "main.rs"));
        assert!(!glob_match("*.rs", "main.py"));
    }

    #[test]
    fn glob_question() {
        // '?' matches exactly one character
        assert!(glob_match("ma?n.rs", "main.rs"));
        // Two characters where '?' expects one → no match
        assert!(!glob_match("ma?n.rs", "maiin.rs"));
    }

    #[test]
    fn glob_no_wildcard() {
        assert!(glob_match("main.rs", "main.rs"));
        assert!(!glob_match("main.rs", "main.py"));
    }

    #[test]
    fn glob_case_insensitive() {
        assert!(glob_match("*.RS", "main.rs"));
    }

    #[test]
    fn search_tmp() {
        let opts = SearchOpts {
            name_glob: Some("*.sh".to_string()),
            max_depth: Some(1),
            ..Default::default()
        };
        let _ = search_directory(Path::new("/tmp"), &opts);
    }
}
