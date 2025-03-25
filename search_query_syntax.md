# Search Query Syntax

This guide provides a comprehensive overview of the search query syntax supported by pg_search. It covers various query types and demonstrates how to use them effectively in your searches.

## Table of Contents

1. [Introduction](#introduction)
2. [Basic Query Types](#basic-query-types)
   - [Term Query](#term-query)
   - [Phrase Query](#phrase-query)
   - [Wildcard Query](#wildcard-query)
3. [Advanced Query Types](#advanced-query-types)
   - [Boolean Query](#boolean-query)
   - [Range Query](#range-query)
   - [Fuzzy Query](#fuzzy-query)
   - [Regex Query](#regex-query)
4. [Special Query Types](#special-query-types)
   - [More Like This Query](#more-like-this-query)
   - [Exists Query](#exists-query)
5. [Query Modifiers](#query-modifiers)
   - [Boosting](#boosting)
   - [Constant Score](#constant-score)

## Introduction

pg_search provides a powerful and flexible query syntax that allows you to perform complex searches on your indexed data. This document will guide you through the various query types and their usage.

## Basic Query Types

### Term Query

A term query searches for an exact match of a term in a specific field.

```json
{
  "term": {
    "field": "title",
    "value": "postgresql"
  }
}
```

### Phrase Query

A phrase query searches for a sequence of terms in a specific order.

```json
{
  "phrase": {
    "field": "content",
    "phrases": ["quick brown fox"],
    "slop": 0
  }
}
```

The `slop` parameter allows for a specified number of intervening unmatched positions between phrase terms.

### Wildcard Query

Wildcard queries use `*` to match any number of characters and `?` to match a single character.

```json
{
  "term": {
    "field": "title",
    "value": "post*"
  }
}
```

## Advanced Query Types

### Boolean Query

Boolean queries combine multiple sub-queries using boolean logic.

```json
{
  "boolean": {
    "must": [
      { "term": { "field": "category", "value": "database" } }
    ],
    "should": [
      { "term": { "field": "tags", "value": "performance" } },
      { "term": { "field": "tags", "value": "indexing" } }
    ],
    "must_not": [
      { "term": { "field": "status", "value": "archived" } }
    ]
  }
}
```

### Range Query

Range queries find documents with field values within a specified range.

```json
{
  "range": {
    "field": "price",
    "lower_bound": { "included": 10 },
    "upper_bound": { "excluded": 100 }
  }
}
```

### Fuzzy Query

Fuzzy queries allow for approximate matching, accounting for typos or slight variations.

```json
{
  "fuzzy_term": {
    "field": "title",
    "value": "postgresql",
    "distance": 2
  }
}
```

### Regex Query

Regex queries use regular expressions for matching.

```json
{
  "regex": {
    "field": "email",
    "pattern": ".*@example\\.com"
  }
}
```

## Special Query Types

### More Like This Query

The More Like This (MLT) query finds documents similar to a given input.

```json
{
  "more_like_this": {
    "document_id": 123,
    "min_term_frequency": 2,
    "max_query_terms": 25
  }
}
```

### Exists Query

The Exists query finds documents where a specific field exists and is not null.

```json
{
  "exists": {
    "field": "tags"
  }
}
```

## Query Modifiers

### Boosting

Boosting increases the relevance score of a query.

```json
{
  "boost": {
    "query": {
      "term": { "field": "title", "value": "important" }
    },
    "factor": 2.0
  }
}
```

### Constant Score

Constant Score assigns a fixed score to all documents matching a query.

```json
{
  "const_score": {
    "query": {
      "term": { "field": "category", "value": "featured" }
    },
    "score": 1.5
  }
}
```

This guide covers the main query types and modifiers supported by pg_search. For more detailed information on each query type and additional options, please refer to the API documentation.