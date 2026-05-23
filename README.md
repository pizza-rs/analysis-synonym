<div align="center">

# 🔀 pizza-analysis-synonym

**Synonym expansion filters for [INFINI Pizza](https://pizza.rs)**

[![Crate](https://img.shields.io/badge/crate-pizza--analysis--synonym-blue)](https://github.com/pizza-rs/analysis-synonym)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

</div>

---

## Overview

Provides synonym expansion and contraction filters with support for both
single-word and multi-word (graph-aware) synonym handling. Supports Solr/Elasticsearch
synonym file format with explicit mappings and equivalent groups.

## Components

| Type | Name | Description |
|:-----|:-----|:------------|
| TokenFilter | `synonym` | Single-word synonym expansion/contraction |
| TokenFilter | `synonym_graph` | Graph-aware multi-word synonym filter |

### Synonym Formats

**Explicit mapping** (one-directional):
```
ipod, i-pod, i pod => ipod
sea biscuit, seabiscuit => seabiscuit
```

**Equivalent groups** (bidirectional):
```
couch, sofa, divan
notebook, laptop, netbook
```

### synonym vs synonym_graph

| Feature | `synonym` | `synonym_graph` |
|:--------|:----------|:----------------|
| Multi-word input | Flattened | Proper graph with posLength |
| Index-time safe | ✅ | ❌ (use at query-time only) |
| Position accuracy | Approximate | Exact graph arcs |

Use `synonym` for index-time expansion and `synonym_graph` for query-time with
multi-word synonyms.

## Example

```rust
use pizza_analysis_synonym::{SynonymFilter, SynonymMap, SynonymRule};

let rules = vec![
    SynonymRule::equivalent(vec!["quick", "fast", "speedy"]),
    SynonymRule::explicit(vec!["ny", "new york"], "new york"),
];
let map = SynonymMap::from_rules(&rules);
let filter = SynonymFilter::new(map);
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

## License

MIT

---

<div align="center">
<sub>Part of the <a href="https://pizza.rs">INFINI Pizza</a> ecosystem</sub>
</div>
