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

//! Wrapper around tantivy_jieba::JiebaTokenizer that normalizes token positions.
//!
//! The upstream tantivy_jieba crate incorrectly sets token.position to character
//! offsets instead of sequential token ordinals. This causes phrase queries to
//! fail because PhraseQuery expects tokens at consecutive positions (0, 1, 2, ...).
//!
//! This wrapper intercepts the token stream and fixes positions to be sequential.
//!
//! TODO: See https://github.com/jiegec/tantivy-jieba/issues/24 about fixing this upstream.

use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

/// A wrapper around JiebaTokenizer that normalizes token positions to be sequential.
#[derive(Clone)]
pub struct JiebaTokenizer {
    inner: tantivy_jieba::JiebaTokenizer,
}

impl JiebaTokenizer {
    pub fn new() -> Self {
        Self {
            inner: tantivy_jieba::JiebaTokenizer {},
        }
    }
}

impl Default for JiebaTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tokenizer for JiebaTokenizer {
    type TokenStream<'a> = JiebaTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        let inner = self.inner.token_stream(text);
        JiebaTokenStream {
            inner,
            position: usize::MAX, // Will wrap to 0 on first advance
        }
    }
}

/// Token stream wrapper that fixes positions to be sequential.
pub struct JiebaTokenStream<'a> {
    inner: tantivy_jieba::JiebaTokenStream<'a>,
    position: usize,
}

impl TokenStream for JiebaTokenStream<'_> {
    fn advance(&mut self) -> bool {
        if self.inner.advance() {
            // Increment position sequentially instead of using character offsets
            self.position = self.position.wrapping_add(1);
            self.inner.token_mut().position = self.position;
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

    fn tokenize(text: &str) -> Vec<(String, usize)> {
        let mut tokenizer = JiebaTokenizer::new();
        let mut stream = tokenizer.token_stream(text);
        let mut tokens = Vec::new();
        while stream.advance() {
            let token = stream.token();
            tokens.push((token.text.clone(), token.position));
        }
        tokens
    }

    #[test]
    fn test_sequential_positions() {
        // "我们都有光明的前途" should tokenize with sequential positions
        let tokens = tokenize("我们都有光明的前途");

        // Verify positions are sequential (0, 1, 2, 3, ...)
        for (i, (_, position)) in tokens.iter().enumerate() {
            assert_eq!(
                *position, i,
                "Expected position {} but got {} for token at index {}",
                i, position, i
            );
        }
    }

    #[test]
    fn test_phrase_compatible_positions() {
        // "转移就业" tokenizes to ["转移", "就业"]
        // With the fix, positions should be [0, 1] not [0, 2]
        let tokens = tokenize("转移就业");

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].1, 0, "First token should be at position 0");
        assert_eq!(tokens[1].1, 1, "Second token should be at position 1");
    }
}
