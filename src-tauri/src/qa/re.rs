//! Compiled-regex cache for QA checks and fixers.
//!
//! The checks run once per segment over a whole volume (thousands of segments), and several
//! build patterns from word lists. Compiling those on every call made one scan take tens of
//! seconds; compiling each distinct pattern once keeps a full scan well under NFR-06.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use regex::Regex;

static CACHE: LazyLock<Mutex<HashMap<String, Regex>>> = LazyLock::new(Default::default);

/// Returns the compiled regex for `pattern`, compiling it only the first time.
/// `None` for an invalid pattern, matching how call sites treated `Regex::new` errors.
pub fn cached(pattern: &str) -> Option<Regex> {
    let mut map = CACHE.lock().ok()?;
    if let Some(re) = map.get(pattern) {
        return Some(re.clone());
    }
    let re = Regex::new(pattern).ok()?;
    map.insert(pattern.to_string(), re.clone());
    Some(re)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caches_valid_patterns_and_rejects_invalid_ones() {
        assert!(cached(r"\bkata\b").is_some_and(|re| re.is_match("satu kata")));
        assert!(cached(r"\bkata\b").is_some());
        assert!(cached(r"(unclosed").is_none());
    }
}
