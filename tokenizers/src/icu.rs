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
    text: &'a str,
    utf16_indices_to_byte_offsets: Vec<usize>,
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
        let ustr = &UChar::try_from(text).expect("text should be an encodable character");

        // Build mapping from UTF-16 code unit indices to byte offsets
        let utf16_units: Vec<u16> = text.encode_utf16().collect();
        let mut utf16_indices_to_byte_offsets = Vec::with_capacity(utf16_units.len() + 1);
        //        let mut utf16_idx = 0;
        let mut byte_offset = 0;
        let bytes = text.as_bytes();

        while byte_offset < bytes.len() {
            let ch = text[byte_offset..].chars().next().unwrap();
            let ch_utf16_len = ch.encode_utf16(&mut [0; 2]).len();
            let ch_utf8_len = ch.len_utf8();

            for _ in 0..ch_utf16_len {
                utf16_indices_to_byte_offsets.push(byte_offset);
                //              utf16_idx += 1;
            }
            byte_offset += ch_utf8_len;
        }
        // Append the final byte offset
        utf16_indices_to_byte_offsets.push(byte_offset);

        ICUBreakingWord {
            text,
            utf16_indices_to_byte_offsets,
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
        let mut start = self.default_breaking_iterator.current() as usize;
        'find_end: loop {
            let mut end = self.default_breaking_iterator.next()?;

            // the inner loop locates the next token.  if the next token begins because of
            // a non-zero break rule then we already have our token between [start..end]
            //
            // if it doesn't, then we move forward in the breaking iterator and move the `start`
            // position to whatever the current `end` position is
            'next_token: loop {
                if self.default_breaking_iterator.get_rule_status() == 0 {
                    // the token boundary is unspecified, so move to the next token
                    start = end as usize;
                    end = self.default_breaking_iterator.next()?;
                    continue 'next_token;
                }

                // translate from utf16 back to utf8 bytes through our translation table
                let start_byte = self.utf16_indices_to_byte_offsets[start];
                let end_byte = self.utf16_indices_to_byte_offsets[end as usize];
                let substring = &self.text[start_byte..end_byte];

                if !substring.chars().any(char::is_alphanumeric) {
                    // the string doesn't contain any alphanumerics, so keep extending it
                    // until it does
                    continue 'find_end;
                }

                return Some((substring.into(), start_byte, end_byte));
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

    #[rstest]
    fn test_emoji_in_text() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("one🥜two");
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![
            Token {
                offset_from: 0,
                offset_to: 3,
                position: 0,
                text: "one".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 7,
                offset_to: 10,
                position: 1,
                text: "two".to_string(),
                position_length: 1,
            },
        ];
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_emoji_between_words() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("one🥜two three");
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![
            Token {
                offset_from: 0,
                offset_to: 3,
                position: 0,
                text: "one".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 7,
                offset_to: 10,
                position: 1,
                text: "two".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 11,
                offset_to: 16,
                position: 2,
                text: "three".to_string(),
                position_length: 1,
            },
        ];
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_emoji_with_punctuation() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("one🥜two three.");
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![
            Token {
                offset_from: 0,
                offset_to: 3,
                position: 0,
                text: "one".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 7,
                offset_to: 10,
                position: 1,
                text: "two".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 11,
                offset_to: 16,
                position: 2,
                text: "three".to_string(),
                position_length: 1,
            },
        ];
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_space_before_emoji() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("one🥜 two three.");
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![
            Token {
                offset_from: 0,
                offset_to: 3,
                position: 0,
                text: "one".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 8,
                offset_to: 11,
                position: 1,
                text: "two".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 12,
                offset_to: 17,
                position: 2,
                text: "three".to_string(),
                position_length: 1,
            },
        ];
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_single_character_after_emoji() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("one🥜t");
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![
            Token {
                offset_from: 0,
                offset_to: 3,
                position: 0,
                text: "one".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 7,
                offset_to: 8,
                position: 1,
                text: "t".to_string(),
                position_length: 1,
            },
        ];
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_at_symbol_alone() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("@");
        let result: Vec<Token> = tokenizer.collect();
        let expected: Vec<Token> = Vec::new(); // Expect no tokens as '@' is not alphanumeric
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_at_symbol_in_text() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("test@");
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![Token {
            offset_from: 0,
            offset_to: 4,
            position: 0,
            text: "test".to_string(),
            position_length: 1,
        }];
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_at_symbol_prefix() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("@test");
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![Token {
            offset_from: 1,
            offset_to: 5,
            position: 0,
            text: "test".to_string(),
            position_length: 1,
        }];
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_chinese_characters_with_emoji() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("📞统");
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![Token {
            offset_from: 4,
            offset_to: 7,
            position: 0,
            text: "统".to_string(),
            position_length: 1,
        }];
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_mixed_language_text() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("hello 你好 🥜 world 世界");
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![
            Token {
                offset_from: 0,
                offset_to: 5,
                position: 0,
                text: "hello".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 6,
                offset_to: 12,
                position: 1,
                text: "你好".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 18,
                offset_to: 23,
                position: 2,
                text: "world".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 24,
                offset_to: 30,
                position: 3,
                text: "世界".to_string(),
                position_length: 1,
            },
        ];
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_emojis_only() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("🥜📞🚀");
        let result: Vec<Token> = tokenizer.collect();
        let expected: Vec<Token> = Vec::new(); // Emojis are not alphanumeric
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_text_with_emojis_and_symbols() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("Call me at 📞123-456-7890!");
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![
            Token {
                offset_from: 0,
                offset_to: 4,
                position: 0,
                text: "Call".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 5,
                offset_to: 7,
                position: 1,
                text: "me".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 8,
                offset_to: 10,
                position: 2,
                text: "at".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 15,
                offset_to: 18,
                position: 3,
                text: "123".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 19,
                offset_to: 22,
                position: 4,
                text: "456".to_string(),
                position_length: 1,
            },
            Token {
                offset_from: 23,
                offset_to: 27,
                position: 5,
                text: "7890".to_string(),
                position_length: 1,
            },
        ];
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_simple_text() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("this is a test");
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![
            Token {
                offset_from: 0,
                offset_to: 4,
                position: 0,
                text: "this".into(),
                position_length: 1,
            },
            Token {
                offset_from: 5,
                offset_to: 7,
                position: 1,
                text: "is".into(),
                position_length: 1,
            },
            Token {
                offset_from: 8,
                offset_to: 9,
                position: 2,
                text: "a".into(),
                position_length: 1,
            },
            Token {
                offset_from: 10,
                offset_to: 14,
                position: 3,
                text: "test".into(),
                position_length: 1,
            },
        ];
        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_complex_email() {
        let tokenizer = &mut ICUTokenizerTokenStream::new("'From: tas@pegasus.com (Len Howard) Subject: Re: Pregnancy without sex? In article <10030@blue.cis.pitt.edu> kxgst1+@pitt.edu (Kenneth Gilbert) writes: >In article <stephen.735806195@mont> stephen@mont.cs.missouri.edu (Stephen Montgom Smith) writes: >:When I was a school boy, my biology teacher told us of an incident >:in which a couple were very passionate without actually having >:sexual intercourse. Somehow the girl became pregnant as sperm >:cells made their way to her through the clothes via perspiration. >:Was my biology teacher misinforming us, or do such incidents actually >:occur? > >Sounds to me like someone was pulling your leg. There is only one way for >pregnancy to occur: intercourse. These days however there is also >artificial insemination and implantation techniques, but we''re speaking of >''natural'' acts here. It is possible for pregnancy to occur if semen is >deposited just outside of the vagina (i.e. coitus interruptus), but that''s >about as far as you can get. Through clothes -- no way. Better go talk >to your biology teacher. >= Kenneth Gilbert __|__ University of Pittsburgh = Well, now, Doc, I sure would not want to bet my life on those little critters not being able to get thru one layer of sweat-soaked cotton on their way to do their programmed task. Infrequent, yes, unlikely, yes, but impossible? I learned a long time ago never to say never in medicine <g> Len Howard MD, FACOG'");
        let result: Vec<Token> = tokenizer.collect();
        let expected = vec![
            Token {
                offset_from: 1,
                offset_to: 5,
                position: 0,
                text: "From".into(),
                position_length: 1,
            },
            Token {
                offset_from: 7,
                offset_to: 10,
                position: 1,
                text: "tas".into(),
                position_length: 1,
            },
            Token {
                offset_from: 11,
                offset_to: 18,
                position: 2,
                text: "pegasus".into(),
                position_length: 1,
            },
            Token {
                offset_from: 19,
                offset_to: 22,
                position: 3,
                text: "com".into(),
                position_length: 1,
            },
            Token {
                offset_from: 24,
                offset_to: 27,
                position: 4,
                text: "Len".into(),
                position_length: 1,
            },
            Token {
                offset_from: 28,
                offset_to: 34,
                position: 5,
                text: "Howard".into(),
                position_length: 1,
            },
            Token {
                offset_from: 36,
                offset_to: 43,
                position: 6,
                text: "Subject".into(),
                position_length: 1,
            },
            Token {
                offset_from: 45,
                offset_to: 47,
                position: 7,
                text: "Re".into(),
                position_length: 1,
            },
            Token {
                offset_from: 49,
                offset_to: 58,
                position: 8,
                text: "Pregnancy".into(),
                position_length: 1,
            },
            Token {
                offset_from: 59,
                offset_to: 66,
                position: 9,
                text: "without".into(),
                position_length: 1,
            },
            Token {
                offset_from: 67,
                offset_to: 70,
                position: 10,
                text: "sex".into(),
                position_length: 1,
            },
            Token {
                offset_from: 72,
                offset_to: 74,
                position: 11,
                text: "In".into(),
                position_length: 1,
            },
            Token {
                offset_from: 75,
                offset_to: 82,
                position: 12,
                text: "article".into(),
                position_length: 1,
            },
            Token {
                offset_from: 84,
                offset_to: 89,
                position: 13,
                text: "10030".into(),
                position_length: 1,
            },
            Token {
                offset_from: 90,
                offset_to: 94,
                position: 14,
                text: "blue".into(),
                position_length: 1,
            },
            Token {
                offset_from: 95,
                offset_to: 98,
                position: 15,
                text: "cis".into(),
                position_length: 1,
            },
            Token {
                offset_from: 99,
                offset_to: 103,
                position: 16,
                text: "pitt".into(),
                position_length: 1,
            },
            Token {
                offset_from: 104,
                offset_to: 107,
                position: 17,
                text: "edu".into(),
                position_length: 1,
            },
            Token {
                offset_from: 109,
                offset_to: 115,
                position: 18,
                text: "kxgst1".into(),
                position_length: 1,
            },
            Token {
                offset_from: 117,
                offset_to: 121,
                position: 19,
                text: "pitt".into(),
                position_length: 1,
            },
            Token {
                offset_from: 122,
                offset_to: 125,
                position: 20,
                text: "edu".into(),
                position_length: 1,
            },
            Token {
                offset_from: 127,
                offset_to: 134,
                position: 21,
                text: "Kenneth".into(),
                position_length: 1,
            },
            Token {
                offset_from: 135,
                offset_to: 142,
                position: 22,
                text: "Gilbert".into(),
                position_length: 1,
            },
            Token {
                offset_from: 144,
                offset_to: 150,
                position: 23,
                text: "writes".into(),
                position_length: 1,
            },
            Token {
                offset_from: 153,
                offset_to: 155,
                position: 24,
                text: "In".into(),
                position_length: 1,
            },
            Token {
                offset_from: 156,
                offset_to: 163,
                position: 25,
                text: "article".into(),
                position_length: 1,
            },
            Token {
                offset_from: 165,
                offset_to: 172,
                position: 26,
                text: "stephen".into(),
                position_length: 1,
            },
            Token {
                offset_from: 173,
                offset_to: 182,
                position: 27,
                text: "735806195".into(),
                position_length: 1,
            },
            Token {
                offset_from: 183,
                offset_to: 187,
                position: 28,
                text: "mont".into(),
                position_length: 1,
            },
            Token {
                offset_from: 189,
                offset_to: 196,
                position: 29,
                text: "stephen".into(),
                position_length: 1,
            },
            Token {
                offset_from: 197,
                offset_to: 201,
                position: 30,
                text: "mont".into(),
                position_length: 1,
            },
            Token {
                offset_from: 202,
                offset_to: 204,
                position: 31,
                text: "cs".into(),
                position_length: 1,
            },
            Token {
                offset_from: 205,
                offset_to: 213,
                position: 32,
                text: "missouri".into(),
                position_length: 1,
            },
            Token {
                offset_from: 214,
                offset_to: 217,
                position: 33,
                text: "edu".into(),
                position_length: 1,
            },
            Token {
                offset_from: 219,
                offset_to: 226,
                position: 34,
                text: "Stephen".into(),
                position_length: 1,
            },
            Token {
                offset_from: 227,
                offset_to: 234,
                position: 35,
                text: "Montgom".into(),
                position_length: 1,
            },
            Token {
                offset_from: 235,
                offset_to: 240,
                position: 36,
                text: "Smith".into(),
                position_length: 1,
            },
            Token {
                offset_from: 242,
                offset_to: 248,
                position: 37,
                text: "writes".into(),
                position_length: 1,
            },
            Token {
                offset_from: 252,
                offset_to: 256,
                position: 38,
                text: "When".into(),
                position_length: 1,
            },
            Token {
                offset_from: 257,
                offset_to: 258,
                position: 39,
                text: "I".into(),
                position_length: 1,
            },
            Token {
                offset_from: 259,
                offset_to: 262,
                position: 40,
                text: "was".into(),
                position_length: 1,
            },
            Token {
                offset_from: 263,
                offset_to: 264,
                position: 41,
                text: "a".into(),
                position_length: 1,
            },
            Token {
                offset_from: 265,
                offset_to: 271,
                position: 42,
                text: "school".into(),
                position_length: 1,
            },
            Token {
                offset_from: 272,
                offset_to: 275,
                position: 43,
                text: "boy".into(),
                position_length: 1,
            },
            Token {
                offset_from: 277,
                offset_to: 279,
                position: 44,
                text: "my".into(),
                position_length: 1,
            },
            Token {
                offset_from: 280,
                offset_to: 287,
                position: 45,
                text: "biology".into(),
                position_length: 1,
            },
            Token {
                offset_from: 288,
                offset_to: 295,
                position: 46,
                text: "teacher".into(),
                position_length: 1,
            },
            Token {
                offset_from: 296,
                offset_to: 300,
                position: 47,
                text: "told".into(),
                position_length: 1,
            },
            Token {
                offset_from: 301,
                offset_to: 303,
                position: 48,
                text: "us".into(),
                position_length: 1,
            },
            Token {
                offset_from: 304,
                offset_to: 306,
                position: 49,
                text: "of".into(),
                position_length: 1,
            },
            Token {
                offset_from: 307,
                offset_to: 309,
                position: 50,
                text: "an".into(),
                position_length: 1,
            },
            Token {
                offset_from: 310,
                offset_to: 318,
                position: 51,
                text: "incident".into(),
                position_length: 1,
            },
            Token {
                offset_from: 321,
                offset_to: 323,
                position: 52,
                text: "in".into(),
                position_length: 1,
            },
            Token {
                offset_from: 324,
                offset_to: 329,
                position: 53,
                text: "which".into(),
                position_length: 1,
            },
            Token {
                offset_from: 330,
                offset_to: 331,
                position: 54,
                text: "a".into(),
                position_length: 1,
            },
            Token {
                offset_from: 332,
                offset_to: 338,
                position: 55,
                text: "couple".into(),
                position_length: 1,
            },
            Token {
                offset_from: 339,
                offset_to: 343,
                position: 56,
                text: "were".into(),
                position_length: 1,
            },
            Token {
                offset_from: 344,
                offset_to: 348,
                position: 57,
                text: "very".into(),
                position_length: 1,
            },
            Token {
                offset_from: 349,
                offset_to: 359,
                position: 58,
                text: "passionate".into(),
                position_length: 1,
            },
            Token {
                offset_from: 360,
                offset_to: 367,
                position: 59,
                text: "without".into(),
                position_length: 1,
            },
            Token {
                offset_from: 368,
                offset_to: 376,
                position: 60,
                text: "actually".into(),
                position_length: 1,
            },
            Token {
                offset_from: 377,
                offset_to: 383,
                position: 61,
                text: "having".into(),
                position_length: 1,
            },
            Token {
                offset_from: 386,
                offset_to: 392,
                position: 62,
                text: "sexual".into(),
                position_length: 1,
            },
            Token {
                offset_from: 393,
                offset_to: 404,
                position: 63,
                text: "intercourse".into(),
                position_length: 1,
            },
            Token {
                offset_from: 406,
                offset_to: 413,
                position: 64,
                text: "Somehow".into(),
                position_length: 1,
            },
            Token {
                offset_from: 414,
                offset_to: 417,
                position: 65,
                text: "the".into(),
                position_length: 1,
            },
            Token {
                offset_from: 418,
                offset_to: 422,
                position: 66,
                text: "girl".into(),
                position_length: 1,
            },
            Token {
                offset_from: 423,
                offset_to: 429,
                position: 67,
                text: "became".into(),
                position_length: 1,
            },
            Token {
                offset_from: 430,
                offset_to: 438,
                position: 68,
                text: "pregnant".into(),
                position_length: 1,
            },
            Token {
                offset_from: 439,
                offset_to: 441,
                position: 69,
                text: "as".into(),
                position_length: 1,
            },
            Token {
                offset_from: 442,
                offset_to: 447,
                position: 70,
                text: "sperm".into(),
                position_length: 1,
            },
            Token {
                offset_from: 450,
                offset_to: 455,
                position: 71,
                text: "cells".into(),
                position_length: 1,
            },
            Token {
                offset_from: 456,
                offset_to: 460,
                position: 72,
                text: "made".into(),
                position_length: 1,
            },
            Token {
                offset_from: 461,
                offset_to: 466,
                position: 73,
                text: "their".into(),
                position_length: 1,
            },
            Token {
                offset_from: 467,
                offset_to: 470,
                position: 74,
                text: "way".into(),
                position_length: 1,
            },
            Token {
                offset_from: 471,
                offset_to: 473,
                position: 75,
                text: "to".into(),
                position_length: 1,
            },
            Token {
                offset_from: 474,
                offset_to: 477,
                position: 76,
                text: "her".into(),
                position_length: 1,
            },
            Token {
                offset_from: 478,
                offset_to: 485,
                position: 77,
                text: "through".into(),
                position_length: 1,
            },
            Token {
                offset_from: 486,
                offset_to: 489,
                position: 78,
                text: "the".into(),
                position_length: 1,
            },
            Token {
                offset_from: 490,
                offset_to: 497,
                position: 79,
                text: "clothes".into(),
                position_length: 1,
            },
            Token {
                offset_from: 498,
                offset_to: 501,
                position: 80,
                text: "via".into(),
                position_length: 1,
            },
            Token {
                offset_from: 502,
                offset_to: 514,
                position: 81,
                text: "perspiration".into(),
                position_length: 1,
            },
            Token {
                offset_from: 518,
                offset_to: 521,
                position: 82,
                text: "Was".into(),
                position_length: 1,
            },
            Token {
                offset_from: 522,
                offset_to: 524,
                position: 83,
                text: "my".into(),
                position_length: 1,
            },
            Token {
                offset_from: 525,
                offset_to: 532,
                position: 84,
                text: "biology".into(),
                position_length: 1,
            },
            Token {
                offset_from: 533,
                offset_to: 540,
                position: 85,
                text: "teacher".into(),
                position_length: 1,
            },
            Token {
                offset_from: 541,
                offset_to: 553,
                position: 86,
                text: "misinforming".into(),
                position_length: 1,
            },
            Token {
                offset_from: 554,
                offset_to: 556,
                position: 87,
                text: "us".into(),
                position_length: 1,
            },
            Token {
                offset_from: 558,
                offset_to: 560,
                position: 88,
                text: "or".into(),
                position_length: 1,
            },
            Token {
                offset_from: 561,
                offset_to: 563,
                position: 89,
                text: "do".into(),
                position_length: 1,
            },
            Token {
                offset_from: 564,
                offset_to: 568,
                position: 90,
                text: "such".into(),
                position_length: 1,
            },
            Token {
                offset_from: 569,
                offset_to: 578,
                position: 91,
                text: "incidents".into(),
                position_length: 1,
            },
            Token {
                offset_from: 579,
                offset_to: 587,
                position: 92,
                text: "actually".into(),
                position_length: 1,
            },
            Token {
                offset_from: 590,
                offset_to: 595,
                position: 93,
                text: "occur".into(),
                position_length: 1,
            },
            Token {
                offset_from: 600,
                offset_to: 606,
                position: 94,
                text: "Sounds".into(),
                position_length: 1,
            },
            Token {
                offset_from: 607,
                offset_to: 609,
                position: 95,
                text: "to".into(),
                position_length: 1,
            },
            Token {
                offset_from: 610,
                offset_to: 612,
                position: 96,
                text: "me".into(),
                position_length: 1,
            },
            Token {
                offset_from: 613,
                offset_to: 617,
                position: 97,
                text: "like".into(),
                position_length: 1,
            },
            Token {
                offset_from: 618,
                offset_to: 625,
                position: 98,
                text: "someone".into(),
                position_length: 1,
            },
            Token {
                offset_from: 626,
                offset_to: 629,
                position: 99,
                text: "was".into(),
                position_length: 1,
            },
            Token {
                offset_from: 630,
                offset_to: 637,
                position: 100,
                text: "pulling".into(),
                position_length: 1,
            },
            Token {
                offset_from: 638,
                offset_to: 642,
                position: 101,
                text: "your".into(),
                position_length: 1,
            },
            Token {
                offset_from: 643,
                offset_to: 646,
                position: 102,
                text: "leg".into(),
                position_length: 1,
            },
            Token {
                offset_from: 648,
                offset_to: 653,
                position: 103,
                text: "There".into(),
                position_length: 1,
            },
            Token {
                offset_from: 654,
                offset_to: 656,
                position: 104,
                text: "is".into(),
                position_length: 1,
            },
            Token {
                offset_from: 657,
                offset_to: 661,
                position: 105,
                text: "only".into(),
                position_length: 1,
            },
            Token {
                offset_from: 662,
                offset_to: 665,
                position: 106,
                text: "one".into(),
                position_length: 1,
            },
            Token {
                offset_from: 666,
                offset_to: 669,
                position: 107,
                text: "way".into(),
                position_length: 1,
            },
            Token {
                offset_from: 670,
                offset_to: 673,
                position: 108,
                text: "for".into(),
                position_length: 1,
            },
            Token {
                offset_from: 675,
                offset_to: 684,
                position: 109,
                text: "pregnancy".into(),
                position_length: 1,
            },
            Token {
                offset_from: 685,
                offset_to: 687,
                position: 110,
                text: "to".into(),
                position_length: 1,
            },
            Token {
                offset_from: 688,
                offset_to: 693,
                position: 111,
                text: "occur".into(),
                position_length: 1,
            },
            Token {
                offset_from: 695,
                offset_to: 706,
                position: 112,
                text: "intercourse".into(),
                position_length: 1,
            },
            Token {
                offset_from: 708,
                offset_to: 713,
                position: 113,
                text: "These".into(),
                position_length: 1,
            },
            Token {
                offset_from: 714,
                offset_to: 718,
                position: 114,
                text: "days".into(),
                position_length: 1,
            },
            Token {
                offset_from: 719,
                offset_to: 726,
                position: 115,
                text: "however".into(),
                position_length: 1,
            },
            Token {
                offset_from: 727,
                offset_to: 732,
                position: 116,
                text: "there".into(),
                position_length: 1,
            },
            Token {
                offset_from: 733,
                offset_to: 735,
                position: 117,
                text: "is".into(),
                position_length: 1,
            },
            Token {
                offset_from: 736,
                offset_to: 740,
                position: 118,
                text: "also".into(),
                position_length: 1,
            },
            Token {
                offset_from: 742,
                offset_to: 752,
                position: 119,
                text: "artificial".into(),
                position_length: 1,
            },
            Token {
                offset_from: 753,
                offset_to: 765,
                position: 120,
                text: "insemination".into(),
                position_length: 1,
            },
            Token {
                offset_from: 766,
                offset_to: 769,
                position: 121,
                text: "and".into(),
                position_length: 1,
            },
            Token {
                offset_from: 770,
                offset_to: 782,
                position: 122,
                text: "implantation".into(),
                position_length: 1,
            },
            Token {
                offset_from: 783,
                offset_to: 793,
                position: 123,
                text: "techniques".into(),
                position_length: 1,
            },
            Token {
                offset_from: 795,
                offset_to: 798,
                position: 124,
                text: "but".into(),
                position_length: 1,
            },
            Token {
                offset_from: 799,
                offset_to: 801,
                position: 125,
                text: "we".into(),
                position_length: 1,
            },
            Token {
                offset_from: 803,
                offset_to: 805,
                position: 126,
                text: "re".into(),
                position_length: 1,
            },
            Token {
                offset_from: 806,
                offset_to: 814,
                position: 127,
                text: "speaking".into(),
                position_length: 1,
            },
            Token {
                offset_from: 815,
                offset_to: 817,
                position: 128,
                text: "of".into(),
                position_length: 1,
            },
            Token {
                offset_from: 821,
                offset_to: 828,
                position: 129,
                text: "natural".into(),
                position_length: 1,
            },
            Token {
                offset_from: 831,
                offset_to: 835,
                position: 130,
                text: "acts".into(),
                position_length: 1,
            },
            Token {
                offset_from: 836,
                offset_to: 840,
                position: 131,
                text: "here".into(),
                position_length: 1,
            },
            Token {
                offset_from: 842,
                offset_to: 844,
                position: 132,
                text: "It".into(),
                position_length: 1,
            },
            Token {
                offset_from: 845,
                offset_to: 847,
                position: 133,
                text: "is".into(),
                position_length: 1,
            },
            Token {
                offset_from: 848,
                offset_to: 856,
                position: 134,
                text: "possible".into(),
                position_length: 1,
            },
            Token {
                offset_from: 857,
                offset_to: 860,
                position: 135,
                text: "for".into(),
                position_length: 1,
            },
            Token {
                offset_from: 861,
                offset_to: 870,
                position: 136,
                text: "pregnancy".into(),
                position_length: 1,
            },
            Token {
                offset_from: 871,
                offset_to: 873,
                position: 137,
                text: "to".into(),
                position_length: 1,
            },
            Token {
                offset_from: 874,
                offset_to: 879,
                position: 138,
                text: "occur".into(),
                position_length: 1,
            },
            Token {
                offset_from: 880,
                offset_to: 882,
                position: 139,
                text: "if".into(),
                position_length: 1,
            },
            Token {
                offset_from: 883,
                offset_to: 888,
                position: 140,
                text: "semen".into(),
                position_length: 1,
            },
            Token {
                offset_from: 889,
                offset_to: 891,
                position: 141,
                text: "is".into(),
                position_length: 1,
            },
            Token {
                offset_from: 893,
                offset_to: 902,
                position: 142,
                text: "deposited".into(),
                position_length: 1,
            },
            Token {
                offset_from: 903,
                offset_to: 907,
                position: 143,
                text: "just".into(),
                position_length: 1,
            },
            Token {
                offset_from: 908,
                offset_to: 915,
                position: 144,
                text: "outside".into(),
                position_length: 1,
            },
            Token {
                offset_from: 916,
                offset_to: 918,
                position: 145,
                text: "of".into(),
                position_length: 1,
            },
            Token {
                offset_from: 919,
                offset_to: 922,
                position: 146,
                text: "the".into(),
                position_length: 1,
            },
            Token {
                offset_from: 923,
                offset_to: 929,
                position: 147,
                text: "vagina".into(),
                position_length: 1,
            },
            Token {
                offset_from: 931,
                offset_to: 932,
                position: 148,
                text: "i".into(),
                position_length: 1,
            },
            Token {
                offset_from: 933,
                offset_to: 934,
                position: 149,
                text: "e".into(),
                position_length: 1,
            },
            Token {
                offset_from: 936,
                offset_to: 942,
                position: 150,
                text: "coitus".into(),
                position_length: 1,
            },
            Token {
                offset_from: 943,
                offset_to: 954,
                position: 151,
                text: "interruptus".into(),
                position_length: 1,
            },
            Token {
                offset_from: 957,
                offset_to: 960,
                position: 152,
                text: "but".into(),
                position_length: 1,
            },
            Token {
                offset_from: 961,
                offset_to: 965,
                position: 153,
                text: "that".into(),
                position_length: 1,
            },
            Token {
                offset_from: 967,
                offset_to: 968,
                position: 154,
                text: "s".into(),
                position_length: 1,
            },
            Token {
                offset_from: 970,
                offset_to: 975,
                position: 155,
                text: "about".into(),
                position_length: 1,
            },
            Token {
                offset_from: 976,
                offset_to: 978,
                position: 156,
                text: "as".into(),
                position_length: 1,
            },
            Token {
                offset_from: 979,
                offset_to: 982,
                position: 157,
                text: "far".into(),
                position_length: 1,
            },
            Token {
                offset_from: 983,
                offset_to: 985,
                position: 158,
                text: "as".into(),
                position_length: 1,
            },
            Token {
                offset_from: 986,
                offset_to: 989,
                position: 159,
                text: "you".into(),
                position_length: 1,
            },
            Token {
                offset_from: 990,
                offset_to: 993,
                position: 160,
                text: "can".into(),
                position_length: 1,
            },
            Token {
                offset_from: 994,
                offset_to: 997,
                position: 161,
                text: "get".into(),
                position_length: 1,
            },
            Token {
                offset_from: 999,
                offset_to: 1006,
                position: 162,
                text: "Through".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1007,
                offset_to: 1014,
                position: 163,
                text: "clothes".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1018,
                offset_to: 1020,
                position: 164,
                text: "no".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1021,
                offset_to: 1024,
                position: 165,
                text: "way".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1026,
                offset_to: 1032,
                position: 166,
                text: "Better".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1033,
                offset_to: 1035,
                position: 167,
                text: "go".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1036,
                offset_to: 1040,
                position: 168,
                text: "talk".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1042,
                offset_to: 1044,
                position: 169,
                text: "to".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1045,
                offset_to: 1049,
                position: 170,
                text: "your".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1050,
                offset_to: 1057,
                position: 171,
                text: "biology".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1058,
                offset_to: 1065,
                position: 172,
                text: "teacher".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1070,
                offset_to: 1077,
                position: 173,
                text: "Kenneth".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1078,
                offset_to: 1085,
                position: 174,
                text: "Gilbert".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1092,
                offset_to: 1102,
                position: 175,
                text: "University".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1103,
                offset_to: 1105,
                position: 176,
                text: "of".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1106,
                offset_to: 1116,
                position: 177,
                text: "Pittsburgh".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1119,
                offset_to: 1123,
                position: 178,
                text: "Well".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1125,
                offset_to: 1128,
                position: 179,
                text: "now".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1130,
                offset_to: 1133,
                position: 180,
                text: "Doc".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1135,
                offset_to: 1136,
                position: 181,
                text: "I".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1137,
                offset_to: 1141,
                position: 182,
                text: "sure".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1142,
                offset_to: 1147,
                position: 183,
                text: "would".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1148,
                offset_to: 1151,
                position: 184,
                text: "not".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1152,
                offset_to: 1156,
                position: 185,
                text: "want".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1157,
                offset_to: 1159,
                position: 186,
                text: "to".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1160,
                offset_to: 1163,
                position: 187,
                text: "bet".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1164,
                offset_to: 1166,
                position: 188,
                text: "my".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1167,
                offset_to: 1171,
                position: 189,
                text: "life".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1172,
                offset_to: 1174,
                position: 190,
                text: "on".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1175,
                offset_to: 1180,
                position: 191,
                text: "those".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1181,
                offset_to: 1187,
                position: 192,
                text: "little".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1188,
                offset_to: 1196,
                position: 193,
                text: "critters".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1197,
                offset_to: 1200,
                position: 194,
                text: "not".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1201,
                offset_to: 1206,
                position: 195,
                text: "being".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1207,
                offset_to: 1211,
                position: 196,
                text: "able".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1212,
                offset_to: 1214,
                position: 197,
                text: "to".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1215,
                offset_to: 1218,
                position: 198,
                text: "get".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1219,
                offset_to: 1223,
                position: 199,
                text: "thru".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1224,
                offset_to: 1227,
                position: 200,
                text: "one".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1228,
                offset_to: 1233,
                position: 201,
                text: "layer".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1234,
                offset_to: 1236,
                position: 202,
                text: "of".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1237,
                offset_to: 1242,
                position: 203,
                text: "sweat".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1243,
                offset_to: 1249,
                position: 204,
                text: "soaked".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1250,
                offset_to: 1256,
                position: 205,
                text: "cotton".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1257,
                offset_to: 1259,
                position: 206,
                text: "on".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1260,
                offset_to: 1265,
                position: 207,
                text: "their".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1266,
                offset_to: 1269,
                position: 208,
                text: "way".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1270,
                offset_to: 1272,
                position: 209,
                text: "to".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1273,
                offset_to: 1275,
                position: 210,
                text: "do".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1276,
                offset_to: 1281,
                position: 211,
                text: "their".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1282,
                offset_to: 1292,
                position: 212,
                text: "programmed".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1293,
                offset_to: 1297,
                position: 213,
                text: "task".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1299,
                offset_to: 1309,
                position: 214,
                text: "Infrequent".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1311,
                offset_to: 1314,
                position: 215,
                text: "yes".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1316,
                offset_to: 1324,
                position: 216,
                text: "unlikely".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1326,
                offset_to: 1329,
                position: 217,
                text: "yes".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1331,
                offset_to: 1334,
                position: 218,
                text: "but".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1335,
                offset_to: 1345,
                position: 219,
                text: "impossible".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1347,
                offset_to: 1348,
                position: 220,
                text: "I".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1349,
                offset_to: 1356,
                position: 221,
                text: "learned".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1357,
                offset_to: 1358,
                position: 222,
                text: "a".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1359,
                offset_to: 1363,
                position: 223,
                text: "long".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1364,
                offset_to: 1368,
                position: 224,
                text: "time".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1369,
                offset_to: 1372,
                position: 225,
                text: "ago".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1373,
                offset_to: 1378,
                position: 226,
                text: "never".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1379,
                offset_to: 1381,
                position: 227,
                text: "to".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1382,
                offset_to: 1385,
                position: 228,
                text: "say".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1386,
                offset_to: 1391,
                position: 229,
                text: "never".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1392,
                offset_to: 1394,
                position: 230,
                text: "in".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1395,
                offset_to: 1403,
                position: 231,
                text: "medicine".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1405,
                offset_to: 1406,
                position: 232,
                text: "g".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1408,
                offset_to: 1411,
                position: 233,
                text: "Len".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1412,
                offset_to: 1418,
                position: 234,
                text: "Howard".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1419,
                offset_to: 1421,
                position: 235,
                text: "MD".into(),
                position_length: 1,
            },
            Token {
                offset_from: 1423,
                offset_to: 1428,
                position: 236,
                text: "FACOG".into(),
                position_length: 1,
            },
        ];
        assert_eq!(result, expected);
    }
}
