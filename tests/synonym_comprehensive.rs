//! Comprehensive tests for pizza-analysis-synonym (synonym expansion/contraction).

use pizza_analysis_synonym::SynonymFilter;
use pizza_analysis_synonym::SynonymFormat;
use pizza_analysis_synonym::SynonymGraphFilter;
use pizza_analysis_synonym::SynonymMap;
use pizza_analysis_synonym::SynonymMode;
use pizza_analysis_synonym::SynonymParser;
use pizza_analysis_synonym::SynonymRule;
use pizza_engine::analysis::AnalysisFactory;
use pizza_engine::analysis::Token;
use pizza_engine::analysis::TokenFilter;

// ═══════════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════════

fn make_token(term: &str) -> Token<'_> {
    Token::new(term, 0, term.len() as u32, 0)
}

fn terms_from_extra(extra: &Option<Vec<Token>>) -> Vec<String> {
    extra
        .as_ref()
        .map(|v| v.iter().map(|t| t.term.to_string()).collect())
        .unwrap_or_default()
}

// ═══════════════════════════════════════════════════════════════════════════════
// SynonymParser — construction
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn parser_construction() {
    let _p = SynonymParser::new();
}

#[test]
fn parser_default() {
    let p = SynonymParser::default();
    assert!(p.ignore_case);
    assert!(p.expand);
    assert_eq!(p.format, SynonymFormat::Solr);
}

#[test]
fn parser_builder_methods() {
    let p = SynonymParser::new()
        .with_format(SynonymFormat::WordNet)
        .with_ignore_case(false)
        .with_expand(false);
    assert_eq!(p.format, SynonymFormat::WordNet);
    assert!(!p.ignore_case);
    assert!(!p.expand);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SynonymParser — Solr format parsing
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn parse_solr_equivalence_expand() {
    let p = SynonymParser::new().with_expand(true);
    let map = p.parse("fast, quick, speedy");
    assert!(!map.is_empty());
    assert!(map.get("fast").is_some());
    assert!(map.get("quick").is_some());
    assert!(map.get("speedy").is_some());
}

#[test]
fn parse_solr_equivalence_contract() {
    let p = SynonymParser::new().with_expand(false);
    let map = p.parse("fast, quick, speedy");
    // Contract: "quick" and "speedy" map to "fast"
    assert!(map.get("quick").is_some());
    assert!(map.get("speedy").is_some());
    // The canonical term ("fast") should NOT be in the map as a source
    assert!(map.get("fast").is_none());
}

#[test]
fn parse_solr_explicit_mapping() {
    let p = SynonymParser::new();
    let map = p.parse("tv => television");
    assert!(map.get("tv").is_some());
    let rule = map.get("tv").unwrap();
    assert!(rule.replacements.contains(&"television".to_string()));
}

#[test]
fn parse_solr_explicit_multi_target() {
    let p = SynonymParser::new();
    let map = p.parse("laptop => notebook, portable computer");
    let rule = map.get("laptop").unwrap();
    assert!(rule.replacements.len() >= 2);
}

#[test]
fn parse_solr_comments_ignored() {
    let p = SynonymParser::new();
    let map = p.parse("# This is a comment\nfast, quick\n# another comment");
    assert!(!map.is_empty());
    assert!(map.get("fast").is_some());
}

#[test]
fn parse_solr_empty_lines() {
    let p = SynonymParser::new();
    let map = p.parse("\n\nfast, quick\n\n");
    assert!(!map.is_empty());
}

#[test]
fn parse_solr_empty_input() {
    let p = SynonymParser::new();
    let map = p.parse("");
    assert!(map.is_empty());
}

#[test]
fn parse_solr_single_word_line() {
    let p = SynonymParser::new();
    let map = p.parse("lonely");
    // A single word with no synonyms should not produce entries
    assert!(map.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════════
// SynonymMap — API
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn synonym_map_new_is_empty() {
    let map = SynonymMap::new();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);
}

#[test]
fn synonym_map_insert_and_get() {
    let mut map = SynonymMap::new();
    map.insert(
        "hello".to_string(),
        SynonymRule {
            replacements: vec!["hi".to_string(), "hey".to_string()],
            mode: SynonymMode::Expand,
            is_multi_word: false,
        },
    );
    assert_eq!(map.len(), 1);
    assert!(!map.is_empty());
    assert!(map.get("hello").is_some());
    assert!(map.get("world").is_none());
}

// ═══════════════════════════════════════════════════════════════════════════════
// SynonymFilter — construction
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn filter_empty_construction() {
    let f = SynonymFilter::empty();
    let mut token = make_token("hello");
    let (deleted, extra) = f.filter(&mut token);
    assert!(!deleted);
    assert!(extra.is_none());
}

#[test]
fn filter_with_map() {
    let p = SynonymParser::new();
    let map = p.parse("fast, quick");
    let _f = SynonymFilter::new(map, true);
}

// ═══════════════════════════════════════════════════════════════════════════════
// SynonymFilter — expand mode
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn filter_expand_produces_extra_tokens() {
    let p = SynonymParser::new().with_expand(true);
    let map = p.parse("fast, quick, speedy");
    let f = SynonymFilter::new(map, true);

    let mut token = make_token("fast");
    let (deleted, extra) = f.filter(&mut token);
    assert!(!deleted);
    let extras = terms_from_extra(&extra);
    assert!(extras.contains(&"quick".to_string()));
    assert!(extras.contains(&"speedy".to_string()));
}

#[test]
fn filter_expand_case_insensitive() {
    let p = SynonymParser::new().with_expand(true);
    let map = p.parse("fast, quick");
    let f = SynonymFilter::new(map, true);

    let mut token = make_token("FAST");
    let (deleted, extra) = f.filter(&mut token);
    assert!(!deleted);
    assert!(extra.is_some());
}

#[test]
fn filter_no_match_passes_through() {
    let p = SynonymParser::new();
    let map = p.parse("fast, quick");
    let f = SynonymFilter::new(map, true);

    let mut token = make_token("slow");
    let (deleted, extra) = f.filter(&mut token);
    assert!(!deleted);
    assert!(extra.is_none());
    assert_eq!(token.term.as_ref(), "slow");
}

// ═══════════════════════════════════════════════════════════════════════════════
// SynonymFilter — contract mode
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn filter_contract_replaces_with_canonical() {
    let p = SynonymParser::new().with_expand(false);
    let map = p.parse("fast, quick, speedy");
    let f = SynonymFilter::new(map, true);

    let mut token = make_token("quick");
    let (deleted, _extra) = f.filter(&mut token);
    assert!(!deleted);
    assert_eq!(token.term.as_ref(), "fast");
}

// ═══════════════════════════════════════════════════════════════════════════════
// SynonymFilter — edge cases
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn filter_empty_token() {
    let f = SynonymFilter::empty();
    let mut token = make_token("");
    let (deleted, extra) = f.filter(&mut token);
    assert!(!deleted);
    assert!(extra.is_none());
}

#[test]
fn filter_reload() {
    let f = SynonymFilter::empty();
    let p = SynonymParser::new();
    let map = p.parse("fast, quick");
    f.reload(map); // hot-reload: no &mut needed!

    let mut token = make_token("fast");
    let (_, extra) = f.filter(&mut token);
    assert!(extra.is_some());
}

#[test]
fn filter_reload_via_handle() {
    let f = SynonymFilter::empty();
    let handle = f.reload_handle();

    // Initially empty — no synonyms
    let mut token = make_token("fast");
    let (_, extra) = f.filter(&mut token);
    assert!(extra.is_none());

    // Hot-reload via external handle
    let map = SynonymParser::new().parse("fast, quick, speedy");
    handle.reload(map);

    // Now synonyms are active
    let mut token = make_token("fast");
    let (_, extra) = f.filter(&mut token);
    assert!(extra.is_some());
    assert_eq!(extra.unwrap().len(), 2); // quick + speedy
}

// ═══════════════════════════════════════════════════════════════════════════════
// SynonymGraphFilter — basic
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn graph_filter_construction() {
    let _f = SynonymGraphFilter::empty();
}

#[test]
fn graph_filter_single_word_synonym() {
    let p = SynonymParser::new();
    let map = p.parse("fast, quick");
    let f = SynonymGraphFilter::new(map, true);

    let mut token = make_token("fast");
    let (deleted, extra) = f.filter(&mut token);
    assert!(!deleted);
    assert!(extra.is_some());
}

#[test]
fn graph_filter_multi_word_synonym() {
    let p = SynonymParser::new();
    let map = p.parse("ny => new york");
    let f = SynonymGraphFilter::new(map, true);

    let mut token = make_token("ny");
    let (deleted, extra) = f.filter(&mut token);
    assert!(!deleted);
    // Multi-word synonym should produce multiple sub-tokens
    let extras = terms_from_extra(&extra);
    assert!(extras.len() >= 2);
    assert!(extras.contains(&"new".to_string()));
    assert!(extras.contains(&"york".to_string()));
}

#[test]
fn graph_filter_empty_token() {
    let f = SynonymGraphFilter::empty();
    let mut token = make_token("");
    let (deleted, extra) = f.filter(&mut token);
    assert!(!deleted);
    assert!(extra.is_none());
}

#[test]
fn graph_filter_reload() {
    let f = SynonymGraphFilter::empty();
    let p = SynonymParser::new();
    let map = p.parse("big, large");
    f.reload(map); // hot-reload: no &mut needed!

    let mut token = make_token("big");
    let (_, extra) = f.filter(&mut token);
    assert!(extra.is_some());
}

#[test]
fn graph_filter_reload_via_handle() {
    let f = SynonymGraphFilter::empty();
    let handle = f.reload_handle();

    let map = SynonymParser::new().parse("ny => new york");
    handle.reload(map);

    let mut token = make_token("ny");
    let (_, extra) = f.filter(&mut token);
    assert!(extra.is_some());
    let tokens = extra.unwrap();
    // "new york" → 2 tokens: "new" at pos 0, "york" at pos 1
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].term.as_ref(), "new");
    assert_eq!(tokens[1].term.as_ref(), "york");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Registration
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn register_all_does_not_panic() {
    let mut factory = AnalysisFactory::new();
    pizza_analysis_synonym::register_all(&mut factory);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Pipeline integration
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn pipeline_multiple_synonyms_in_sequence() {
    let p = SynonymParser::new().with_expand(true);
    let rules = "fast, quick\nbig, large, huge";
    let map = p.parse(rules);
    let f = SynonymFilter::new(map, true);

    let input_terms = ["fast", "big", "dog"];
    let mut all_terms: Vec<String> = Vec::new();

    for &term in &input_terms {
        let mut token = make_token(term);
        let (_, extra) = f.filter(&mut token);
        all_terms.push(token.term.to_string());
        if let Some(extras) = extra {
            for e in extras {
                all_terms.push(e.term.to_string());
            }
        }
    }

    assert!(all_terms.contains(&"quick".to_string()));
    assert!(all_terms.contains(&"large".to_string()));
    assert!(all_terms.contains(&"dog".to_string()));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Unicode handling
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn synonym_with_unicode_terms() {
    let p = SynonymParser::new();
    let map = p.parse("café, coffee shop");
    let f = SynonymFilter::new(map, true);

    let mut token = make_token("café");
    let (deleted, extra) = f.filter(&mut token);
    assert!(!deleted);
    assert!(extra.is_some());
}

#[test]
fn synonym_with_cjk_terms() {
    let p = SynonymParser::new();
    let map = p.parse("电脑, 计算机");
    let f = SynonymFilter::new(map, true);

    let mut token = make_token("电脑");
    let (deleted, extra) = f.filter(&mut token);
    assert!(!deleted);
    assert!(extra.is_some());
}
