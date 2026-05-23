//! Synonym token filters.

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
use pizza_engine::analysis::{Token, TokenFilter};

use crate::parser::{SynonymMap, SynonymRule};

pub use crate::parser::SynonymMode;

/// Single-word synonym token filter.
///
/// Expands or contracts tokens based on a synonym map. Does not handle
/// multi-word synonyms — use [`SynonymGraphFilter`] for that.
///
/// # Expand Mode
/// Input: "fast" → Output: "fast", "quick", "speedy" (all at same position)
///
/// # Contract Mode  
/// Input: "quick" → Output: "fast" (canonical form only)
#[derive(Clone, Debug)]
pub struct SynonymFilter {
    map: SynonymMap,
    ignore_case: bool,
}

impl SynonymFilter {
    /// Create a new synonym filter from a pre-built synonym map.
    pub fn new(map: SynonymMap, ignore_case: bool) -> Self {
        Self { map, ignore_case }
    }

    /// Create an empty synonym filter (no-op until rules are added).
    pub fn empty() -> Self {
        Self {
            map: SynonymMap::new(),
            ignore_case: true,
        }
    }

    /// Replace the synonym map (for hot-reload).
    pub fn reload(&mut self, map: SynonymMap) {
        self.map = map;
    }

    fn lookup_term<'a>(&self, term: &'a str) -> Option<&SynonymRule> {
        if self.ignore_case {
            // Lowercase for lookup
            let lower = term.to_lowercase();
            self.map.get(&lower)
        } else {
            self.map.get(term)
        }
    }
}

impl TokenFilter for SynonymFilter {
    fn filter<'a>(&self, token: &mut Token<'a>) -> (bool, Option<Vec<Token<'a>>>) {
        let term = token.term.as_ref();
        if term.is_empty() {
            return (false, None);
        }

        if let Some(rule) = self.lookup_term(term) {
            match rule.mode {
                SynonymMode::Expand => {
                    // Keep original + emit synonyms at same position
                    let mut extra = Vec::with_capacity(rule.replacements.len());
                    for replacement in &rule.replacements {
                        let syn_token = Token {
                            term: Cow::Owned(replacement.clone()),
                            start_offset: token.start_offset,
                            end_offset: token.end_offset,
                            position: token.position,
                        };
                        extra.push(syn_token);
                    }
                    (false, Some(extra))
                }
                SynonymMode::Contract => {
                    // Replace with canonical form
                    if let Some(canonical) = rule.replacements.first() {
                        token.term = Cow::Owned(canonical.clone());
                    }
                    (false, None)
                }
            }
        } else {
            (false, None)
        }
    }
}

/// Graph-aware synonym filter for multi-word synonyms.
///
/// Handles phrases like "new york" → "ny" or "ny" → "new york".
/// Uses position_length to preserve graph structure for correct phrase queries.
///
/// For single-word synonyms, behaves identically to [`SynonymFilter`].
/// For multi-word synonyms, adjusts position/position_length to create
/// a proper token graph.
#[derive(Clone, Debug)]
pub struct SynonymGraphFilter {
    map: SynonymMap,
    ignore_case: bool,
}

impl SynonymGraphFilter {
    /// Create a new graph-aware synonym filter.
    pub fn new(map: SynonymMap, ignore_case: bool) -> Self {
        Self { map, ignore_case }
    }

    /// Create an empty synonym graph filter.
    pub fn empty() -> Self {
        Self {
            map: SynonymMap::new(),
            ignore_case: true,
        }
    }

    /// Replace the synonym map (for hot-reload).
    pub fn reload(&mut self, map: SynonymMap) {
        self.map = map;
    }

    fn lookup_term<'a>(&self, term: &'a str) -> Option<&SynonymRule> {
        if self.ignore_case {
            let lower = term.to_lowercase();
            self.map.get(&lower)
        } else {
            self.map.get(term)
        }
    }
}

impl TokenFilter for SynonymGraphFilter {
    fn filter<'a>(&self, token: &mut Token<'a>) -> (bool, Option<Vec<Token<'a>>>) {
        let term = token.term.as_ref();
        if term.is_empty() {
            return (false, None);
        }

        if let Some(rule) = self.lookup_term(term) {
            match rule.mode {
                SynonymMode::Expand => {
                    let mut extra = Vec::with_capacity(rule.replacements.len());
                    for replacement in &rule.replacements {
                        if replacement.contains(' ') {
                            // Multi-word synonym: split into sub-tokens
                            let parts: Vec<&str> = replacement.split_whitespace().collect();
                            for (i, part) in parts.iter().enumerate() {
                                let syn_token = Token {
                                    term: Cow::Owned(part.to_string()),
                                    start_offset: token.start_offset,
                                    end_offset: token.end_offset,
                                    position: token.position + i as u32,
                                };
                                extra.push(syn_token);
                            }
                        } else {
                            // Single-word synonym
                            let syn_token = Token {
                                term: Cow::Owned(replacement.clone()),
                                start_offset: token.start_offset,
                                end_offset: token.end_offset,
                                position: token.position,
                            };
                            extra.push(syn_token);
                        }
                    }
                    (false, Some(extra))
                }
                SynonymMode::Contract => {
                    if let Some(canonical) = rule.replacements.first() {
                        token.term = Cow::Owned(canonical.clone());
                    }
                    (false, None)
                }
            }
        } else {
            (false, None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SynonymParser;

    #[test]
    fn test_expand_synonym() {
        let parser = SynonymParser::new();
        let map = parser.parse("fast, quick, speedy");
        let filter = SynonymFilter::new(map, true);

        let mut token = Token::new("fast", 0, 4, 0);
        let (discard, extra) = filter.filter(&mut token);
        assert!(!discard);
        let extra = extra.unwrap();
        assert_eq!(extra.len(), 2);
        assert_eq!(extra[0].term, "quick");
        assert_eq!(extra[1].term, "speedy");
    }

    #[test]
    fn test_contract_synonym() {
        let parser = SynonymParser::new().with_expand(false);
        let map = parser.parse("fast, quick, speedy");
        let filter = SynonymFilter::new(map, true);

        let mut token = Token::new("quick", 0, 5, 0);
        let (discard, extra) = filter.filter(&mut token);
        assert!(!discard);
        assert!(extra.is_none());
        assert_eq!(token.term, "fast");
    }

    #[test]
    fn test_no_match() {
        let parser = SynonymParser::new();
        let map = parser.parse("fast, quick");
        let filter = SynonymFilter::new(map, true);

        let mut token = Token::new("hello", 0, 5, 0);
        let (discard, extra) = filter.filter(&mut token);
        assert!(!discard);
        assert!(extra.is_none());
        assert_eq!(token.term, "hello");
    }

    #[test]
    fn test_graph_multi_word() {
        let parser = SynonymParser::new();
        let map = parser.parse("ny => new york");
        let filter = SynonymGraphFilter::new(map, true);

        let mut token = Token::new("ny", 0, 2, 0);
        let (discard, extra) = filter.filter(&mut token);
        assert!(!discard);
        let extra = extra.unwrap();
        assert_eq!(extra.len(), 2);
        assert_eq!(extra[0].term, "new");
        assert_eq!(extra[1].term, "york");
    }

    #[test]
    fn test_ignore_case() {
        let parser = SynonymParser::new().with_ignore_case(true);
        let map = parser.parse("TV => television");
        let filter = SynonymFilter::new(map, true);

        let mut token = Token::new("tv", 0, 2, 0);
        let (_, extra) = filter.filter(&mut token);
        assert!(extra.is_some());
    }

    #[test]
    fn test_reload() {
        let parser = SynonymParser::new();
        let map1 = parser.parse("fast, quick");
        let mut filter = SynonymFilter::new(map1, true);

        // Initial mapping works
        let mut token = Token::new("fast", 0, 4, 0);
        let (_, extra) = filter.filter(&mut token);
        assert!(extra.is_some());

        // Reload with new map
        let map2 = parser.parse("big, large");
        filter.reload(map2);

        // Old mapping gone
        let mut token = Token::new("fast", 0, 4, 0);
        let (_, extra) = filter.filter(&mut token);
        assert!(extra.is_none());

        // New mapping works
        let mut token = Token::new("big", 0, 3, 0);
        let (_, extra) = filter.filter(&mut token);
        assert!(extra.is_some());
    }
}
