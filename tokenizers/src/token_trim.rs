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
            let token = self.tail.token_mut();
            let original_len = token.text.len();
            let trimmed = token.text.trim();

            // Skip whitespace-only tokens.
            if trimmed.is_empty() {
                continue;
            }

            // Fast path when no trimming is needed.
            if trimmed.len() == original_len {
                return true;
            }

            let leading = original_len - token.text.trim_start().len();
            let trailing = original_len - token.text.trim_end().len();

            // Remove trailing bytes before removing the leading bytes to keep ranges valid.
            token.text.truncate(original_len - trailing);
            token.text.drain(..leading);

            token.offset_from += leading;
            token.offset_to -= trailing;
            return true;
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

    #[test]
    fn test_trim_filter_with_leading_trailing_whitespace() {
        // Test with RawTokenizer (like keyword tokenizer) which doesn't split on whitespace
        use tantivy::tokenizer::RawTokenizer;

        let mut a = TextAnalyzer::builder(RawTokenizer::default())
            .filter(TokenTrimFilter::new())
            .build();

        // Text with leading and trailing whitespace
        let text = "  hello world  ";
        let mut token_stream = a.token_stream(text);
        let mut tokens: Vec<Token> = vec![];
        let mut add_token = |token: &Token| {
            tokens.push(token.clone());
        };
        token_stream.process(&mut add_token);

        // Should have 1 token with whitespace trimmed
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "hello world");
    }

    #[test]
    fn test_trim_filter_removes_whitespace_only_token() {
        // Test that a token consisting only of whitespace is removed
        use tantivy::tokenizer::RawTokenizer;

        let mut a = TextAnalyzer::builder(RawTokenizer::default())
            .filter(TokenTrimFilter::new())
            .build();

        // Text with only whitespace
        let text = "   ";
        let mut token_stream = a.token_stream(text);
        let mut tokens: Vec<Token> = vec![];
        let mut add_token = |token: &Token| {
            tokens.push(token.clone());
        };
        token_stream.process(&mut add_token);

        // Should have 0 tokens (whitespace-only token removed)
        assert_eq!(tokens.len(), 0);
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
