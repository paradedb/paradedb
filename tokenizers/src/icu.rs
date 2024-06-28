/*
 *
 * IMPORTANT NOTICE:
 * This file has been copied from tantivy-analysis-contrib, an open source project, and is subject to the terms
 * and conditions of the Apache License, Version 2.0.
 * Please review the full licensing details at <http://www.apache.org/licenses/LICENSE-2.0>.
 * By using this file, you agree to comply with the Apache v2.0 terms.
 *
 */

use rust_icu_ubrk::UBreakIterator;
use std::str::Chars;
use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

const DEFAULT_RULES: &str = include_str!("breaking_rules/Default.rbbi");

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
        ICUBreakingWord {
            text: text.chars(),
            default_breaking_iterator: UBreakIterator::try_new_rules(DEFAULT_RULES, text)
                .expect("Can't read default rules."),
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
                let substring: String = self
                    .text
                    .clone()
                    .take(index as usize)
                    .skip(start as usize)
                    .collect();
                Some((substring, start as usize, index as usize))
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
    fn test_armenian() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("Վիքիպեդիայի 13 միլիոն հոդվածները (4,600` հայերեն վիքիպեդիայում) գրվել են կամավորների կողմից ու համարյա բոլոր հոդվածները կարող է խմբագրել ցանկաց մարդ ով կարող է բացել Վիքիպեդիայի կայքը։");
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![
            Token {
                offset_from: 0,
                offset_to: 11,
                position: 0,
                text: "Վիքիպեդիայի".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 12,
                offset_to: 14,
                position: 1,
                text: "13".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 15,
                offset_to: 21,
                position: 2,
                text: "միլիոն".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 22,
                offset_to: 32,
                position: 3,
                text: "հոդվածները".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 34,
                offset_to: 39,
                position: 4,
                text: "4,600".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 41,
                offset_to: 48,
                position: 5,
                text: "հայերեն".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 49,
                offset_to: 62,
                position: 6,
                text: "վիքիպեդիայում".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 64,
                offset_to: 69,
                position: 7,
                text: "գրվել".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 70,
                offset_to: 72,
                position: 8,
                text: "են".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 73,
                offset_to: 84,
                position: 9,
                text: "կամավորների".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 85,
                offset_to: 91,
                position: 10,
                text: "կողմից".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 92,
                offset_to: 94,
                position: 11,
                text: "ու".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 95,
                offset_to: 102,
                position: 12,
                text: "համարյա".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 103,
                offset_to: 108,
                position: 13,
                text: "բոլոր".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 109,
                offset_to: 119,
                position: 14,
                text: "հոդվածները".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 120,
                offset_to: 125,
                position: 15,
                text: "կարող".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 126,
                offset_to: 127,
                position: 16,
                text: "է".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 128,
                offset_to: 136,
                position: 17,
                text: "խմբագրել".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 137,
                offset_to: 143,
                position: 18,
                text: "ցանկաց".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 144,
                offset_to: 148,
                position: 19,
                text: "մարդ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 149,
                offset_to: 151,
                position: 20,
                text: "ով".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 152,
                offset_to: 157,
                position: 21,
                text: "կարող".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 158,
                offset_to: 159,
                position: 22,
                text: "է".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 160,
                offset_to: 165,
                position: 23,
                text: "բացել".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 166,
                offset_to: 177,
                position: 24,
                text: "Վիքիպեդիայի".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 178,
                offset_to: 183,
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
                offset_to: 5,
                position: 0,
                text: "ዊኪፔድያ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 6,
                offset_to: 9,
                position: 1,
                text: "የባለ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 10,
                offset_to: 12,
                position: 2,
                text: "ብዙ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 13,
                offset_to: 16,
                position: 3,
                text: "ቋንቋ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 17,
                offset_to: 21,
                position: 4,
                text: "የተሟላ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 22,
                offset_to: 28,
                position: 5,
                text: "ትክክለኛና".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 29,
                offset_to: 31,
                position: 6,
                text: "ነጻ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 32,
                offset_to: 36,
                position: 7,
                text: "መዝገበ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 37,
                offset_to: 41,
                position: 8,
                text: "ዕውቀት".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 43,
                offset_to: 52,
                position: 9,
                text: "ኢንሳይክሎፒዲያ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 54,
                offset_to: 56,
                position: 10,
                text: "ነው".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 58,
                offset_to: 63,
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
                offset_to: 6,
                position: 0,
                text: "الفيلم".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 7,
                offset_to: 15,
                position: 1,
                text: "الوثائقي".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 16,
                offset_to: 21,
                position: 2,
                text: "الأول".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 22,
                offset_to: 24,
                position: 3,
                text: "عن".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 25,
                offset_to: 34,
                position: 4,
                text: "ويكيبيديا".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 35,
                offset_to: 39,
                position: 5,
                text: "يسمى".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 41,
                offset_to: 48,
                position: 6,
                text: "الحقيقة".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 49,
                offset_to: 57,
                position: 7,
                text: "بالأرقام".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 59,
                offset_to: 62,
                position: 8,
                text: "قصة".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 63,
                offset_to: 72,
                position: 9,
                text: "ويكيبيديا".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 75,
                offset_to: 86,
                position: 10,
                text: "بالإنجليزية".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 88,
                offset_to: 93,
                position: 11,
                text: "Truth".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 94,
                offset_to: 96,
                position: 12,
                text: "in".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 97,
                offset_to: 104,
                position: 13,
                text: "Numbers".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 106,
                offset_to: 109,
                position: 14,
                text: "The".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 110,
                offset_to: 119,
                position: 15,
                text: "Wikipedia".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 120,
                offset_to: 125,
                position: 16,
                text: "Story".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 128,
                offset_to: 132,
                position: 17,
                text: "سيتم".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 133,
                offset_to: 139,
                position: 18,
                text: "إطلاقه".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 140,
                offset_to: 142,
                position: 19,
                text: "في".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 143,
                offset_to: 147,
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
                offset_to: 8,
                position: 0,
                text: "ܘܝܩܝܦܕܝܐ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 10,
                offset_to: 16,
                position: 1,
                text: "ܐܢܓܠܝܐ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 18,
                offset_to: 27,
                position: 2,
                text: "Wikipedia".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 29,
                offset_to: 31,
                position: 3,
                text: "ܗܘ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 32,
                offset_to: 43,
                position: 4,
                text: "ܐܝܢܣܩܠܘܦܕܝܐ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 44,
                offset_to: 49,
                position: 5,
                text: "ܚܐܪܬܐ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 50,
                offset_to: 57,
                position: 6,
                text: "ܕܐܢܛܪܢܛ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 58,
                offset_to: 64,
                position: 7,
                text: "ܒܠܫܢ̈ܐ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 65,
                offset_to: 71,
                position: 8,
                text: "ܣܓܝܐ̈ܐ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 73,
                offset_to: 76,
                position: 9,
                text: "ܫܡܗ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 77,
                offset_to: 80,
                position: 10,
                text: "ܐܬܐ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 81,
                offset_to: 83,
                position: 11,
                text: "ܡܢ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 84,
                offset_to: 89,
                position: 12,
                text: "ܡ̈ܠܬܐ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 90,
                offset_to: 91,
                position: 13,
                text: "ܕ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 92,
                offset_to: 96,
                position: 14,
                text: "ܘܝܩܝ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 98,
                offset_to: 99,
                position: 15,
                text: "ܘ".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 100,
                offset_to: 111,
                position: 16,
                text: "ܐܝܢܣܩܠܘܦܕܝܐ".to_string(),
                position_length: 1,
            },
        ];
        assert_eq!(result, expected);
    }
}
