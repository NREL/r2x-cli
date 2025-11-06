//! Shared utility functions for plugin discovery

/// Check if a dependency looks like an r2x plugin
pub fn looks_like_r2x_plugin(dep: &str) -> bool {
    dep.starts_with("r2x-") || dep == "r2x" || dep.contains("plugin")
}

/// Find the matching closing delimiter for an opening delimiter at `start` position
pub fn find_matching_delimiter(content: &str, start: usize, open: char, close: char) -> Option<usize> {
    if start >= content.len() {
        return None;
    }

    let chars: Vec<char> = content.chars().collect();
    if chars[start] != open {
        return None;
    }

    let mut count = 0;
    for (i, &ch) in chars.iter().enumerate().skip(start) {
        if ch == open {
            count += 1;
        } else if ch == close {
            count -= 1;
            if count == 0 {
                return Some(i);
            }
        }
    }

    None
}

/// Find the matching closing parenthesis for an opening parenthesis at `start`
pub fn find_matching_paren(content: &str, start: usize) -> Option<usize> {
    find_matching_delimiter(content, start, '(', ')')
}

/// Find the matching closing bracket for an opening bracket at `start`
pub fn find_matching_bracket(content: &str, start: usize) -> Option<usize> {
    find_matching_delimiter(content, start, '[', ']')
}
