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

/// `TokenLengthFilter` removes tokens that are longer
/// than a given number of bytes or shorter than a given number of bytes (in UTF-8 representation).
#[derive(Clone)]
pub struct TokenLengthFilter {
    min: Option<usize>,
    max: Option<usize>,
}

impl TokenLengthFilter {
    /// Creates a `TokenLengthFilter` given a minimum and maximum number of bytes of the UTF-8 representation.
    pub fn new(min: Option<usize>, max: Option<usize>) -> TokenLengthFilter {
        TokenLengthFilter { min, max }
    }
}

impl<T> TokenLengthFilterStream<T> {
    fn predicate(&self, token: &Token) -> bool {
        if let Some(min) = self.min {
            if token.text.len() < min {
                return false;
            }
        }
        if let Some(max) = self.max {
            if token.text.len() > max {
                return false;
            }
        }
        true
    }
}

impl TokenFilter for TokenLengthFilter {
    type Tokenizer<T: Tokenizer> = TokenLengthFilterWrapper<T>;

    fn transform<T: Tokenizer>(self, tokenizer: T) -> TokenLengthFilterWrapper<T> {
        TokenLengthFilterWrapper {
            min: self.min,
            max: self.max,
            inner: tokenizer,
        }
    }
}

#[derive(Clone)]
pub struct TokenLengthFilterWrapper<T: Tokenizer> {
    min: Option<usize>,
    max: Option<usize>,
    inner: T,
}

impl<T: Tokenizer> Tokenizer for TokenLengthFilterWrapper<T> {
    type TokenStream<'a> = TokenLengthFilterStream<T::TokenStream<'a>>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        TokenLengthFilterStream {
            min: self.min,
            max: self.max,
            tail: self.inner.token_stream(text),
        }
    }
}

pub struct TokenLengthFilterStream<T> {
    min: Option<usize>,
    max: Option<usize>,
    tail: T,
}

impl<T: TokenStream> TokenStream for TokenLengthFilterStream<T> {
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
    use super::TokenLengthFilter;
    use tantivy::tokenizer::{SimpleTokenizer, TextAnalyzer, Token};

    #[test]
    fn test_token_length() {
        let tokens = token_stream_helper(
            "a sentence with a veryveryveryveryveryveryveryveryveryveryveryveryverylong token",
            Some(3),
            Some(20),
        );
        let expected_tokens = vec![
            Token {
                offset_from: 2,
                offset_to: 10,
                position: 1,
                text: "sentence".to_owned(),
                position_length: 1,
            },
            Token {
                offset_from: 11,
                offset_to: 15,
                position: 2,
                text: "with".to_owned(),
                position_length: 1,
            },
            Token {
                offset_from: 75,
                offset_to: 80,
                position: 5,
                text: "token".to_owned(),
                position_length: 1,
            },
        ];
        assert_eq!(tokens, expected_tokens);

        let tokens = token_stream_helper(
            "a sentence with a veryveryveryveryveryveryveryveryveryveryveryveryverylong token",
            Some(5),
            None,
        );
        let expected_tokens = vec![
            Token {
                offset_from: 2,
                offset_to: 10,
                position: 1,
                text: "sentence".to_owned(),
                position_length: 1,
            },
            Token {
                offset_from: 18,
                offset_to: 74,
                position: 4,
                text: "veryveryveryveryveryveryveryveryveryveryveryveryverylong".to_owned(),
                position_length: 1,
            },
            Token {
                offset_from: 75,
                offset_to: 80,
                position: 5,
                text: "token".to_owned(),
                position_length: 1,
            },
        ];
        assert_eq!(tokens, expected_tokens);

        let tokens = token_stream_helper(
            "a sentence with a veryveryveryveryveryveryveryveryveryveryveryveryverylong token",
            None,
            Some(20),
        );
        let expected_tokens = vec![
            Token {
                offset_from: 0,
                offset_to: 1,
                position: 0,
                text: "a".to_owned(),
                position_length: 1,
            },
            Token {
                offset_from: 2,
                offset_to: 10,
                position: 1,
                text: "sentence".to_owned(),
                position_length: 1,
            },
            Token {
                offset_from: 11,
                offset_to: 15,
                position: 2,
                text: "with".to_owned(),
                position_length: 1,
            },
            Token {
                offset_from: 16,
                offset_to: 17,
                position: 3,
                text: "a".to_owned(),
                position_length: 1,
            },
            Token {
                offset_from: 75,
                offset_to: 80,
                position: 5,
                text: "token".to_owned(),
                position_length: 1,
            },
        ];
        assert_eq!(tokens, expected_tokens);
    }

    fn token_stream_helper(text: &str, min: Option<usize>, max: Option<usize>) -> Vec<Token> {
        let mut a = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(TokenLengthFilter::new(min, max))
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
