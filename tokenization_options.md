# Tokenization Options in ParadeDB

ParadeDB offers a variety of tokenization options to suit different types of text data and search requirements. This document outlines the available tokenizers, their use cases, and how to choose the right one for your needs.

## Available Tokenizers

ParadeDB supports the following tokenizers:

1. Default
2. Raw
3. Lowercase
4. Whitespace
5. Regex
6. Chinese Compatible
7. Source Code
8. Ngram
9. Chinese Lindera
10. Japanese Lindera
11. Korean Lindera
12. ICU (if the feature is enabled)

### Default Tokenizer

The default tokenizer uses a simple tokenization strategy and is suitable for general text in most Western languages.

Example configuration:
```json
{
  "type": "default",
  "remove_long": 255,
  "lowercase": true
}
```

### Raw Tokenizer

The raw tokenizer does minimal processing and is useful when you want to preserve the original text as-is.

Example configuration:
```json
{
  "type": "raw",
  "remove_long": 1000000
}
```

### Lowercase Tokenizer

This tokenizer converts all text to lowercase, which can be useful for case-insensitive searches.

Example configuration:
```json
{
  "type": "lowercase",
  "remove_long": 255
}
```

### Whitespace Tokenizer

The whitespace tokenizer splits text on whitespace characters, which is useful for simple word-based tokenization.

Example configuration:
```json
{
  "type": "whitespace",
  "remove_long": 255,
  "lowercase": true
}
```

### Regex Tokenizer

The regex tokenizer allows you to define custom tokenization rules using regular expressions.

Example configuration:
```json
{
  "type": "regex",
  "pattern": "\\w+",
  "remove_long": 255,
  "lowercase": true
}
```

### Chinese Compatible Tokenizer

This tokenizer is optimized for Chinese text, providing better word segmentation for Chinese characters.

Example configuration:
```json
{
  "type": "chinese_compatible",
  "remove_long": 255,
  "lowercase": true
}
```

### Source Code Tokenizer

The source code tokenizer is designed to handle programming language syntax, making it ideal for indexing and searching code snippets.

Example configuration:
```json
{
  "type": "source_code",
  "remove_long": 255,
  "lowercase": true
}
```

### Ngram Tokenizer

The Ngram tokenizer creates tokens of specified lengths, which can be useful for partial matching and auto-complete features.

Example configuration:
```json
{
  "type": "ngram",
  "min_gram": 2,
  "max_gram": 3,
  "prefix_only": false,
  "remove_long": 255,
  "lowercase": true
}
```

### Language-specific Lindera Tokenizers

ParadeDB provides specialized tokenizers for Chinese, Japanese, and Korean languages using the Lindera library:

- Chinese Lindera: `"type": "chinese_lindera"`
- Japanese Lindera: `"type": "japanese_lindera"`
- Korean Lindera: `"type": "korean_lindera"`

These tokenizers are optimized for their respective languages and provide better word segmentation compared to general-purpose tokenizers.

### ICU Tokenizer

If the ICU feature is enabled, ParadeDB provides an ICU-based tokenizer that offers advanced Unicode text segmentation capabilities.

Example configuration:
```json
{
  "type": "icu",
  "remove_long": 255,
  "lowercase": true
}
```

## Choosing the Right Tokenizer

When selecting a tokenizer, consider the following factors:

1. Language: Use language-specific tokenizers for better results with Chinese, Japanese, or Korean text.
2. Text type: Choose specialized tokenizers like the source code tokenizer for specific types of content.
3. Search requirements: Consider using Ngram tokenizers for partial matching or auto-complete features.
4. Case sensitivity: Use the lowercase option if you need case-insensitive searches.
5. Token length: Adjust the `remove_long` parameter to control the maximum token length.

## Common Configuration Options

Most tokenizers support the following configuration options:

- `remove_long`: Maximum token length (default: 255)
- `lowercase`: Convert tokens to lowercase (default: true)
- `stemmer`: Apply stemming to tokens (language-specific)

## Conclusion

ParadeDB's diverse set of tokenization options allows you to fine-tune your search index for optimal performance across various languages and text types. Experiment with different tokenizers and configurations to find the best fit for your specific use case.