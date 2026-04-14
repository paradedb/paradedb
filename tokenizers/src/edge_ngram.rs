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

use icu_properties::props::GeneralCategory;
use icu_properties::CodePointMapData;
use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenCharClass {
    Letter,
    Digit,
    Whitespace,
    Punctuation,
    Symbol,
}

impl std::str::FromStr for TokenCharClass {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "letter" => Ok(Self::Letter),
            "digit" => Ok(Self::Digit),
            "whitespace" => Ok(Self::Whitespace),
            "punctuation" => Ok(Self::Punctuation),
            "symbol" => Ok(Self::Symbol),
            other => Err(format!("unknown token_chars class: '{other}'. expected one of: letter, digit, whitespace, punctuation, symbol")),
        }
    }
}

impl TokenCharClass {
    fn matches(&self, c: char) -> bool {
        match self {
            Self::Letter => c.is_alphabetic(),
            Self::Digit => c.is_numeric(),
            Self::Whitespace => c.is_whitespace(),
            Self::Punctuation | Self::Symbol => {
                let gc = CodePointMapData::<GeneralCategory>::new();
                let cat = gc.get(c);
                match self {
                    Self::Punctuation => matches!(
                        cat,
                        GeneralCategory::ConnectorPunctuation
                            | GeneralCategory::DashPunctuation
                            | GeneralCategory::ClosePunctuation
                            | GeneralCategory::FinalPunctuation
                            | GeneralCategory::InitialPunctuation
                            | GeneralCategory::OtherPunctuation
                            | GeneralCategory::OpenPunctuation
                    ),
                    Self::Symbol => matches!(
                        cat,
                        GeneralCategory::CurrencySymbol
                            | GeneralCategory::ModifierSymbol
                            | GeneralCategory::MathSymbol
                            | GeneralCategory::OtherSymbol
                    ),
                    _ => unreachable!(),
                }
            }
        }
    }
}

fn matches_any(c: char, classes: &[TokenCharClass]) -> bool {
    classes.iter().any(|cls| cls.matches(c))
}

#[derive(Clone)]
pub struct EdgeNgramTokenizer {
    min_gram: usize,
    max_gram: usize,
    token_chars: Vec<TokenCharClass>,
}

impl EdgeNgramTokenizer {
    pub fn new(
        min_gram: usize,
        max_gram: usize,
        token_chars: Vec<TokenCharClass>,
    ) -> tantivy::Result<Self> {
        if min_gram < 1 {
            return Err(tantivy::TantivyError::InvalidArgument(
                "min_gram must be >= 1".to_string(),
            ));
        }
        if max_gram < min_gram {
            return Err(tantivy::TantivyError::InvalidArgument(
                "max_gram must be >= min_gram".to_string(),
            ));
        }
        Ok(Self {
            min_gram,
            max_gram,
            token_chars,
        })
    }
}

pub struct EdgeNgramTokenStream<'a> {
    text: &'a str,
    min_gram: usize,
    max_gram: usize,
    token_chars: Vec<TokenCharClass>,
    token: Token,
    // Scanning state: byte offset in the input where we look for the next word
    scan_offset: usize,
    // Current word we're emitting grams for
    word_start: usize,
    word_char_count: usize,
    // How many chars of the current word we've emitted so far
    current_gram_chars: usize,
    // Whether we're currently emitting grams for a word
    in_word: bool,
    // Position counter (increments per word)
    position: usize,
    first_advance: bool,
}

impl Tokenizer for EdgeNgramTokenizer {
    type TokenStream<'a> = EdgeNgramTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        EdgeNgramTokenStream {
            text,
            min_gram: self.min_gram,
            max_gram: self.max_gram,
            token_chars: self.token_chars.clone(),
            token: Token::default(),
            scan_offset: 0,
            word_start: 0,
            word_char_count: 0,
            current_gram_chars: 0,
            in_word: false,
            position: 0,
            first_advance: true,
        }
    }
}

impl EdgeNgramTokenStream<'_> {
    fn find_next_word(&mut self) -> bool {
        let len = self.text.len();

        // Skip non-matching characters
        let mut offset = self.scan_offset;
        for c in self.text[offset..].chars() {
            if matches_any(c, &self.token_chars) {
                break;
            }
            offset += c.len_utf8();
        }

        if offset >= len {
            return false;
        }

        // Collect matching characters
        self.word_start = offset;
        self.word_char_count = 0;
        for c in self.text[offset..].chars() {
            if !matches_any(c, &self.token_chars) {
                break;
            }
            offset += c.len_utf8();
            self.word_char_count += 1;
        }
        self.scan_offset = offset;
        true
    }
}

impl TokenStream for EdgeNgramTokenStream<'_> {
    fn advance(&mut self) -> bool {
        loop {
            if self.in_word {
                self.current_gram_chars += 1;
                if self.current_gram_chars > self.max_gram
                    || self.current_gram_chars > self.word_char_count
                {
                    self.in_word = false;
                    continue;
                }

                // Compute byte end for current_gram_chars characters from word_start
                let byte_end = self.text[self.word_start..]
                    .chars()
                    .take(self.current_gram_chars)
                    .map(|c| c.len_utf8())
                    .sum::<usize>()
                    + self.word_start;

                self.token.text.clear();
                self.token
                    .text
                    .push_str(&self.text[self.word_start..byte_end]);
                self.token.offset_from = self.word_start;
                self.token.offset_to = byte_end;
                self.token.position = self.position;
                return true;
            }

            // Find next word
            if !self.find_next_word() {
                return false;
            }

            // Skip words shorter than min_gram
            if self.word_char_count < self.min_gram {
                continue;
            }

            if !self.first_advance {
                self.position += 1;
            }
            self.first_advance = false;
            self.in_word = true;
            self.current_gram_chars = self.min_gram - 1; // will be incremented to min_gram on next loop
        }
    }

    fn token(&self) -> &Token {
        &self.token
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_tokens(tokenizer: &mut EdgeNgramTokenizer, text: &str) -> Vec<(String, usize)> {
        let mut stream = tokenizer.token_stream(text);
        let mut tokens = Vec::new();
        while stream.advance() {
            tokens.push((stream.token().text.clone(), stream.token().position));
        }
        tokens
    }

    fn collect_text(tokenizer: &mut EdgeNgramTokenizer, text: &str) -> Vec<String> {
        collect_tokens(tokenizer, text)
            .into_iter()
            .map(|(t, _)| t)
            .collect()
    }

    #[test]
    fn test_basic() {
        let mut tok =
            EdgeNgramTokenizer::new(2, 5, vec![TokenCharClass::Letter, TokenCharClass::Digit])
                .unwrap();
        assert_eq!(
            collect_text(&mut tok, "Quick Fox"),
            vec!["Qu", "Qui", "Quic", "Quick", "Fo", "Fox"]
        );
    }

    #[test]
    fn test_defaults() {
        let mut tok =
            EdgeNgramTokenizer::new(1, 2, vec![TokenCharClass::Letter, TokenCharClass::Digit])
                .unwrap();
        assert_eq!(
            collect_text(&mut tok, "Quick Fox"),
            vec!["Q", "Qu", "F", "Fo"]
        );
    }

    #[test]
    fn test_words_shorter_than_min_gram_skipped() {
        let mut tok = EdgeNgramTokenizer::new(3, 5, vec![TokenCharClass::Letter]).unwrap();
        assert_eq!(collect_text(&mut tok, "I am here"), vec!["her", "here"]);
    }

    #[test]
    fn test_empty_input() {
        let mut tok = EdgeNgramTokenizer::new(1, 3, vec![TokenCharClass::Letter]).unwrap();
        assert_eq!(collect_text(&mut tok, ""), Vec::<String>::new());
    }

    #[test]
    fn test_unicode() {
        let mut tok = EdgeNgramTokenizer::new(1, 4, vec![TokenCharClass::Letter]).unwrap();
        assert_eq!(
            collect_text(&mut tok, "café"),
            vec!["c", "ca", "caf", "café"]
        );
    }

    #[test]
    fn test_token_chars_with_punctuation() {
        let mut tok = EdgeNgramTokenizer::new(
            2,
            5,
            vec![TokenCharClass::Letter, TokenCharClass::Punctuation],
        )
        .unwrap();
        // Hyphen is ASCII punctuation, so "Quick-Fox" is one word
        assert_eq!(
            collect_text(&mut tok, "Quick-Fox"),
            vec!["Qu", "Qui", "Quic", "Quick"]
        );
    }

    #[test]
    fn test_digits_as_tokens() {
        let mut tok =
            EdgeNgramTokenizer::new(1, 3, vec![TokenCharClass::Letter, TokenCharClass::Digit])
                .unwrap();
        assert_eq!(
            collect_text(&mut tok, "abc 123"),
            vec!["a", "ab", "abc", "1", "12", "123"]
        );
    }

    #[test]
    fn test_positions() {
        let mut tok = EdgeNgramTokenizer::new(2, 4, vec![TokenCharClass::Letter]).unwrap();
        let tokens = collect_tokens(&mut tok, "hello world");
        // All grams from "hello" share position 0, all from "world" share position 1
        assert_eq!(
            tokens,
            vec![
                ("he".to_string(), 0),
                ("hel".to_string(), 0),
                ("hell".to_string(), 0),
                ("wo".to_string(), 1),
                ("wor".to_string(), 1),
                ("worl".to_string(), 1),
            ]
        );
    }

    #[test]
    fn test_offsets() {
        let mut tok = EdgeNgramTokenizer::new(2, 3, vec![TokenCharClass::Letter]).unwrap();
        let mut stream = tok.token_stream("hi world");
        assert!(stream.advance());
        assert_eq!(stream.token().text, "hi");
        assert_eq!(stream.token().offset_from, 0);
        assert_eq!(stream.token().offset_to, 2);

        assert!(stream.advance());
        assert_eq!(stream.token().text, "wo");
        assert_eq!(stream.token().offset_from, 3);
        assert_eq!(stream.token().offset_to, 5);

        assert!(stream.advance());
        assert_eq!(stream.token().text, "wor");
        assert_eq!(stream.token().offset_from, 3);
        assert_eq!(stream.token().offset_to, 6);
    }

    #[test]
    fn test_max_gram_clamped_to_word_length() {
        let mut tok = EdgeNgramTokenizer::new(1, 10, vec![TokenCharClass::Letter]).unwrap();
        assert_eq!(collect_text(&mut tok, "hi"), vec!["h", "hi"]);
    }

    #[test]
    fn test_only_delimiters() {
        let mut tok = EdgeNgramTokenizer::new(1, 3, vec![TokenCharClass::Letter]).unwrap();
        assert_eq!(collect_text(&mut tok, "123 !@#"), Vec::<String>::new());
    }
}
