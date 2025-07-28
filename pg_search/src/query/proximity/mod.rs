use crate::api::Regex;
use pgrx::{InOutFuncs, PostgresType, StringInfo};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::ffi::CStr;
use tantivy::schema::Field;
use tantivy::SegmentReader;

pub mod query;
mod scorer;
mod weight;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum ProximityTermStyle {
    Term(String),
    Regex(Regex, usize),
}

impl ProximityTermStyle {
    pub fn as_str(&self) -> &str {
        match self {
            ProximityTermStyle::Term(term) => term.as_str(),
            ProximityTermStyle::Regex(regex, ..) => regex.as_str(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, PostgresType, Serialize, Deserialize)]
#[inoutfuncs]
pub enum ProximityClause {
    Term(ProximityTermStyle),
    Clauses(Vec<ProximityClause>),
    Proximity {
        left: Box<ProximityClause>,
        distance: ProximityDistance,
        right: Box<ProximityClause>,
    },
}

impl InOutFuncs for ProximityClause {
    fn input(input: &CStr) -> Self
    where
        Self: Sized,
    {
        if let Ok(from_json) = serde_json::from_slice::<ProximityClause>(input.to_bytes()) {
            from_json
        } else {
            // assume it's just a string
            ProximityClause::Term(ProximityTermStyle::Term(
                input
                    .to_str()
                    .expect("input should be valid UTF8")
                    .to_string(),
            ))
        }
    }

    fn output(&self, buffer: &mut StringInfo) {
        if let ProximityClause::Term(ProximityTermStyle::Term(s)) = self {
            buffer.push_str(s);
        } else {
            serde_json::to_writer(buffer, self).unwrap();
        }
    }
}

#[derive(Copy, Clone)]
pub enum WhichTerms {
    Left,
    Right,
    All,
}

impl ProximityClause {
    pub fn is_empty(&self) -> bool {
        match self {
            ProximityClause::Term(_) => false,
            ProximityClause::Clauses(clauses) => {
                clauses.is_empty() || clauses.iter().all(|clause| clause.is_empty())
            }
            ProximityClause::Proximity { left, right, .. } => left.is_empty() || right.is_empty(),
        }
    }

    pub fn terms<'a>(
        &'a self,
        field: Field,
        segment_reader: Option<&'a SegmentReader>,
        which_terms: WhichTerms,
    ) -> tantivy::Result<impl Iterator<Item = Cow<'a, ProximityTermStyle>>> {
        let iter: Box<dyn Iterator<Item = Cow<'a, ProximityTermStyle>>> = match self {
            ProximityClause::Term(term @ ProximityTermStyle::Term(_)) => {
                Box::new(std::iter::once(Cow::Borrowed(term)))
            }
            ProximityClause::Term(term @ ProximityTermStyle::Regex(..))
                if segment_reader.is_none() =>
            {
                Box::new(std::iter::once(Cow::Borrowed(term)))
            }
            ProximityClause::Term(ProximityTermStyle::Regex(re, ..)) => {
                let segment_reader = segment_reader.unwrap();
                let regex = tantivy_fst::Regex::new(re.as_str()).unwrap_or_else(|e| panic!("{e}"));
                let inverted_index = segment_reader.inverted_index(field)?;
                let dict = inverted_index.terms();
                let mut term_stream = dict.search_with_state(regex).into_stream()?;

                let mut terms = Vec::new();
                while let Some((bytes, ..)) = term_stream.next() {
                    terms.push(Cow::Owned(ProximityTermStyle::Term(
                        String::from_utf8_lossy(bytes).to_string(),
                    )));
                }
                Box::new(terms.into_iter())
            }

            ProximityClause::Clauses(clauses) => {
                let iter = clauses
                    .iter()
                    .map(move |clause| clause.terms(field, segment_reader, which_terms))
                    .collect::<tantivy::Result<Vec<_>>>()?;

                Box::new(iter.into_iter().flatten())
            }
            ProximityClause::Proximity { left, right, .. } => match which_terms {
                WhichTerms::Left => Box::new(left.terms(field, segment_reader, which_terms)?),
                WhichTerms::Right => Box::new(right.terms(field, segment_reader, which_terms)?),
                WhichTerms::All => Box::new(
                    left.terms(field, segment_reader, which_terms)?
                        .chain(right.terms(field, segment_reader, which_terms)?),
                ),
            },
        };
        Ok(iter)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum ProximityDistance {
    InOrder(u32),
    AnyOrder(u32),
}

impl ProximityDistance {
    #[inline(always)]
    pub fn diff(&self, l: u32, r: u32) -> u32 {
        match self {
            ProximityDistance::InOrder(_) => r.wrapping_sub(l),
            ProximityDistance::AnyOrder(_) => r.abs_diff(l),
        }
    }

    pub fn distance(&self) -> u32 {
        match self {
            ProximityDistance::InOrder(distance) => *distance,
            ProximityDistance::AnyOrder(distance) => *distance,
        }
    }

    pub fn in_order(&self) -> bool {
        matches!(self, ProximityDistance::InOrder(_))
    }
}
