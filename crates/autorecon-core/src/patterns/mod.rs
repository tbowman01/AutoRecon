//! Pattern matching for scan output
//!
//! This module provides pattern matching functionality for identifying
//! interesting findings in scan output.

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::Result;

/// A pattern to match against scan output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    /// Description of what this pattern matches
    pub description: String,

    /// The regex pattern
    #[serde(with = "serde_regex")]
    pub pattern: Regex,

    /// Whether this pattern indicates a critical finding
    #[serde(default)]
    pub is_critical: bool,
}

impl Pattern {
    /// Create a new pattern
    ///
    /// # Arguments
    /// * `description` - Description of the pattern
    /// * `pattern` - Regex pattern string
    /// * `is_critical` - Whether this is a critical finding
    pub fn new(description: String, pattern: &str, is_critical: bool) -> Result<Self> {
        Ok(Pattern {
            description,
            pattern: Regex::new(pattern)?,
            is_critical,
        })
    }

    /// Check if the pattern matches the given text
    ///
    /// # Arguments
    /// * `text` - Text to search
    ///
    /// # Returns
    /// A vector of matches (empty if no matches)
    pub fn find_matches(&self, text: &str) -> Vec<PatternMatch> {
        self.pattern
            .find_iter(text)
            .map(|m| PatternMatch {
                description: self.description.clone(),
                matched_text: m.as_str().to_string(),
                start: m.start(),
                end: m.end(),
                is_critical: self.is_critical,
            })
            .collect()
    }

    /// Check if the pattern matches the text at least once
    pub fn is_match(&self, text: &str) -> bool {
        self.pattern.is_match(text)
    }
}

/// A matched pattern in scan output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatch {
    /// Description of the pattern that matched
    pub description: String,

    /// The actual text that was matched
    pub matched_text: String,

    /// Start position in the original text
    pub start: usize,

    /// End position in the original text
    pub end: usize,

    /// Whether this is a critical finding
    pub is_critical: bool,
}

/// Pattern matcher that applies multiple patterns to text
#[derive(Debug)]
pub struct PatternMatcher {
    /// Global patterns to apply to all output
    global_patterns: Vec<Pattern>,

    /// Scan-specific patterns (keyed by scan name)
    scan_patterns: std::collections::HashMap<String, Vec<Pattern>>,
}

impl PatternMatcher {
    /// Create a new pattern matcher
    pub fn new(
        global_patterns: Vec<Pattern>,
        scan_patterns: std::collections::HashMap<String, Vec<Pattern>>,
    ) -> Self {
        PatternMatcher {
            global_patterns,
            scan_patterns,
        }
    }

    /// Match patterns against text
    ///
    /// # Arguments
    /// * `text` - Text to search
    /// * `scan_name` - Optional scan name for scan-specific patterns
    ///
    /// # Returns
    /// A vector of all matches
    pub fn match_patterns(&self, text: &str, scan_name: Option<&str>) -> Vec<PatternMatch> {
        let mut matches = Vec::new();

        // Apply global patterns
        for pattern in &self.global_patterns {
            matches.extend(pattern.find_matches(text));
        }

        // Apply scan-specific patterns if provided
        if let Some(name) = scan_name {
            if let Some(patterns) = self.scan_patterns.get(name) {
                for pattern in patterns {
                    matches.extend(pattern.find_matches(text));
                }
            }
        }

        matches
    }

    /// Check if any pattern matches (faster than getting all matches)
    pub fn has_match(&self, text: &str, scan_name: Option<&str>) -> bool {
        // Check global patterns
        if self.global_patterns.iter().any(|p| p.is_match(text)) {
            return true;
        }

        // Check scan-specific patterns
        if let Some(name) = scan_name {
            if let Some(patterns) = self.scan_patterns.get(name) {
                if patterns.iter().any(|p| p.is_match(text)) {
                    return true;
                }
            }
        }

        false
    }
}

/// Serde support for Regex
mod serde_regex {
    use regex::Regex;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(regex: &Regex, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        regex.as_str().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Regex, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Regex::new(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_match() {
        let pattern = Pattern::new(
            "IP address".to_string(),
            r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b",
            false,
        )
        .unwrap();

        let matches = pattern.find_matches("Found 192.168.1.1 and 10.0.0.1");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].matched_text, "192.168.1.1");
        assert_eq!(matches[1].matched_text, "10.0.0.1");
    }

    #[test]
    fn test_pattern_is_match() {
        let pattern = Pattern::new("Test".to_string(), "test", false).unwrap();
        assert!(pattern.is_match("this is a test"));
        assert!(!pattern.is_match("no match here"));
    }

    #[test]
    fn test_pattern_matcher() {
        let global = vec![Pattern::new("Global".to_string(), "global", false).unwrap()];
        let mut scan_patterns = std::collections::HashMap::new();
        scan_patterns.insert(
            "scan1".to_string(),
            vec![Pattern::new("Scan1".to_string(), "scan1", false).unwrap()],
        );

        let matcher = PatternMatcher::new(global, scan_patterns);

        // Should match global pattern
        let matches = matcher.match_patterns("this is global", None);
        assert_eq!(matches.len(), 1);

        // Should match both global and scan-specific
        let matches = matcher.match_patterns("global and scan1", Some("scan1"));
        assert_eq!(matches.len(), 2);

        // Should only match global (different scan)
        let matches = matcher.match_patterns("global and scan1", Some("scan2"));
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_pattern_has_match() {
        let global = vec![Pattern::new("Test".to_string(), "test", false).unwrap()];
        let matcher = PatternMatcher::new(global, std::collections::HashMap::new());

        assert!(matcher.has_match("this is a test", None));
        assert!(!matcher.has_match("no match", None));
    }
}
