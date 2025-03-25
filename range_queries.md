# Range Queries

Range queries allow you to search for documents where a field's value falls within a specified range. ParadeDB supports range queries on numeric, date, and other range-supporting fields. This guide explains how to perform range queries and provides best practices for efficient range searching.

## Supported Field Types

Range queries can be performed on the following field types:

- Numeric fields (integers and floating-point numbers)
- Date fields
- Range fields (e.g., `int4range`, `int8range`, `numrange`, `daterange`, `tsrange`, `tstzrange`)

## Query Syntax

To perform a range query, you can use the `Range`, `RangeContains`, `RangeIntersects`, or `RangeWithin` query types in the `SearchQueryInput` enum. The general structure of a range query is as follows:

```rust
SearchQueryInput::Range {
    field: String,
    lower_bound: std::ops::Bound<tantivy::schema::OwnedValue>,
    upper_bound: std::ops::Bound<tantivy::schema::OwnedValue>,
    is_datetime: bool,
}
```

- `field`: The name of the field you want to search.
- `lower_bound`: The lower bound of the range.
- `upper_bound`: The upper bound of the range.
- `is_datetime`: Set to `true` if the field is a date/time field.

The `Bound` enum can be one of:
- `Bound::Included(value)`: The value is included in the range.
- `Bound::Excluded(value)`: The value is excluded from the range.
- `Bound::Unbounded`: No bound (used for open-ended ranges).

## Examples

### Numeric Range Query

To search for documents where a numeric field `price` is between 10 and 50 (inclusive):

```rust
SearchQueryInput::Range {
    field: "price".to_string(),
    lower_bound: Bound::Included(OwnedValue::U64(10)),
    upper_bound: Bound::Included(OwnedValue::U64(50)),
    is_datetime: false,
}
```

### Date Range Query

To search for documents where a date field `created_at` is between two dates:

```rust
SearchQueryInput::Range {
    field: "created_at".to_string(),
    lower_bound: Bound::Included(OwnedValue::Date(DateTime::from_timestamp_micros(...))),
    upper_bound: Bound::Excluded(OwnedValue::Date(DateTime::from_timestamp_micros(...))),
    is_datetime: true,
}
```

### Open-ended Range Query

To search for documents where a numeric field `age` is greater than or equal to 18:

```rust
SearchQueryInput::Range {
    field: "age".to_string(),
    lower_bound: Bound::Included(OwnedValue::U64(18)),
    upper_bound: Bound::Unbounded,
    is_datetime: false,
}
```

## Range Types

ParadeDB supports different types of range queries:

1. `Range`: Standard range query.
2. `RangeContains`: Finds ranges that contain the specified range.
3. `RangeIntersects`: Finds ranges that intersect with the specified range.
4. `RangeWithin`: Finds ranges that are entirely within the specified range.

These query types have the same structure as the `Range` query, but they behave differently in how they match documents.

## Best Practices

1. **Use appropriate field types**: Ensure that fields you want to perform range queries on are indexed with the correct type (e.g., `Numeric`, `Date`, or `Range`).

2. **Optimize for performance**: When possible, use `fast: true` in your field configuration to enable fast fields for better query performance.

3. **Be mindful of inclusive vs. exclusive bounds**: Choose between `Bound::Included` and `Bound::Excluded` carefully to match your exact requirements.

4. **Use open-ended ranges when appropriate**: For queries like "greater than" or "less than", use `Bound::Unbounded` for the open end of the range.

5. **Consider using `RangeContains`, `RangeIntersects`, or `RangeWithin** for more complex range comparisons when dealing with range fields.

6. **Date precision**: When working with date/time fields, be aware of the precision of your data and queries. ParadeDB uses microsecond precision for date/time values.

By following these guidelines and understanding the different range query options, you can effectively search and filter your data based on ranges in ParadeDB.