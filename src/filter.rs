//! Synonym token filters with lock-free hot-reload support.
//!
//! Both [`SynonymFilter`] and [`SynonymGraphFilter`] share their synonym map
//! behind an `Arc<RwLock>`, enabling concurrent read access during query time
//! while allowing atomic map replacement for hot-reload without restart.

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::RwLock;

use pizza_engine::analysis::Token;
use pizza_engine::analysis::TokenFilter;

use crate::parser::SynonymMap;
use crate::parser::SynonymRule;

pub use crate::parser::SynonymMode;

/// Shared synonym map handle, enabling hot-reload across all cloned filter instances.
///
/// When a filter is cloned (e.g. for use across threads or in multiple analyzer
/// pipelines), all clones share the same underlying `Arc<RwLock<SynonymMap>>`.
/// Reloading via any clone or via the [`SynonymReloadHandle`] updates all of them.
type SharedMap = Arc<RwLock<SynonymMap>>;

/// A handle for hot-reloading synonym rules without needing `&mut` access to the filter.
///
/// Obtain one via [`SynonymFilter::reload_handle()`] or [`SynonymGraphFilter::reload_handle()`].
/// The handle can be stored separately (e.g. in a config watcher) and used to swap
/// the synonym map atomically while queries continue reading the old map.
///
/// # Example
///
/// ```ignore
/// let filter = SynonymFilter::new(initial_map, true);
/// let handle = filter.reload_handle();
///
/// // Later, from a config watcher thread:
/// let new_map = parser.parse(&new_synonym_text);
/// handle.reload(new_map);
/// // All queries now use the new map — no restart needed.
/// ```
#[derive(Clone, Debug)]
pub struct SynonymReloadHandle {
    inner: SharedMap,
}

impl SynonymReloadHandle {
    /// Atomically replace the synonym map. All active filters sharing this handle
    /// will see the new map on their next `filter()` call.
    pub fn reload(&self, map: SynonymMap) {
        let mut guard = self.inner.write();
        *guard = map;
    }

    /// Get the current number of synonym source entries.
    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    /// Whether the current map is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }
}

/// Single-word synonym token filter with hot-reload support.
///
/// Expands or contracts tokens based on a synonym map. Does not handle
/// multi-word synonyms — use [`SynonymGraphFilter`] for that.
///
/// # Hot-Reload
///
/// The synonym map is stored behind `Arc<RwLock<...>>`. Call [`reload()`](Self::reload)
/// or use a [`SynonymReloadHandle`] to swap the map without restarting the server.
///
/// # Expand Mode
/// Input: "fast" → Output: "fast", "quick", "speedy" (all at same position)
///
/// # Contract Mode  
/// Input: "quick" → Output: "fast" (canonical form only)
#[derive(Clone, Debug)]
pub struct SynonymFilter {
    map: SharedMap,
    ignore_case: bool,
}

impl SynonymFilter {
    /// Create a new synonym filter from a pre-built synonym map.
    pub fn new(map: SynonymMap, ignore_case: bool) -> Self {
        Self {
            map: Arc::new(RwLock::new(map)),
            ignore_case,
        }
    }

    /// Create an empty synonym filter (no-op until rules are loaded).
    pub fn empty() -> Self {
        Self {
            map: Arc::new(RwLock::new(SynonymMap::new())),
            ignore_case: true,
        }
    }

    /// Atomically replace the synonym map (hot-reload).
    ///
    /// This does NOT require `&mut self` — safe to call while queries are in flight.
    /// Active `filter()` calls reading the old map will complete normally;
    /// subsequent calls will see the new map.
    pub fn reload(&self, map: SynonymMap) {
        let mut guard = self.map.write();
        *guard = map;
    }

    /// Get a reload handle that can be stored separately for external hot-reload.
    pub fn reload_handle(&self) -> SynonymReloadHandle {
        SynonymReloadHandle {
            inner: Arc::clone(&self.map),
        }
    }

    /// Get the current number of synonym source entries.
    pub fn rule_count(&self) -> usize {
        self.map.read().len()
    }

    fn lookup_and_apply<'a>(&self, token: &mut Token<'a>) -> (bool, Option<Vec<Token<'a>>>) {
        let term = token.term.as_ref();
        if term.is_empty() {
            return (false, None);
        }

        let guard = self.map.read();

        let rule = if self.ignore_case {
            let lower = term.to_lowercase();
            guard.get(&lower)
        } else {
            guard.get(term)
        };

        match rule {
            Some(rule) => match rule.mode {
                SynonymMode::Expand => {
                    let mut extra = Vec::with_capacity(rule.replacements.len());
                    for replacement in &rule.replacements {
                        extra.push(Token {
                            term: Cow::Owned(replacement.clone()),
                            start_offset: token.start_offset,
                            end_offset: token.end_offset,
                            position: token.position,
                        });
                    }
                    (false, Some(extra))
                }
                SynonymMode::Contract => {
                    if let Some(canonical) = rule.replacements.first() {
                        token.term = Cow::Owned(canonical.clone());
                    }
                    (false, None)
                }
            },
            None => (false, None),
        }
    }
}

impl TokenFilter for SynonymFilter {
    fn filter<'a>(&self, token: &mut Token<'a>) -> (bool, Option<Vec<Token<'a>>>) {
        self.lookup_and_apply(token)
    }
}

/// Graph-aware synonym filter with hot-reload support.
///
/// Handles phrases like "new york" → "ny" or "ny" → "new york".
/// For multi-word synonyms, adjusts position to create a proper token graph.
///
/// # Hot-Reload
///
/// Same as [`SynonymFilter`] — the map is shared via `Arc<RwLock<...>>` and
/// can be swapped atomically via [`reload()`](Self::reload) or a
/// [`SynonymReloadHandle`].
#[derive(Clone, Debug)]
pub struct SynonymGraphFilter {
    map: SharedMap,
    ignore_case: bool,
}

impl SynonymGraphFilter {
    /// Create a new graph-aware synonym filter.
    pub fn new(map: SynonymMap, ignore_case: bool) -> Self {
        Self {
            map: Arc::new(RwLock::new(map)),
            ignore_case,
        }
    }

    /// Create an empty synonym graph filter.
    pub fn empty() -> Self {
        Self {
            map: Arc::new(RwLock::new(SynonymMap::new())),
            ignore_case: true,
        }
    }

    /// Atomically replace the synonym map (hot-reload).
    pub fn reload(&self, map: SynonymMap) {
        let mut guard = self.map.write();
        *guard = map;
    }

    /// Get a reload handle for external hot-reload.
    pub fn reload_handle(&self) -> SynonymReloadHandle {
        SynonymReloadHandle {
            inner: Arc::clone(&self.map),
        }
    }

    /// Get the current number of synonym source entries.
    pub fn rule_count(&self) -> usize {
        self.map.read().len()
    }

    fn lookup_and_apply<'a>(&self, token: &mut Token<'a>) -> (bool, Option<Vec<Token<'a>>>) {
        let term = token.term.as_ref();
        if term.is_empty() {
            return (false, None);
        }

        let guard = self.map.read();

        let rule = if self.ignore_case {
            let lower = term.to_lowercase();
            guard.get(&lower)
        } else {
            guard.get(term)
        };

        match rule {
            Some(rule) => match rule.mode {
                SynonymMode::Expand => {
                    let mut extra = Vec::with_capacity(rule.replacements.len());
                    for replacement in &rule.replacements {
                        if replacement.contains(' ') {
                            // Multi-word synonym: split into sub-tokens
                            let parts: Vec<&str> = replacement.split_whitespace().collect();
                            for (i, part) in parts.iter().enumerate() {
                                extra.push(Token {
                                    term: Cow::Owned(part.to_string()),
                                    start_offset: token.start_offset,
                                    end_offset: token.end_offset,
                                    position: token.position + i as u32,
                                });
                            }
                        } else {
                            extra.push(Token {
                                term: Cow::Owned(replacement.clone()),
                                start_offset: token.start_offset,
                                end_offset: token.end_offset,
                                position: token.position,
                            });
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
            },
            None => (false, None),
        }
    }
}

impl TokenFilter for SynonymGraphFilter {
    fn filter<'a>(&self, token: &mut Token<'a>) -> (bool, Option<Vec<Token<'a>>>) {
        self.lookup_and_apply(token)
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
