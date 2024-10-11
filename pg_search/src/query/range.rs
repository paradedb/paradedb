use tantivy::{
    query::TermQuery,
    query_grammar::Occur,
    schema::{Field, FieldType, OwnedValue},
};

const LOWER_KEY: &str = "lower";
const UPPER_KEY: &str = "upper";
const LOWER_INCLUSIVE_KEY: &str = "lower_inclusive";
const UPPER_INCLUSIVE_KEY: &str = "upper_inclusive";
const LOWER_UNBOUNDED_KEY: &str = "lower_unbounded";
const UPPER_UNBOUNDED_KEY: &str = "upper_unbounded";

