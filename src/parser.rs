//! Synonym map parser supporting Solr and WordNet formats.

use alloc::string::String;
use alloc::vec::Vec;
use hashbrown::HashMap;

/// Synonym expansion mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SynonymMode {
    /// Expand: source token emits all synonyms at the same position.
    Expand,
    /// Contract: all synonyms map to a single canonical form.
    Contract,
}

/// A single synonym rule.
#[derive(Clone, Debug)]
pub struct SynonymRule {
    /// The replacement terms.
    pub replacements: Vec<String>,
    /// Whether to expand or contract.
    pub mode: SynonymMode,
    /// Whether this is a multi-word mapping (graph-aware).
    pub is_multi_word: bool,
}

/// Supported synonym file formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SynonymFormat {
    /// Solr format: `a, b, c` (equivalence) or `a => b, c` (explicit).
    Solr,
    /// WordNet prolog format: `s(synset_id, w_num, 'word', ss_type, sense_number, tag_count).`
    WordNet,
}

/// Parsed synonym map — maps source terms to their synonym rules.
#[derive(Clone, Debug, Default)]
pub struct SynonymMap {
    /// The synonym rules indexed by source term (lowercased).
    pub rules: HashMap<String, SynonymRule>,
}

impl SynonymMap {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    /// Number of synonym source entries.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Look up a term in the synonym map.
    pub fn get(&self, term: &str) -> Option<&SynonymRule> {
        self.rules.get(term)
    }

    /// Insert a rule for a source term.
    pub fn insert(&mut self, source: String, rule: SynonymRule) {
        self.rules.insert(source, rule);
    }
}

/// Parser for synonym definition files.
#[derive(Clone, Debug)]
pub struct SynonymParser {
    /// Whether to lowercase all terms.
    pub ignore_case: bool,
    /// The expansion mode for equivalence rules.
    pub expand: bool,
    /// Format to parse.
    pub format: SynonymFormat,
}

impl Default for SynonymParser {
    fn default() -> Self {
        Self {
            ignore_case: true,
            expand: true,
            format: SynonymFormat::Solr,
        }
    }
}

impl SynonymParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_format(mut self, format: SynonymFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_ignore_case(mut self, ignore_case: bool) -> Self {
        self.ignore_case = ignore_case;
        self
    }

    pub fn with_expand(mut self, expand: bool) -> Self {
        self.expand = expand;
        self
    }

    /// Parse a synonym definition string into a `SynonymMap`.
    pub fn parse(&self, text: &str) -> SynonymMap {
        match self.format {
            SynonymFormat::Solr => self.parse_solr(text),
            SynonymFormat::WordNet => self.parse_wordnet(text),
        }
    }

    /// Parse Solr format synonym definitions.
    ///
    /// Supports:
    /// - `a, b, c` — equivalence (all map to each other if expand=true, or all→first if expand=false)
    /// - `a => b, c` — explicit mapping (a produces b and c)
    fn parse_solr(&self, text: &str) -> SynonymMap {
        let mut map = SynonymMap::new();

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((left, right)) = line.split_once("=>") {
                // Explicit mapping: source => replacement1, replacement2
                let source = self.normalize_term(left.trim());
                let replacements: Vec<String> = right
                    .split(',')
                    .map(|s| self.normalize_term(s.trim()))
                    .filter(|s| !s.is_empty())
                    .collect();

                if !source.is_empty() && !replacements.is_empty() {
                    let is_multi_word =
                        source.contains(' ') || replacements.iter().any(|r| r.contains(' '));
                    map.insert(
                        source,
                        SynonymRule {
                            replacements,
                            mode: SynonymMode::Expand,
                            is_multi_word,
                        },
                    );
                }
            } else {
                // Equivalence: a, b, c
                let terms: Vec<String> = line
                    .split(',')
                    .map(|s| self.normalize_term(s.trim()))
                    .filter(|s| !s.is_empty())
                    .collect();

                if terms.len() >= 2 {
                    if self.expand {
                        // Each term maps to all others
                        for (i, source) in terms.iter().enumerate() {
                            let replacements: Vec<String> = terms
                                .iter()
                                .enumerate()
                                .filter(|(j, _)| *j != i)
                                .map(|(_, t)| t.clone())
                                .collect();
                            let is_multi_word = terms.iter().any(|t| t.contains(' '));
                            map.insert(
                                source.clone(),
                                SynonymRule {
                                    replacements,
                                    mode: SynonymMode::Expand,
                                    is_multi_word,
                                },
                            );
                        }
                    } else {
                        // Contract: all terms map to the first
                        let canonical = terms[0].clone();
                        for source in terms.iter().skip(1) {
                            map.insert(
                                source.clone(),
                                SynonymRule {
                                    replacements: alloc::vec![canonical.clone()],
                                    mode: SynonymMode::Contract,
                                    is_multi_word: canonical.contains(' '),
                                },
                            );
                        }
                    }
                }
            }
        }

        map
    }

    /// Parse WordNet prolog format.
    ///
    /// Format: `s(synset_id, w_num, 'word', ss_type, sense_number, tag_count).`
    /// Words sharing the same synset_id are synonyms.
    fn parse_wordnet(&self, text: &str) -> SynonymMap {
        // Group words by synset_id
        let mut synsets: HashMap<String, Vec<String>> = HashMap::new();

        for line in text.lines() {
            let line = line.trim();
            if !line.starts_with("s(") || !line.ends_with(").") {
                continue;
            }

            // Parse: s(synset_id, w_num, 'word', ...)
            let inner = &line[2..line.len() - 2]; // strip s( and ).
            let parts: Vec<&str> = inner.splitn(6, ',').collect();
            if parts.len() < 3 {
                continue;
            }

            let synset_id = parts[0].trim().to_string();
            let word = parts[2].trim().trim_matches('\'');
            let word = self.normalize_term(word);

            if !word.is_empty() {
                synsets.entry(synset_id).or_insert_with(Vec::new).push(word);
            }
        }

        // Convert synsets to synonym rules
        let mut map = SynonymMap::new();
        for (_id, terms) in synsets {
            if terms.len() < 2 {
                continue;
            }
            if self.expand {
                for (i, source) in terms.iter().enumerate() {
                    let replacements: Vec<String> = terms
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| *j != i)
                        .map(|(_, t)| t.clone())
                        .collect();
                    let is_multi_word = terms.iter().any(|t| t.contains(' '));
                    map.insert(
                        source.clone(),
                        SynonymRule {
                            replacements,
                            mode: SynonymMode::Expand,
                            is_multi_word,
                        },
                    );
                }
            } else {
                let canonical = terms[0].clone();
                for source in terms.iter().skip(1) {
                    map.insert(
                        source.clone(),
                        SynonymRule {
                            replacements: alloc::vec![canonical.clone()],
                            mode: SynonymMode::Contract,
                            is_multi_word: canonical.contains(' '),
                        },
                    );
                }
            }
        }

        map
    }

    fn normalize_term(&self, term: &str) -> String {
        if self.ignore_case {
            term.to_lowercase()
        } else {
            String::from(term)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solr_equivalence() {
        let parser = SynonymParser::new();
        let map = parser.parse("fast, quick, speedy");
        assert_eq!(map.len(), 3);
        let rule = map.get("fast").unwrap();
        assert!(rule.replacements.contains(&"quick".into()));
        assert!(rule.replacements.contains(&"speedy".into()));
    }

    #[test]
    fn test_solr_explicit() {
        let parser = SynonymParser::new();
        let map = parser.parse("tv => television");
        assert_eq!(map.len(), 1);
        let rule = map.get("tv").unwrap();
        assert_eq!(rule.replacements, vec!["television"]);
    }

    #[test]
    fn test_solr_multi_word() {
        let parser = SynonymParser::new();
        let map = parser.parse("ny => new york");
        let rule = map.get("ny").unwrap();
        assert!(rule.is_multi_word);
    }

    #[test]
    fn test_solr_contract_mode() {
        let parser = SynonymParser::new().with_expand(false);
        let map = parser.parse("fast, quick, speedy");
        // quick → fast, speedy → fast
        assert!(map.get("fast").is_none()); // canonical not mapped
        let rule = map.get("quick").unwrap();
        assert_eq!(rule.replacements, vec!["fast"]);
        assert_eq!(rule.mode, SynonymMode::Contract);
    }

    #[test]
    fn test_comments_and_blanks() {
        let parser = SynonymParser::new();
        let map = parser.parse("# This is a comment\n\nfast, quick\n");
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn test_wordnet_format() {
        let parser = SynonymParser::new().with_format(SynonymFormat::WordNet);
        let input = "\
s(100000001,1,'fast',a,1,0).
s(100000001,2,'quick',a,1,0).
s(100000001,3,'speedy',a,1,0).
s(200000002,1,'big',a,1,0).
s(200000002,2,'large',a,1,0).
";
        let map = parser.parse(input);
        assert!(map.get("fast").is_some());
        assert!(map.get("big").is_some());
        let rule = map.get("fast").unwrap();
        assert!(rule.replacements.contains(&"quick".into()));
    }

    #[test]
    fn test_ignore_case() {
        let parser = SynonymParser::new().with_ignore_case(true);
        let map = parser.parse("TV => Television");
        assert!(map.get("tv").is_some());
        let rule = map.get("tv").unwrap();
        assert_eq!(rule.replacements[0], "television");
    }
}
