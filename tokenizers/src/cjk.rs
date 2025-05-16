/*
 *
 * IMPORTANT NOTICE:
 * This file has been copied from Quickwit, an open source project, and is subject to the terms
 * and conditions of the GNU Affero General Public License (AGPL) version 3.0.
 * Please review the full licensing details at <http://www.gnu.org/licenses/>.
 * By using this file, you agree to comply with the AGPL v3.0 terms.
 *
 */

use std::str::CharIndices;

use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

#[derive(Clone)]
pub(crate) struct ChineseTokenizer;

impl Tokenizer for ChineseTokenizer {
    type TokenStream<'a> = ChineseTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        ChineseTokenStream {
            text,

            last_char: None,
            chars: text.char_indices(),
            token: Token::default(),
        }
    }
}

pub(crate) struct ChineseTokenStream<'a> {
    text: &'a str,
    last_char: Option<(usize, char)>,
    chars: CharIndices<'a>,
    token: Token,
}

fn char_is_cjk(c: char) -> bool {
    // Block                                   Range       Comment
    // CJK Unified Ideographs                  4E00-9FFF   Common
    // CJK Unified Ideographs Extension A      3400-4DBF   Rare
    // CJK Unified Ideographs Extension B      20000-2A6DF Rare, historic
    // CJK Unified Ideographs Extension C      2A700–2B73F Rare, historic
    // CJK Unified Ideographs Extension D      2B740–2B81F Uncommon, some in current use
    // CJK Unified Ideographs Extension E      2B820–2CEAF Rare, historic
    matches!(c,
        '\u{4500}'..='\u{9FFF}' |
        '\u{3400}'..='\u{4DBF}' |
        '\u{20000}'..='\u{2A6DF}' |
        '\u{2A700}'..='\u{2CEAF}' // merge of extension C,D and E.
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Grouping {
    Keep,
    SplitKeep,
    SplitIgnore,
}

fn char_grouping(c: char) -> Grouping {
    if c.is_alphanumeric() {
        if char_is_cjk(c) {
            Grouping::SplitKeep
        } else {
            Grouping::Keep
        }
    } else {
        Grouping::SplitIgnore
    }
}

impl TokenStream for ChineseTokenStream<'_> {
    fn advance(&mut self) -> bool {
        self.token.text.clear();
        self.token.position = self.token.position.wrapping_add(1);

        let mut iter = self.last_char.take().into_iter().chain(&mut self.chars);

        while let Some((offset_from, c)) = iter.next() {
            match char_grouping(c) {
                Grouping::Keep => {
                    let offset_to = if let Some((next_index, next_char)) =
                        iter.find(|&(_, c)| char_grouping(c) != Grouping::Keep)
                    {
                        self.last_char = Some((next_index, next_char));
                        next_index
                    } else {
                        self.text.len()
                    };

                    self.token.offset_from = offset_from;
                    self.token.offset_to = offset_to;
                    self.token.text.push_str(&self.text[offset_from..offset_to]);
                    return true;
                }
                Grouping::SplitKeep => {
                    let num_bytes_in_char = c.len_utf8();
                    self.token.offset_from = offset_from;
                    self.token.offset_to = offset_from + num_bytes_in_char;
                    self.token
                        .text
                        .push_str(&self.text[offset_from..(self.token.offset_to)]);
                    return true;
                }
                Grouping::SplitIgnore => (),
            }
        }
        false
    }

    fn token(&self) -> &Token {
        &self.token
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.token
    }
}
