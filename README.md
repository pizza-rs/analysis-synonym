# pizza-analysis-synonym

Synonym expansion and graph-based synonym support for the Pizza search engine.

Part of the [Pizza](https://pizza.rs) search engine.

## Components

| Name | Type | Description |
|------|------|-------------|
| `synonym` | Token Filter | Expands or replaces tokens using a synonym map |
| `synonym_graph` | Token Filter | Graph-aware synonym filter preserving multi-word synonym positions |

## Usage

### Custom Pipeline

```json
{
  "analyzer": {
    "type": "custom",
    "tokenizer": "standard",
    "filter": ["synonym", "synonym_graph"]
  }
}
```

## License

MIT — see [LICENSE](LICENSE).

## Related Crates

- [analysis-core](https://github.com/pizza-rs/analysis-core) — Core analysis components and pipeline
- [analysis-icu](https://github.com/pizza-rs/analysis-icu) — ICU Unicode normalization and tokenization
