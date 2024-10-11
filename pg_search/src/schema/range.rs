use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TantivyRange<T> {
    lower: Option<T>,
    upper: Option<T>,
    lower_inclusive: bool,
    upper_inclusive: bool,
    lower_unbounded: bool,
    upper_unbounded: bool,
}

pub fn lower_key() -> String {
    "lower".to_string()
}

pub fn upper_key() -> String {
    "upper".to_string()
}

pub fn lower_inclusive_key() -> String {
    "lower_inclusive".to_string()
}

pub fn upper_inclusive_key() -> String {
    "upper_inclusive".to_string()
}

pub fn lower_unbounded_key() -> String {
    "lower_unbounded".to_string()
}

pub fn upper_unbounded_key() -> String {
    "upper_unbounded".to_string()
}

pub struct TantivyRangeBuilder<T> {
    lower: Option<T>,
    upper: Option<T>,
    lower_inclusive: Option<bool>,
    upper_inclusive: Option<bool>,
    lower_unbounded: Option<bool>,
    upper_unbounded: Option<bool>,
}

impl<T> TantivyRangeBuilder<T>
where
    T: Clone,
{
    pub fn new() -> Self {
        Self {
            lower: None,
            upper: None,
            lower_inclusive: None,
            upper_inclusive: None,
            lower_unbounded: None,
            upper_unbounded: None,
        }
    }

    pub fn lower(mut self, lower: Option<T>) -> Self {
        self.lower = lower;
        self
    }

    pub fn upper(mut self, upper: Option<T>) -> Self {
        self.upper = upper;
        self
    }

    pub fn lower_inclusive(mut self, lower_inclusive: bool) -> Self {
        self.lower_inclusive = Some(lower_inclusive);
        self
    }

    pub fn upper_inclusive(mut self, upper_inclusive: bool) -> Self {
        self.upper_inclusive = Some(upper_inclusive);
        self
    }

    pub fn lower_unbounded(mut self, lower_unbounded: bool) -> Self {
        self.lower_unbounded = Some(lower_unbounded);
        self
    }

    pub fn upper_unbounded(mut self, upper_unbounded: bool) -> Self {
        self.upper_unbounded = Some(upper_unbounded);
        self
    }

    pub fn build(self) -> TantivyRange<T> {
        TantivyRange {
            lower: self.lower,
            upper: self.upper,
            lower_inclusive: self.lower_inclusive.unwrap_or(true),
            upper_inclusive: self.upper_inclusive.unwrap_or(false),
            lower_unbounded: self.lower_unbounded.unwrap_or(false),
            upper_unbounded: self.upper_unbounded.unwrap_or(false),
        }
    }
}
