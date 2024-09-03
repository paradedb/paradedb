/*
 *
 * IMPORTANT NOTICE:
 * This file has been copied from tantivy-analysis-contrib, an open source project, and is subject to the terms
 * and conditions of the Apache License, Version 2.0.
 * Please review the full licensing details at <http://www.apache.org/licenses/LICENSE-2.0>.
 * By using this file, you agree to comply with the Apache v2.0 terms.
 *
 */

use rust_icu_sys::UBreakIteratorType;
use rust_icu_ubrk::UBreakIterator;
use rust_icu_uloc;
use rust_icu_ustring::UChar;
use std::str::Chars;
use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

#[derive(Clone, Copy, Debug, Default)]
pub struct ICUTokenizer;

impl Tokenizer for ICUTokenizer {
    type TokenStream<'a> = ICUTokenizerTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        ICUTokenizerTokenStream::new(text)
    }
}

struct ICUBreakingWord<'a> {
    text: Chars<'a>,
    char_indices: Vec<(usize, char)>,
    default_breaking_iterator: UBreakIterator,
}

impl<'a> std::fmt::Debug for ICUBreakingWord<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ICUBreakingWord")
            .field("text", &self.text)
            .finish()
    }
}

impl<'a> From<&'a str> for ICUBreakingWord<'a> {
    fn from(text: &'a str) -> Self {
        let loc = rust_icu_uloc::get_default();
        let ustr = &UChar::try_from(text).expect("is an encodable character");
        // Implementation from a similar fix in https://github.com/jiegec/tantivy-jieba/pull/5
        // referenced by Tantivy issue https://github.com/quickwit-oss/tantivy/issues/1134
        let mut char_indices = text.char_indices().collect::<Vec<_>>();
        char_indices.push((text.len(), '\0'));

        ICUBreakingWord {
            text: text.chars(),
            char_indices,
            default_breaking_iterator: UBreakIterator::try_new_ustring(
                UBreakIteratorType::UBRK_WORD,
                &loc,
                ustr,
            )
            .expect("cannot create iterator"),
        }
    }
}

impl<'a> Iterator for ICUBreakingWord<'a> {
    type Item = (String, usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        // It is a port in Rust of Lucene algorithm
        let mut cont = true;
        let mut start = self.default_breaking_iterator.current();
        let mut end = self.default_breaking_iterator.next();
        while cont && end.is_some() {
            if end.is_some() && self.default_breaking_iterator.get_rule_status() == 0 {
                start = end.unwrap();
                end = self.default_breaking_iterator.next();
            }
            if let Some(index) = end {
                cont = !self
                    .text
                    .clone()
                    .take(index as usize)
                    .skip(start as usize)
                    .any(char::is_alphanumeric);
            }
        }

        match end {
            None => None,
            Some(index) => {
                let start_bytes = self.char_indices[start as usize].0;
                let end_bytes = self.char_indices[index as usize].0;
                let substring: String = self
                    .text
                    .clone()
                    .take(index as usize)
                    .skip(start as usize)
                    .collect();
                Some((substring, start_bytes as usize, end_bytes as usize))
            }
        }
    }
}

#[derive(Debug)]
pub struct ICUTokenizerTokenStream<'a> {
    breaking_word: ICUBreakingWord<'a>,
    token: Token,
}

impl<'a> ICUTokenizerTokenStream<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        ICUTokenizerTokenStream {
            breaking_word: ICUBreakingWord::from(text),
            token: Token::default(),
        }
    }
}

impl<'a> TokenStream for ICUTokenizerTokenStream<'a> {
    fn advance(&mut self) -> bool {
        let token = self.breaking_word.next();
        match token {
            None => false,
            Some(token) => {
                self.token.text.clear();
                self.token.position = self.token.position.wrapping_add(1);
                self.token.offset_from = token.1;
                self.token.offset_to = token.2;
                self.token.text.push_str(&token.0);
                true
            }
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
    /// Same tests as Lucene ICU tokenizer might be enough
    use super::*;
    use rstest::*;
    use tantivy::tokenizer::{Token, TokenStream};

    impl<'a> Iterator for ICUTokenizerTokenStream<'a> {
        type Item = Token;

        fn next(&mut self) -> Option<Self::Item> {
            if self.advance() {
                return Some(self.token().clone());
            }

            None
        }
    }

    #[rstest]
    fn test_huge_doc() {
        let mut huge_doc = " ".repeat(4094);
        huge_doc.push_str("testing 1234");
        let tokenizer = &mut ICUTokenizerTokenStream::new(huge_doc.as_str());
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![
            Token {
                offset_from: 4094,
                offset_to: 4101,
                position: 0,
                text: "testing".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 4102,
                offset_to: 4106,
                position: 1,
                text: "1234".to_string(),
                position_length: 1,
            },
        ];
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_czech() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("tvář je zkažená prachem, potem a krví; kdo se statečně snaží; kdo se mýlí, kdo znovu a znovu přichází zkrátka");
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![
            Token {
                offset_from: 0,
                offset_to: 6,
                position: 0,
                text: "tvář".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 7,
                offset_to: 9,
                position: 1,
                text: "je".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 10,
                offset_to: 19,
                position: 2,
                text: "zkažená".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 20,
                offset_to: 27,
                position: 3,
                text: "prachem".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 29,
                offset_to: 34,
                position: 4,
                text: "potem".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 35,
                offset_to: 36,
                position: 5,
                text: "a".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 37,
                offset_to: 42,
                position: 6,
                text: "krví".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 44,
                offset_to: 47,
                position: 7,
                text: "kdo".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 48,
                offset_to: 50,
                position: 8,
                text: "se".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 51,
                offset_to: 61,
                position: 9,
                text: "statečně".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 62,
                offset_to: 69,
                position: 10,
                text: "snaží".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 71,
                offset_to: 74,
                position: 11,
                text: "kdo".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 75,
                offset_to: 77,
                position: 12,
                text: "se".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 78,
                offset_to: 84,
                position: 13,
                text: "mýlí".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 86,
                offset_to: 89,
                position: 14,
                text: "kdo".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 90,
                offset_to: 95,
                position: 15,
                text: "znovu".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 96,
                offset_to: 97,
                position: 16,
                text: "a".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 98,
                offset_to: 103,
                position: 17,
                text: "znovu".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 104,
                offset_to: 115,
                position: 18,
                text: "přichází".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 116,
                offset_to: 124,
                position: 19,
                text: "zkrátka".to_string(),
                position_length: 1,
            },
        ];

        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_armenian() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("Վիքիպեդիայի 13 միլիոն հոդվածները (4,600` հայերեն վիքիպեդիայում) գրվել են կամավորների կողմից ու համարյա բոլոր հոդվածները կարող է խմբագրել ցանկաց մարդ ով կարող է բացել Վիքիպեդիայի կայքը։");
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![
            Token {
                offset_from: 0,
                offset_to: 22,
                position: 0,
                text: "Վիքիպեդիայի".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 23,
                offset_to: 25,
                position: 1,
                text: "13".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 26,
                offset_to: 38,
                position: 2,
                text: "միլիոն".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 39,
                offset_to: 59,
                position: 3,
                text: "հոդվածները".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 61,
                offset_to: 66,
                position: 4,
                text: "4,600".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 68,
                offset_to: 82,
                position: 5,
                text: "հայերեն".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 83,
                offset_to: 109,
                position: 6,
                text: "վիքիպեդիայում".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 111,
                offset_to: 121,
                position: 7,
                text: "գրվել".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 122,
                offset_to: 126,
                position: 8,
                text: "են".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 127,
                offset_to: 149,
                position: 9,
                text: "կամավորների".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 150,
                offset_to: 162,
                position: 10,
                text: "կողմից".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 163,
                offset_to: 167,
                position: 11,
                text: "ու".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 168,
                offset_to: 182,
                position: 12,
                text: "համարյա".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 183,
                offset_to: 193,
                position: 13,
                text: "բոլոր".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 194,
                offset_to: 214,
                position: 14,
                text: "հոդվածները".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 215,
                offset_to: 225,
                position: 15,
                text: "կարող".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 226,
                offset_to: 228,
                position: 16,
                text: "է".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 229,
                offset_to: 245,
                position: 17,
                text: "խմբագրել".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 246,
                offset_to: 258,
                position: 18,
                text: "ցանկաց".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 259,
                offset_to: 267,
                position: 19,
                text: "մարդ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 268,
                offset_to: 272,
                position: 20,
                text: "ով".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 273,
                offset_to: 283,
                position: 21,
                text: "կարող".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 284,
                offset_to: 286,
                position: 22,
                text: "է".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 287,
                offset_to: 297,
                position: 23,
                text: "բացել".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 298,
                offset_to: 320,
                position: 24,
                text: "Վիքիպեդիայի".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 321,
                offset_to: 331,
                position: 25,
                text: "կայքը".to_string(),
                position_length: 1,
            },
        ];
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_amharic() {
        let tokenizer = &mut ICUTokenizerTokenStream::new(
            "ዊኪፔድያ የባለ ብዙ ቋንቋ የተሟላ ትክክለኛና ነጻ መዝገበ ዕውቀት (ኢንሳይክሎፒዲያ) ነው። ማንኛውም",
        );
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![
            Token {
                offset_from: 0,
                offset_to: 15,
                position: 0,
                text: "ዊኪፔድያ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 16,
                offset_to: 25,
                position: 1,
                text: "የባለ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 26,
                offset_to: 32,
                position: 2,
                text: "ብዙ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 33,
                offset_to: 42,
                position: 3,
                text: "ቋንቋ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 43,
                offset_to: 55,
                position: 4,
                text: "የተሟላ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 56,
                offset_to: 74,
                position: 5,
                text: "ትክክለኛና".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 75,
                offset_to: 81,
                position: 6,
                text: "ነጻ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 82,
                offset_to: 94,
                position: 7,
                text: "መዝገበ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 95,
                offset_to: 107,
                position: 8,
                text: "ዕውቀት".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 109,
                offset_to: 136,
                position: 9,
                text: "ኢንሳይክሎፒዲያ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 138,
                offset_to: 144,
                position: 10,
                text: "ነው".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 148,
                offset_to: 163,
                position: 11,
                text: "ማንኛውም".to_string(),
                position_length: 1,
            },
        ];
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_arabic() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("الفيلم الوثائقي الأول عن ويكيبيديا يسمى \"الحقيقة بالأرقام: قصة ويكيبيديا\" (بالإنجليزية: Truth in Numbers: The Wikipedia Story)، سيتم إطلاقه في 2008.");
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![
            Token {
                offset_from: 0,
                offset_to: 12,
                position: 0,
                text: "الفيلم".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 13,
                offset_to: 29,
                position: 1,
                text: "الوثائقي".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 30,
                offset_to: 40,
                position: 2,
                text: "الأول".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 41,
                offset_to: 45,
                position: 3,
                text: "عن".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 46,
                offset_to: 64,
                position: 4,
                text: "ويكيبيديا".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 65,
                offset_to: 73,
                position: 5,
                text: "يسمى".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 75,
                offset_to: 89,
                position: 6,
                text: "الحقيقة".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 90,
                offset_to: 106,
                position: 7,
                text: "بالأرقام".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 108,
                offset_to: 114,
                position: 8,
                text: "قصة".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 115,
                offset_to: 133,
                position: 9,
                text: "ويكيبيديا".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 136,
                offset_to: 158,
                position: 10,
                text: "بالإنجليزية".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 160,
                offset_to: 165,
                position: 11,
                text: "Truth".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 166,
                offset_to: 168,
                position: 12,
                text: "in".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 169,
                offset_to: 176,
                position: 13,
                text: "Numbers".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 178,
                offset_to: 181,
                position: 14,
                text: "The".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 182,
                offset_to: 191,
                position: 15,
                text: "Wikipedia".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 192,
                offset_to: 197,
                position: 16,
                text: "Story".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 201,
                offset_to: 209,
                position: 17,
                text: "سيتم".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 210,
                offset_to: 222,
                position: 18,
                text: "إطلاقه".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 223,
                offset_to: 227,
                position: 19,
                text: "في".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 228,
                offset_to: 232,
                position: 20,
                text: "2008".to_string(),
                position_length: 1,
            },
        ];
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_aramaic() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("ܘܝܩܝܦܕܝܐ (ܐܢܓܠܝܐ: Wikipedia) ܗܘ ܐܝܢܣܩܠܘܦܕܝܐ ܚܐܪܬܐ ܕܐܢܛܪܢܛ ܒܠܫܢ̈ܐ ܣܓܝܐ̈ܐ܂ ܫܡܗ ܐܬܐ ܡܢ ܡ̈ܠܬܐ ܕ\"ܘܝܩܝ\" ܘ\"ܐܝܢܣܩܠܘܦܕܝܐ\"܀");
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![
            Token {
                offset_from: 0,
                offset_to: 16,
                position: 0,
                text: "ܘܝܩܝܦܕܝܐ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 18,
                offset_to: 30,
                position: 1,
                text: "ܐܢܓܠܝܐ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 32,
                offset_to: 41,
                position: 2,
                text: "Wikipedia".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 43,
                offset_to: 47,
                position: 3,
                text: "ܗܘ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 48,
                offset_to: 70,
                position: 4,
                text: "ܐܝܢܣܩܠܘܦܕܝܐ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 71,
                offset_to: 81,
                position: 5,
                text: "ܚܐܪܬܐ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 82,
                offset_to: 96,
                position: 6,
                text: "ܕܐܢܛܪܢܛ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 97,
                offset_to: 109,
                position: 7,
                text: "ܒܠܫܢ\u{308}ܐ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 110,
                offset_to: 122,
                position: 8,
                text: "ܣܓܝܐ\u{308}ܐ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 125,
                offset_to: 131,
                position: 9,
                text: "ܫܡܗ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 132,
                offset_to: 138,
                position: 10,
                text: "ܐܬܐ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 139,
                offset_to: 143,
                position: 11,
                text: "ܡܢ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 144,
                offset_to: 154,
                position: 12,
                text: "ܡ\u{308}ܠܬܐ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 155,
                offset_to: 157,
                position: 13,
                text: "ܕ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 158,
                offset_to: 166,
                position: 14,
                text: "ܘܝܩܝ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 168,
                offset_to: 170,
                position: 15,
                text: "ܘ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 171,
                offset_to: 193,
                position: 16,
                text: "ܐܝܢܣܩܠܘܦܕܝܐ".to_string(),
                position_length: 1,
            },
        ];
        assert_eq!(result, expected);
    }
}
