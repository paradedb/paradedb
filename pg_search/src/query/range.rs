use crate::query::value_to_json_term;
use crate::schema::IndexRecordOption;
use anyhow::Result;
use std::ops::Bound;
use tantivy::{
    query::{RangeQuery, TermQuery},
    schema::{Field, OwnedValue},
    Term,
};

const EMPTY_KEY: &str = "empty";
const LOWER_KEY: &str = "lower";
const UPPER_KEY: &str = "upper";
const LOWER_INCLUSIVE_KEY: &str = "lower_inclusive";
const UPPER_INCLUSIVE_KEY: &str = "upper_inclusive";
const LOWER_UNBOUNDED_KEY: &str = "lower_unbounded";
const UPPER_UNBOUNDED_KEY: &str = "upper_unbounded";
// Always false for range fields
const EXPAND_DOTS: bool = false;

#[derive(Clone, Debug)]
pub struct RangeField {
    field: Field,
    is_datetime: bool,
}

#[derive(Debug, PartialEq)]
pub enum Comparison {
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

impl RangeField {
    pub fn new(field: Field, is_datetime: bool) -> Self {
        Self { field, is_datetime }
    }

    pub fn empty(&self, val: bool) -> Result<TermQuery, Box<dyn std::error::Error>> {
        Ok(TermQuery::new(
            Self::as_range_term(self, &OwnedValue::Bool(val), Some(EMPTY_KEY))?,
            IndexRecordOption::WithFreqsAndPositions.into(),
        ))
    }

    pub fn upper_bound_inclusive(
        &self,
        val: bool,
    ) -> Result<TermQuery, Box<dyn std::error::Error>> {
        Ok(TermQuery::new(
            Self::as_range_term(self, &OwnedValue::Bool(val), Some(UPPER_INCLUSIVE_KEY))?,
            IndexRecordOption::WithFreqsAndPositions.into(),
        ))
    }

    pub fn lower_bound_inclusive(
        &self,
        val: bool,
    ) -> Result<TermQuery, Box<dyn std::error::Error>> {
        Ok(TermQuery::new(
            Self::as_range_term(self, &OwnedValue::Bool(val), Some(LOWER_INCLUSIVE_KEY))?,
            IndexRecordOption::WithFreqsAndPositions.into(),
        ))
    }

    pub fn upper_bound_unbounded(
        &self,
        val: bool,
    ) -> Result<TermQuery, Box<dyn std::error::Error>> {
        Ok(TermQuery::new(
            Self::as_range_term(self, &OwnedValue::Bool(val), Some(UPPER_UNBOUNDED_KEY))?,
            IndexRecordOption::WithFreqsAndPositions.into(),
        ))
    }

    pub fn lower_bound_unbounded(
        &self,
        val: bool,
    ) -> Result<TermQuery, Box<dyn std::error::Error>> {
        Ok(TermQuery::new(
            Self::as_range_term(self, &OwnedValue::Bool(val), Some(LOWER_UNBOUNDED_KEY))?,
            IndexRecordOption::WithFreqsAndPositions.into(),
        ))
    }

    pub fn compare_lower_bound(
        &self,
        owned: &OwnedValue,
        comparison: Comparison,
    ) -> Result<RangeQuery, Box<dyn std::error::Error>> {
        let query = match comparison {
            Comparison::LessThan => RangeQuery::new(
                Bound::Excluded(Self::as_range_term(self, owned, Some(LOWER_KEY))?),
                Bound::Unbounded,
            ),
            Comparison::LessThanOrEqual => RangeQuery::new(
                Bound::Included(Self::as_range_term(self, owned, Some(LOWER_KEY))?),
                Bound::Unbounded,
            ),
            Comparison::GreaterThan => RangeQuery::new(
                Bound::Unbounded,
                Bound::Excluded(Self::as_range_term(self, owned, Some(LOWER_KEY))?),
            ),
            Comparison::GreaterThanOrEqual => RangeQuery::new(
                Bound::Unbounded,
                Bound::Included(Self::as_range_term(self, owned, Some(LOWER_KEY))?),
            ),
        };

        Ok(query)
    }

    pub fn compare_upper_bound(
        &self,
        owned: &OwnedValue,
        comparison: Comparison,
    ) -> Result<RangeQuery, Box<dyn std::error::Error>> {
        let query = match comparison {
            Comparison::LessThan => RangeQuery::new(
                Bound::Excluded(Self::as_range_term(self, owned, Some(UPPER_KEY))?),
                Bound::Unbounded,
            ),
            Comparison::LessThanOrEqual => RangeQuery::new(
                Bound::Included(Self::as_range_term(self, owned, Some(UPPER_KEY))?),
                Bound::Unbounded,
            ),
            Comparison::GreaterThan => RangeQuery::new(
                Bound::Unbounded,
                Bound::Excluded(Self::as_range_term(self, owned, Some(UPPER_KEY))?),
            ),
            Comparison::GreaterThanOrEqual => RangeQuery::new(
                Bound::Unbounded,
                Bound::Included(Self::as_range_term(self, owned, Some(UPPER_KEY))?),
            ),
        };

        Ok(query)
    }

    fn as_range_term(
        &self,
        value: &OwnedValue,
        path: Option<&str>,
    ) -> Result<Term, Box<dyn std::error::Error>> {
        value_to_json_term(self.field, value, path, EXPAND_DOTS, self.is_datetime)
    }
}
