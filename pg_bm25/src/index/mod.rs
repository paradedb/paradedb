#![allow(unused)]
pub mod score;
pub mod search;
pub mod state;

use crate::schema::{SearchConfig, SearchField, SearchFieldId};
use core::panic;
use derive_more::{AsRef, Display, From};
use once_cell::sync::Lazy;
use pgrx::pg_sys::Alias;
use score::SearchIndexScore;
pub use search::*;
use shared::postgres::transaction::{Transaction, TransactionError};
use std::{
    borrow::Borrow,
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
};
use tantivy::{schema::Field, DocAddress, Document, Snippet, SnippetGenerator};
use thiserror::Error;
