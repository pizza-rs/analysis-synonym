#![cfg_attr(not(feature = "std"), no_std)]
//! Dynamic synonym analysis plugin for Pizza search engine.
//!
//! Extends the base synonym support in `analysis-core` with:
//! - File-based synonym loading (Solr and WordNet formats)
//! - Multi-word synonym graph support
//! - Configurable expand/contract modes
//! - **Hot-reload** — atomically swap synonym maps without restarting or blocking queries
//!
//! # Hot-Reload
//!
//! ```ignore
//! let filter = SynonymFilter::new(initial_map, true);
//! let handle = filter.reload_handle();
//!
//! // From a file-watcher or config reload endpoint:
//! let new_map = SynonymParser::new().parse(&new_text);
//! handle.reload(new_map); // atomic, lock-free for readers
//! ```
//!
//! # Synonym Format (Solr)
//!
//! ```text
//! # Equivalence (bidirectional):
//! fast, quick, speedy
//!
//! # Explicit mapping (unidirectional):
//! tv => television
//! laptop => notebook, portable computer
//! ```
//!
//! # Synonym Format (WordNet)
//!
//! ```text
//! s(100000001,1,'fast',a,1,0).
//! s(100000001,2,'quick',a,1,0).
//! s(100000001,3,'speedy',a,1,0).
//! ```
extern crate alloc;
mod filter;
mod parser;

pub mod register;

pub use filter::{SynonymFilter, SynonymGraphFilter, SynonymMode, SynonymReloadHandle};
pub use parser::{SynonymFormat, SynonymMap, SynonymParser, SynonymRule};
pub use register::register_all;
