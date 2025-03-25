```yaml
---
title: Searching and Indexing JSON Fields
description: A guide on how to effectively search and index JSON fields in ParadeDB
---
```

# Searching and Indexing JSON Fields

ParadeDB provides powerful capabilities for searching and indexing JSON fields. This guide will walk you through the configuration options and query examples specific to JSON data.

## Configuring JSON Fields

When creating an index with JSON fields, you can specify various options to optimize search performance and functionality.

### JSON Field Configuration Options

```rust
SearchFieldConfig::Json {
    indexed: bool,
    fast: bool,
    stored: bool,
    fieldnorms: bool,
    expand_dots: bool,
    tokenizer: SearchTokenizer,
    record: IndexRecordOption,
    normalizer: SearchNormalizer,
    column: Option<String>,
}
```

- `indexed`: Whether the field should be indexed (default: true)
- `fast`: Whether to create a fast field for sorting and aggregations (default: false)
- `stored`: Whether to store the original value (default: false)
- `fieldnorms`: Whether to compute and store field norms (default: true)
- `expand_dots`: Whether to expand dot notation in JSON paths (default: true)
- `tokenizer`: The tokenizer to use for text analysis
- `record`: The type of index to create (frequencies, positions, etc.)
- `normalizer`: The normalizer to apply to the field
- `column`: The name of the source column in the PostgreSQL table

## Querying JSON Fields

ParadeDB supports various query types for JSON fields, allowing you to search for specific values, ranges, or perform more complex operations.

### Term Query

To search for an exact value in a JSON field:

```rust
SearchQueryInput::Term {
    field: Some("json_field.nested_key"),
    value: OwnedValue::Str("search_value".to_string()),
    is_datetime: false,
}
```

### Range Query

For numeric or date ranges within JSON fields:

```rust
SearchQueryInput::Range {
    field: "json_field.numeric_key",
    lower_bound: Bound::Included(OwnedValue::U64(10)),
    upper_bound: Bound::Excluded(OwnedValue::U64(20)),
    is_datetime: false,
}
```

### Exists Query

To check if a JSON field or nested key exists:

```rust
SearchQueryInput::Exists {
    field: "json_field.nested_key".to_string(),
}
```

### Fuzzy Term Query

For fuzzy matching on text values in JSON fields:

```rust
SearchQueryInput::FuzzyTerm {
    field: "json_field.text_key".to_string(),
    value: "approximte".to_string(),
    distance: Some(2),
    transposition_cost_one: Some(true),
    prefix: Some(false),
}
```

## JSON Path Support

ParadeDB supports querying nested JSON structures using dot notation. When `expand_dots` is set to true in the field configuration, you can use paths like `json_field.nested_key.deeply_nested_key` to access nested values.

## Best Practices

1. Use `fast: true` for fields that you frequently sort or aggregate on.
2. Set `expand_dots: true` to enable easy querying of nested JSON structures.
3. Choose an appropriate tokenizer and normalizer for text fields within JSON to improve search quality.
4. Use the `Exists` query to filter documents based on the presence of specific JSON keys.
5. Leverage range queries for numeric and date values stored in JSON fields.

## Example: Creating an Index with JSON Fields

```sql
CREATE INDEX idx_json_data ON your_table USING paradedb (
    json_column JSON '{"indexed": true, "fast": true, "expand_dots": true}'
);
```

This creates an index on the `json_column`, enabling fast searches and dot notation expansion for nested structures.

By leveraging these features, you can efficiently search and index JSON fields in ParadeDB, allowing for complex queries and optimized performance when working with JSON data.