# Index Configuration

This guide explains how to configure search indexes in ParadeDB, including field types, tokenization options, and other available settings.

## Table of Contents
1. [Field Types](#field-types)
2. [Field Configuration Options](#field-configuration-options)
3. [Tokenization and Normalization](#tokenization-and-normalization)
4. [Common Configuration Examples](#common-configuration-examples)

## Field Types

ParadeDB supports the following field types for search indexes:

- Text
- Uuid
- I64 (64-bit signed integer)
- F64 (64-bit floating-point)
- U64 (64-bit unsigned integer)
- Bool
- Json
- Date
- Range

Each field type has its own configuration options to optimize indexing and searching performance.

## Field Configuration Options

### Text Fields

Text fields are used for full-text search and support the following options:

```rust
SearchFieldConfig::Text {
    indexed: bool,
    fast: bool,
    stored: bool,
    fieldnorms: bool,
    tokenizer: SearchTokenizer,
    record: IndexRecordOption,
    normalizer: SearchNormalizer,
    column: Option<String>,
}
```

- `indexed`: Enable indexing for this field (default: true)
- `fast`: Enable fast field for sorting and aggregations (default: false)
- `stored`: Store the original value in the index (default: false)
- `fieldnorms`: Enable field-length normalization (default: true)
- `tokenizer`: Specify the tokenizer to use (default: default tokenizer)
- `record`: Specify the index record option (default: WithFreqsAndPositions)
- `normalizer`: Specify the normalizer to use (default: Raw)
- `column`: Specify the source column name (optional)

### Numeric Fields

Numeric fields (I64, F64, U64) use the following configuration:

```rust
SearchFieldConfig::Numeric {
    indexed: bool,
    fast: bool,
    stored: bool,
    column: Option<String>,
}
```

- `indexed`: Enable indexing for this field (default: true)
- `fast`: Enable fast field for sorting and aggregations (default: true)
- `stored`: Store the original value in the index (default: false)
- `column`: Specify the source column name (optional)

### Boolean Fields

Boolean fields use the same configuration as numeric fields:

```rust
SearchFieldConfig::Boolean {
    indexed: bool,
    fast: bool,
    stored: bool,
    column: Option<String>,
}
```

### Json Fields

Json fields support the following options:

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

- `expand_dots`: Enable dot notation expansion for nested JSON fields (default: true)

Other options are similar to text fields.

### Date Fields

Date fields use the following configuration:

```rust
SearchFieldConfig::Date {
    indexed: bool,
    fast: bool,
    stored: bool,
    column: Option<String>,
}
```

### Range Fields

Range fields have a simplified configuration:

```rust
SearchFieldConfig::Range {
    stored: bool,
    column: Option<String>,
}
```

## Tokenization and Normalization

ParadeDB uses the `SearchTokenizer` and `SearchNormalizer` enums to configure text analysis:

### Tokenizers

- `Raw`: No tokenization (treats the entire field as a single token)
- `Default`: Standard tokenization
- `Whitespace`: Tokenize on whitespace
- `NGram`: Generate n-grams from the text
- `Chinese`: Tokenize Chinese text

### Normalizers

- `Raw`: No normalization
- `Lowercase`: Convert text to lowercase

## Common Configuration Examples

### Basic Text Field

```rust
SearchFieldConfig::Text {
    indexed: true,
    fast: false,
    stored: false,
    fieldnorms: true,
    tokenizer: SearchTokenizer::Default(SearchTokenizerFilters::default()),
    record: IndexRecordOption::WithFreqsAndPositions,
    normalizer: SearchNormalizer::Raw,
    column: None,
}
```

### Keyword Field (for exact matching)

```rust
SearchFieldConfig::Text {
    indexed: true,
    fast: true,
    stored: false,
    fieldnorms: false,
    tokenizer: SearchTokenizer::Raw(SearchTokenizerFilters::raw()),
    record: IndexRecordOption::Basic,
    normalizer: SearchNormalizer::Raw,
    column: None,
}
```

### Numeric Field for Sorting and Filtering

```rust
SearchFieldConfig::Numeric {
    indexed: true,
    fast: true,
    stored: false,
    column: None,
}
```

### JSON Field with Dot Notation Support

```rust
SearchFieldConfig::Json {
    indexed: true,
    fast: false,
    stored: true,
    fieldnorms: true,
    expand_dots: true,
    tokenizer: SearchTokenizer::Default(SearchTokenizerFilters::default()),
    record: IndexRecordOption::WithFreqsAndPositions,
    normalizer: SearchNormalizer::Raw,
    column: None,
}
```

These examples demonstrate common configurations for different use cases. Adjust the options based on your specific requirements for indexing and querying performance.