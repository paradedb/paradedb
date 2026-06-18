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

//! A [`tantivy`] tokenizer backed by [`alyze`](https://github.com/turbopuffer/alyze), a
//! high-performance [UAX #29](https://www.unicode.org/reports/tr29/) word segmenter implemented
//! as a hand-rolled deterministic finite automaton (DFA).
//!
//! `alyze` exposes a push-based API: [`alyze::uax29::word::tokenize`] drives a callback with each
//! word boundary it discovers. tantivy's [`TokenStream`] is pull-based, so we eagerly run the
//! segmenter once per input in [`Tokenizer::token_stream`], collecting the byte spans we want to
//! keep, then hand them out one at a time from [`TokenStream::advance`].
//!
//! Each boundary callback reports the [`TokenProperties`] of the span that just closed. When
//! `word_like_only` is set (the default) we keep only spans alyze classifies as "word-like"
//! (letters, digits, CJK ideographs, emoji, etc.), discarding the whitespace and punctuation
//! segments that UAX #29 also emits. This mirrors the behavior of the `unicode_words` tokenizer.

use alyze::uax29::word::{tokenize, Options};
use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

#[derive(Clone)]
pub struct AlyzeTokenizer {
    token: Token,
    /// When true, only word-like spans are emitted (whitespace/punctuation are dropped).
    word_like_only: bool,
}

impl Default for AlyzeTokenizer {
    fn default() -> Self {
        Self::new(true)
    }
}

impl AlyzeTokenizer {
    pub fn new(word_like_only: bool) -> Self {
        Self {
            token: Token::default(),
            word_like_only,
        }
    }
}

pub struct AlyzeTokenStream<'a> {
    text: &'a str,
    token: &'a mut Token,
    /// Byte spans `[offset_from, offset_to)` of the tokens we decided to keep, in order.
    spans: std::vec::IntoIter<(usize, usize)>,
}

impl Tokenizer for AlyzeTokenizer {
    type TokenStream<'a> = AlyzeTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        let word_like_only = self.word_like_only;

        // `alyze` reports each boundary as the *end* of the span that just closed, together with
        // that span's properties. The very first callback is the leading boundary at byte 0, which
        // closes no span (it carries default/vacuous properties), so we skip it by only emitting
        // once we have a previous boundary to pair with.
        let mut spans: Vec<(usize, usize)> = Vec::new();
        let mut prev: Option<usize> = None;
        tokenize(text, Options::default(), |boundary, props| {
            if let Some(start) = prev {
                if !word_like_only || props.is_word_like() {
                    spans.push((start, boundary));
                }
            }
            prev = Some(boundary);
            true
        });

        // Reset the position counter so the first `advance()` yields position 0 (it wraps from
        // `usize::MAX`, which is `Token`'s default).
        self.token.position = usize::MAX;

        AlyzeTokenStream {
            text,
            token: &mut self.token,
            spans: spans.into_iter(),
        }
    }
}

impl TokenStream for AlyzeTokenStream<'_> {
    fn advance(&mut self) -> bool {
        match self.spans.next() {
            Some((offset_from, offset_to)) => {
                self.token.position = self.token.position.wrapping_add(1);
                self.token.offset_from = offset_from;
                self.token.offset_to = offset_to;
                self.token.text.clear();
                self.token.text.push_str(&self.text[offset_from..offset_to]);
                true
            }
            None => false,
        }
    }

    fn token(&self) -> &Token {
        self.token
    }

    fn token_mut(&mut self) -> &mut Token {
        self.token
    }
}

#[cfg(test)]
mod tests {
    use super::AlyzeTokenizer;
    use tantivy::tokenizer::{TokenStream, Tokenizer};

    fn collect(tokenizer: &mut AlyzeTokenizer, text: &str) -> Vec<(String, usize, usize, usize)> {
        let mut stream = tokenizer.token_stream(text);
        let mut tokens = vec![];
        while stream.advance() {
            let token = stream.token();
            tokens.push((
                token.text.clone(),
                token.position,
                token.offset_from,
                token.offset_to,
            ));
        }
        tokens
    }

    #[test]
    fn test_word_like_only_default() {
        // Default keeps only word-like spans: punctuation and whitespace are dropped, offsets and
        // positions track the original text.
        let mut tokenizer = AlyzeTokenizer::default();
        let tokens = collect(&mut tokenizer, "Hello, world! 123");
        assert_eq!(
            tokens,
            vec![
                ("Hello".into(), 0, 0, 5),
                ("world".into(), 1, 7, 12),
                ("123".into(), 2, 14, 17),
            ]
        );
    }

    #[test]
    fn test_contractions_and_cjk_and_emoji() {
        let mut tokenizer = AlyzeTokenizer::default();
        let tokens = collect(&mut tokenizer, "won't 中文 👍");
        let texts: Vec<String> = tokens.iter().map(|(t, ..)| t.clone()).collect();
        assert_eq!(texts, vec!["won't", "中", "文", "👍"]);
    }

    #[test]
    fn test_keep_all_segments() {
        // With word_like_only disabled, every UAX #29 segment is emitted, including the spaces
        // and punctuation between words.
        let mut tokenizer = AlyzeTokenizer::new(false);
        let texts: Vec<String> = collect(&mut tokenizer, "a, b")
            .into_iter()
            .map(|(t, ..)| t)
            .collect();
        assert_eq!(texts, vec!["a", ",", " ", "b"]);
    }

    #[test]
    fn test_empty_input() {
        let mut tokenizer = AlyzeTokenizer::default();
        assert!(collect(&mut tokenizer, "").is_empty());
    }

    #[test]
    fn test_stream_reuse_resets_position() {
        // Re-running the same tokenizer must restart positions at 0 rather than continuing.
        let mut tokenizer = AlyzeTokenizer::default();
        let _ = collect(&mut tokenizer, "alpha beta");
        let tokens = collect(&mut tokenizer, "gamma delta");
        assert_eq!(tokens[0], ("gamma".into(), 0, 0, 5));
        assert_eq!(tokens[1], ("delta".into(), 1, 6, 11));
    }
}
