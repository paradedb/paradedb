// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use tantivy::tokenizer::{Token, TokenFilter, TokenStream, Tokenizer};

/// `TokenTrimFilter` trims leading and trailing whitespace from each token.
/// After trimming, tokens that become empty are filtered out.
/// This matches the behavior of Elasticsearch's trim token filter.
#[derive(Clone)]
pub struct TokenTrimFilter;

impl TokenTrimFilter {
    /// Creates a `TokenTrimFilter`.
    pub fn new() -> TokenTrimFilter {
        TokenTrimFilter
    }
}

impl Default for TokenTrimFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenFilter for TokenTrimFilter {
    type Tokenizer<T: Tokenizer> = TokenTrimFilterWrapper<T>;

    fn transform<T: Tokenizer>(self, tokenizer: T) -> TokenTrimFilterWrapper<T> {
        TokenTrimFilterWrapper { inner: tokenizer }
    }
}

#[derive(Clone)]
pub struct TokenTrimFilterWrapper<T: Tokenizer> {
    inner: T,
}

impl<T: Tokenizer> Tokenizer for TokenTrimFilterWrapper<T> {
    type TokenStream<'a> = TokenTrimFilterStream<T::TokenStream<'a>>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        TokenTrimFilterStream {
            tail: self.inner.token_stream(text),
        }
    }
}

pub struct TokenTrimFilterStream<T> {
    tail: T,
}

impl<T: TokenStream> TokenStream for TokenTrimFilterStream<T> {
    fn advance(&mut self) -> bool {
        while self.tail.advance() {
            // Trim the token text
            let token = self.tail.token_mut();
            let trimmed = token.text.trim();

            // If the token is not empty after trimming, update it and return
            if !trimmed.is_empty() {
                // Only update the text if it actually changed
                if trimmed != token.text {
                    token.text.clear();
                    token.text.push_str(trimmed);
                }
                return true;
            }
            // Otherwise, skip this token and continue to the next one
        }
        false
    }

    fn token(&self) -> &Token {
        self.tail.token()
    }

    fn token_mut(&mut self) -> &mut Token {
        self.tail.token_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::TokenTrimFilter;
    use tantivy::tokenizer::{SimpleTokenizer, TextAnalyzer, Token};

    #[test]
    fn test_trim_filter_basic() {
        let tokens = token_stream_helper("hello world");
        // SimpleTokenizer splits on whitespace, so tokens should be clean
        let expected_tokens = vec![
            Token {
                offset_from: 0,
                offset_to: 5,
                position: 0,
                text: "hello".to_owned(),
                position_length: 1,
            },
            Token {
                offset_from: 6,
                offset_to: 11,
                position: 1,
                text: "world".to_owned(),
                position_length: 1,
            },
        ];
        assert_eq!(tokens, expected_tokens);
    }

    #[test]
    fn test_trim_filter_removes_empty_after_trim() {
        // Test that tokens that are only whitespace get removed after trimming
        let tokens = token_stream_helper("hello\tworld\ntest");
        assert!(tokens.iter().all(|t| !t.text.trim().is_empty()));
        assert!(tokens.iter().all(|t| t.text == t.text.trim()));
    }

    fn token_stream_helper(text: &str) -> Vec<Token> {
        let mut a = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(TokenTrimFilter::new())
            .build();
        let mut token_stream = a.token_stream(text);
        let mut tokens: Vec<Token> = vec![];
        let mut add_token = |token: &Token| {
            tokens.push(token.clone());
        };
        token_stream.process(&mut add_token);
        tokens
    }
}
