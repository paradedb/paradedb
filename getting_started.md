---
title: Getting Started with pg_search
---

# Getting Started with pg_search

Welcome to pg_search, a powerful extension for PostgreSQL that brings advanced search capabilities to your database. This guide will walk you through the installation process, basic usage, and how to create and use BM25 indexes.

## Installation

To install pg_search, follow these steps:

1. Ensure you have PostgreSQL installed on your system.

2. Add pg_search to your `postgresql.conf` file:

   ```
   shared_preload_libraries = 'pg_search'
   ```

3. Restart your PostgreSQL server to load the new shared library.

4. Connect to your database and create the extension:

   ```sql
   CREATE EXTENSION pg_search;
   ```

## Basic Usage

Once installed, pg_search provides several functions and a new index access method. Here's how to get started:

### Creating a BM25 Index

To create a BM25 index on a text column:

```sql
CREATE INDEX idx_name ON your_table USING bm25 (your_text_column);
```

This creates an index using the BM25 algorithm, which is excellent for full-text search.

### Performing a Search

To search using the BM25 index, you can use the `@@@` operator:

```sql
SELECT * FROM your_table WHERE your_text_column @@@ 'search query';
```

This will return rows where the text column matches the search query, ranked by relevance.

## Advanced Features

### Random Word Generation

pg_search includes a utility function for generating random words, which can be useful for testing:

```sql
SELECT random_words(5);
```

This will generate a string of 5 random words.

### Custom Search Queries

For more complex searches, you can use the `SearchQueryInput` type:

```sql
SELECT * FROM your_table WHERE your_text_column @@@ '{"query": "advanced search", "fields": ["title", "body"]}';
```

This allows you to specify which fields to search and other parameters.

## Configuration

pg_search can be configured using GUCs (Grand Unified Configurations). These are set in your `postgresql.conf` file or at runtime. For example:

```sql
SET pg_search.some_option = 'value';
```

Refer to the full documentation for a list of available configuration options.

## Next Steps

- Explore the API documentation for more advanced usage.
- Check out the query syntax for complex search operations.
- Learn about index management and optimization techniques.

For more detailed information, please refer to the full documentation.