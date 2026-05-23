<div align="center">

# 🔄 pizza-analysis-synonym

**Synonym expansion and graph-based synonym plugin for [INFINI Pizza](https://pizza.rs)**

[![Crate](https://img.shields.io/badge/crate-pizza--analysis--synonym-blue)](https://github.com/pizza-rs/analysis-synonym)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

</div>

---

## Overview

`pizza-analysis-synonym` provides synonym expansion capabilities for the [INFINI Pizza](https://pizza.rs) search engine:

- **Synonym Filter** — Single-word synonym expansion and contraction
- **Synonym Graph Filter** — Multi-word, graph-aware synonym handling that preserves phrase query correctness
- **File-based Rules** — Load synonym rules from external files
- **Hot Reload** — Dynamically update synonym rules without restart

## Components

| Type | Name | Description |
|:-----|:-----|:------------|
| Filter | `synonym` | Single-word synonym expansion/contraction |
| Filter | `synonym_graph` | Graph-aware multi-word synonym filter |

## Synonym Rule Formats

```text
# Equivalent synonyms (bidirectional)
fast, quick, speedy

# Explicit mapping (unidirectional)
laptop => computer
cellphone, mobile => phone
```

## Installation

```toml
[dependencies]
pizza-analysis-synonym = "0.1"
```

Or via `pizza-analysis-all`:

```toml
[dependencies]
pizza-analysis-all = { version = "0.1", features = ["synonym"] }
```

## Usage

```rust
use pizza_engine::analysis::AnalysisFactory;

let mut factory = AnalysisFactory::new();
pizza_analysis_synonym::register_all(&mut factory);
```

## License

MIT

---

<div align="center">
<sub>Part of the <a href="https://pizza.rs">INFINI Pizza</a> ecosystem</sub>
</div>
