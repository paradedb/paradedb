// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

//! Wrapper around Tantivy's NgramTokenizer that can optionally emit sequential positions.
//!
//! Tantivy's built-in NgramTokenizer always sets `token.position = 0`, which breaks positional
//! queries (phrase/proximity). When `positions=true` AND `min_gram==max_gram`, we can safely
//! emit sequential token positions (0, 1, 2, ...) for each produced gram.

use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

#[derive(Clone)]
pub struct NgramTokenizer {
    inner: tantivy::tokenizer::NgramTokenizer,
    enable_positions: bool,
}

impl NgramTokenizer {
    pub fn new(
        min_gram: usize,
        max_gram: usize,
        prefix_only: bool,
        positions: bool,
    ) -> tantivy::Result<Self> {
        if positions && min_gram != max_gram {
            return Err(tantivy::TantivyError::InvalidArgument(
                "min_gram must equal max_gram when positions are enabled".to_string(),
            ));
        }
        let inner = tantivy::tokenizer::NgramTokenizer::new(min_gram, max_gram, prefix_only)?;
        Ok(Self {
            inner,
            enable_positions: positions,
        })
    }
}

pub struct NgramTokenStream<'a> {
    inner: Box<dyn TokenStream + 'a>,
    enable_positions: bool,
    position: usize,
}

impl Tokenizer for NgramTokenizer {
    type TokenStream<'a> = NgramTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        let inner = self.inner.token_stream(text);
        NgramTokenStream {
            inner: Box::new(inner),
            enable_positions: self.enable_positions,
            position: 0,
        }
    }
}

impl TokenStream for NgramTokenStream<'_> {
    fn advance(&mut self) -> bool {
        if self.inner.advance() {
            if self.enable_positions {
                self.inner.token_mut().position = self.position;
                self.position = self.position.wrapping_add(1);
            }
            true
        } else {
            false
        }
    }

    fn token(&self) -> &Token {
        self.inner.token()
    }

    fn token_mut(&mut self) -> &mut Token {
        self.inner.token_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_positions_enabled_fixed_gram() {
        let mut tokenizer = NgramTokenizer::new(3, 3, false, true).unwrap();
        let mut stream = tokenizer.token_stream("hello");

        assert!(stream.advance());
        assert_eq!(stream.token().text, "hel");
        assert_eq!(stream.token().position, 0);

        assert!(stream.advance());
        assert_eq!(stream.token().text, "ell");
        assert_eq!(stream.token().position, 1);

        assert!(stream.advance());
        assert_eq!(stream.token().text, "llo");
        assert_eq!(stream.token().position, 2);
    }

    #[test]
    fn test_positions_disabled_defaults_to_zero() {
        let mut tokenizer = NgramTokenizer::new(3, 3, false, false).unwrap();
        let mut stream = tokenizer.token_stream("hello");

        while stream.advance() {
            assert_eq!(stream.token().position, 0);
        }
    }
}
