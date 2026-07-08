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

use tantivy::tokenizer::{Token, TokenStream, Tokenizer};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Default)]
pub struct UnicodeWordsTokenizer {
    token: Token,
    remove_emojis: bool,
}

impl UnicodeWordsTokenizer {
    pub fn new(remove_emojis: bool) -> Self {
        Self {
            token: Default::default(),
            remove_emojis,
        }
    }
}

pub struct UnicodeWordsTokenStream<'a> {
    remove_emojis: bool,
    iter: unicode_segmentation::UWordBounds<'a>,
    token: &'a mut Token,
    text: &'a str,
}

impl Tokenizer for UnicodeWordsTokenizer {
    type TokenStream<'a> = UnicodeWordsTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        // tantivy reuses a single tokenizer instance across every document in a segment, and the
        // `Token` (with its `position` counter) lives on the tokenizer. Reset it per call so
        // positions restart at 0 for each document, matching tantivy's built-in tokenizers
        // (`SimpleTokenizer`/`WhitespaceTokenizer` both call `self.token.reset()` here). Without
        // this the counter accumulates across the whole segment, producing huge position values
        // that bloat the positions index (and risk a u32 overflow on very large segments).
        self.token.reset();
        UnicodeWordsTokenStream {
            remove_emojis: self.remove_emojis,
            iter: text.split_word_bounds(),
            token: &mut self.token,
            text,
        }
    }
}

impl TokenStream for UnicodeWordsTokenStream<'_> {
    fn advance(&mut self) -> bool {
        loop {
            if let Some(next) = self.iter.next() {
                let is_word = next.unicode_words().next().is_some();
                let keep = is_word || (!self.remove_emojis && emojis::get(next).is_some());

                if !keep {
                    continue;
                }
                self.token.position = self.token.position.wrapping_add(1);

                // Calculate byte offsets
                let offset_from = unsafe { next.as_ptr().offset_from(self.text.as_ptr()) as usize };
                let offset_to = offset_from + next.len();

                self.token.offset_from = offset_from;
                self.token.offset_to = offset_to;

                self.token.text.clear();
                self.token.text.push_str(next);
                return true;
            }
            return false;
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
    use crate::unicode_words::UnicodeWordsTokenizer;
    use tantivy::tokenizer::{TokenStream, Tokenizer};

    #[test]
    fn test_unicode_words_with_emojis() {
        let mut tokenizer = UnicodeWordsTokenizer::default();
        let text = "it's Paul's birthday today!  🎂  hurray!";
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

        assert_eq!(
            tokens,
            vec![
                ("it's".into(), 0, 0, 4),
                ("Paul's".into(), 1, 5, 11),
                ("birthday".into(), 2, 12, 20),
                ("today".into(), 3, 21, 26),
                ("🎂".into(), 4, 29, 33),
                ("hurray".into(), 5, 35, 41)
            ]
        )
    }
    #[test]
    fn test_unicode_words_without_emojis() {
        let mut tokenizer = UnicodeWordsTokenizer::new(true);
        let text = "it's Paul's birthday today!  🎂  hurray!";
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

        assert_eq!(
            tokens,
            vec![
                ("it's".into(), 0, 0, 4),
                ("Paul's".into(), 1, 5, 11),
                ("birthday".into(), 2, 12, 20),
                ("today".into(), 3, 21, 26),
                ("hurray".into(), 4, 35, 41)
            ]
        )
    }

    /// Regression test: tantivy reuses one tokenizer instance across every document in a segment,
    /// so token positions must restart at 0 for each `token_stream` call. Previously the position
    /// counter leaked across documents, producing ever-growing positions that bloat the positions
    /// index (~2.7x on a real corpus) and risk a u32 overflow on large segments.
    #[test]
    fn test_position_resets_between_documents() {
        fn positions(stream: &mut impl TokenStream) -> Vec<usize> {
            let mut p = vec![];
            while stream.advance() {
                p.push(stream.token().position);
            }
            p
        }

        // One tokenizer instance reused across two "documents", as tantivy does during indexing.
        let mut tokenizer = UnicodeWordsTokenizer::default();
        let first = positions(&mut tokenizer.token_stream("alpha beta gamma"));
        let second = positions(&mut tokenizer.token_stream("delta epsilon"));

        assert_eq!(first, vec![0, 1, 2]);
        // The second document must start at position 0 again, not continue from 3.
        assert_eq!(second, vec![0, 1]);
    }
}
