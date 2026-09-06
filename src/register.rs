//! Register synonym analysis components into [`AnalysisFactory`].

use alloc::boxed::Box;

use pizza_engine::analysis::AnalysisFactory;

use crate::filter::SynonymFilter;
use crate::filter::SynonymGraphFilter;

/// Register synonym token filters into the factory.
///
/// Registers:
/// - `"synonym"` — single-word synonym expansion/contraction filter
/// - `"synonym_graph"` — graph-aware multi-word synonym filter
///
/// Both filters start empty (no rules). Rules are loaded at runtime via
/// configuration or the `reload()` method on the filter instance.
pub fn register_all(factory: &mut AnalysisFactory) {
    factory.register_token_filter("synonym", Box::new(SynonymFilter::empty()));
    factory.register_token_filter("synonym_graph", Box::new(SynonymGraphFilter::empty()));
}
