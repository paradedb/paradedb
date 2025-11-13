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

/// `TokenWhitespaceFilter` removes tokens that consist entirely of whitespace characters.
/// This is useful for tokenizers like Jieba that may produce whitespace-only tokens.
#[derive(Clone)]
pub struct TokenWhitespaceFilter;

impl TokenWhitespaceFilter {
    /// Creates a `TokenWhitespaceFilter`.
    pub fn new() -> TokenWhitespaceFilter {
        TokenWhitespaceFilter
    }
}

impl Default for TokenWhitespaceFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TokenWhitespaceFilterStream<T> {
    fn predicate(&self, token: &Token) -> bool {
        // Return false if the token is entirely whitespace, true otherwise
        !token.text.chars().all(|c| c.is_whitespace())
    }
}

impl TokenFilter for TokenWhitespaceFilter {
    type Tokenizer<T: Tokenizer> = TokenWhitespaceFilterWrapper<T>;

    fn transform<T: Tokenizer>(self, tokenizer: T) -> TokenWhitespaceFilterWrapper<T> {
        TokenWhitespaceFilterWrapper {
            inner: tokenizer,
        }
    }
}

#[derive(Clone)]
pub struct TokenWhitespaceFilterWrapper<T: Tokenizer> {
    inner: T,
}

impl<T: Tokenizer> Tokenizer for TokenWhitespaceFilterWrapper<T> {
    type TokenStream<'a> = TokenWhitespaceFilterStream<T::TokenStream<'a>>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        TokenWhitespaceFilterStream {
            tail: self.inner.token_stream(text),
        }
    }
}

pub struct TokenWhitespaceFilterStream<T> {
    tail: T,
}

impl<T: TokenStream> TokenStream for TokenWhitespaceFilterStream<T> {
    fn advance(&mut self) -> bool {
        while self.tail.advance() {
            if self.predicate(self.tail.token()) {
                return true;
            }
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
    use super::TokenWhitespaceFilter;
    use tantivy::tokenizer::{SimpleTokenizer, TextAnalyzer, Token};

    #[test]
    fn test_whitespace_filter_removes_space() {
        let tokens = token_stream_helper("hello world");
        // SimpleTokenizer splits on whitespace, so no space token should appear
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
    fn test_whitespace_filter_removes_tabs_and_newlines() {
        // This tests that various whitespace characters would be filtered
        // Note: SimpleTokenizer already splits on whitespace, so this is more
        // relevant for tokenizers like Jieba that might produce whitespace tokens
        let tokens = token_stream_helper("hello\tworld\ntest");
        assert!(tokens.iter().all(|t| !t.text.trim().is_empty()));
    }

    fn token_stream_helper(text: &str) -> Vec<Token> {
        let mut a = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(TokenWhitespaceFilter::new())
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
