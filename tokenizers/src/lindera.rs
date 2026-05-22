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

/*
 *
 * IMPORTANT NOTICE:
 * This file has been copied from Quickwit, an open source project, and is subject to the terms
 * and conditions of the GNU Affero General Public License (AGPL) version 3.0.
 * Please review the full licensing details at <http://www.gnu.org/licenses/>.
 * By using this file, you agree to comply with the AGPL v3.0 terms.
 *
 */
use lindera::character_filter::unicode_normalize::{
    UnicodeNormalizeCharacterFilter, UnicodeNormalizeKind,
};
use lindera::character_filter::BoxCharacterFilter;
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::token::Token as LinderaToken;
use lindera::token_filter::japanese_reading_form::JapaneseReadingFormTokenFilter;
use lindera::token_filter::korean_reading_form::KoreanReadingFormTokenFilter;
use lindera::token_filter::BoxTokenFilter;
use lindera::tokenizer::Tokenizer as LinderaTokenizer;
use once_cell::sync::{Lazy, OnceCell};
use std::sync::Arc;
use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct LinderaOptions {
    keep_whitespace: bool,
    nfkc: bool,
    reading_form: bool,
}

impl LinderaOptions {
    const fn new(keep_whitespace: bool, nfkc: bool, reading_form: bool) -> Self {
        Self {
            keep_whitespace,
            nfkc,
            reading_form,
        }
    }

    const fn index(self) -> usize {
        (self.keep_whitespace as usize)
            | ((self.nfkc as usize) << 1)
            | ((self.reading_form as usize) << 2)
    }
}

#[derive(Clone, Copy)]
enum ReadingForm {
    None,
    Japanese,
    Korean,
}

fn build_lindera_tokenizer(
    dictionary_uri: &str,
    dictionary_name: &str,
    options: LinderaOptions,
    reading_form: ReadingForm,
) -> Arc<LinderaTokenizer> {
    let dictionary = load_dictionary(dictionary_uri)
        .unwrap_or_else(|_| panic!("Lindera `{dictionary_name}` dictionary must be present"));
    let mut tokenizer = LinderaTokenizer::new(
        lindera::segmenter::Segmenter::new(Mode::Normal, dictionary, None)
            .keep_whitespace(options.keep_whitespace),
    );

    if options.nfkc {
        tokenizer.append_character_filter(BoxCharacterFilter::from(
            UnicodeNormalizeCharacterFilter::new(UnicodeNormalizeKind::NFKC),
        ));
    }

    if options.reading_form {
        match reading_form {
            ReadingForm::None => {}
            ReadingForm::Japanese => {
                tokenizer.append_token_filter(BoxTokenFilter::from(
                    JapaneseReadingFormTokenFilter::new(),
                ));
            }
            ReadingForm::Korean => {
                tokenizer
                    .append_token_filter(BoxTokenFilter::from(KoreanReadingFormTokenFilter::new()));
            }
        };
    }

    Arc::new(tokenizer)
}

static CMN_TOKENIZERS: Lazy<[OnceCell<Arc<LinderaTokenizer>>; 8]> =
    Lazy::new(|| std::array::from_fn(|_| OnceCell::new()));
static JPN_TOKENIZERS: Lazy<[OnceCell<Arc<LinderaTokenizer>>; 8]> =
    Lazy::new(|| std::array::from_fn(|_| OnceCell::new()));
static KOR_TOKENIZERS: Lazy<[OnceCell<Arc<LinderaTokenizer>>; 8]> =
    Lazy::new(|| std::array::from_fn(|_| OnceCell::new()));

fn chinese_tokenizer(options: LinderaOptions) -> &'static Arc<LinderaTokenizer> {
    CMN_TOKENIZERS[options.index()].get_or_init(|| {
        build_lindera_tokenizer(
            "embedded://cc-cedict",
            "cc-cedict",
            options,
            ReadingForm::None,
        )
    })
}

fn japanese_tokenizer(options: LinderaOptions) -> &'static Arc<LinderaTokenizer> {
    JPN_TOKENIZERS[options.index()].get_or_init(|| {
        build_lindera_tokenizer(
            "embedded://ipadic",
            "ipadic",
            options,
            ReadingForm::Japanese,
        )
    })
}

fn korean_tokenizer(options: LinderaOptions) -> &'static Arc<LinderaTokenizer> {
    KOR_TOKENIZERS[options.index()].get_or_init(|| {
        build_lindera_tokenizer("embedded://ko-dic", "ko-dic", options, ReadingForm::Korean)
    })
}

#[derive(Clone, Default)]
pub struct LinderaChineseTokenizer {
    options: LinderaOptions,
    token: Token,
}
impl LinderaChineseTokenizer {
    pub fn new(keep_whitespace: bool) -> Self {
        Self::with_options(keep_whitespace, false)
    }

    pub fn with_options(keep_whitespace: bool, nfkc: bool) -> Self {
        Self {
            options: LinderaOptions::new(keep_whitespace, nfkc, false),
            token: Default::default(),
        }
    }
}

#[derive(Clone, Default)]
pub struct LinderaJapaneseTokenizer {
    token: Token,
    options: LinderaOptions,
}
impl LinderaJapaneseTokenizer {
    pub fn new(keep_whitespace: bool) -> Self {
        Self::with_options(keep_whitespace, false, false)
    }

    pub fn with_options(keep_whitespace: bool, nfkc: bool, reading_form: bool) -> Self {
        Self {
            options: LinderaOptions::new(keep_whitespace, nfkc, reading_form),
            token: Default::default(),
        }
    }
}

#[derive(Clone, Default)]
pub struct LinderaKoreanTokenizer {
    options: LinderaOptions,
    token: Token,
}
impl LinderaKoreanTokenizer {
    pub fn new(keep_whitespace: bool) -> Self {
        Self::with_options(keep_whitespace, false, false)
    }

    pub fn with_options(keep_whitespace: bool, nfkc: bool, reading_form: bool) -> Self {
        Self {
            options: LinderaOptions::new(keep_whitespace, nfkc, reading_form),
            token: Default::default(),
        }
    }
}

impl Tokenizer for LinderaChineseTokenizer {
    type TokenStream<'a> = MultiLanguageTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        if text.trim().is_empty() {
            return MultiLanguageTokenStream::Empty;
        }

        let tokenizer = chinese_tokenizer(self.options);

        let lindera_token_stream = LinderaTokenStream {
            tokens: tokenizer
                .tokenize(text)
                .expect("Lindera Chinese tokenizer failed"),
            token: &mut self.token,
        };

        MultiLanguageTokenStream::Lindera(lindera_token_stream)
    }
}

impl Tokenizer for LinderaJapaneseTokenizer {
    type TokenStream<'a> = MultiLanguageTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        if text.trim().is_empty() {
            return MultiLanguageTokenStream::Empty;
        }

        let tokenizer = japanese_tokenizer(self.options);

        let lindera_token_stream = LinderaTokenStream {
            tokens: tokenizer
                .tokenize(text)
                .expect("Lindera Japanese tokenizer failed"),
            token: &mut self.token,
        };

        MultiLanguageTokenStream::Lindera(lindera_token_stream)
    }
}

impl Tokenizer for LinderaKoreanTokenizer {
    type TokenStream<'a> = MultiLanguageTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        if text.trim().is_empty() {
            return MultiLanguageTokenStream::Empty;
        }

        let tokenizer = korean_tokenizer(self.options);

        let lindera_token_stream = LinderaTokenStream {
            tokens: tokenizer
                .tokenize(text)
                .expect("Lindera Korean tokenizer failed"),
            token: &mut self.token,
        };

        MultiLanguageTokenStream::Lindera(lindera_token_stream)
    }
}

pub enum MultiLanguageTokenStream<'a> {
    Empty,
    Lindera(LinderaTokenStream<'a>),
}

pub struct LinderaTokenStream<'a> {
    pub tokens: Vec<LinderaToken<'a>>,
    pub token: &'a mut Token,
}

impl TokenStream for MultiLanguageTokenStream<'_> {
    fn advance(&mut self) -> bool {
        match self {
            MultiLanguageTokenStream::Empty => false,
            MultiLanguageTokenStream::Lindera(tokenizer) => tokenizer.advance(),
        }
    }

    fn token(&self) -> &Token {
        match self {
            MultiLanguageTokenStream::Empty => {
                panic!("Cannot call token() on an empty token stream.")
            }
            MultiLanguageTokenStream::Lindera(tokenizer) => tokenizer.token(),
        }
    }

    fn token_mut(&mut self) -> &mut Token {
        match self {
            MultiLanguageTokenStream::Empty => {
                panic!("Cannot call token_mut() on an empty token stream.")
            }
            MultiLanguageTokenStream::Lindera(tokenizer) => tokenizer.token_mut(),
        }
    }
}

impl TokenStream for LinderaTokenStream<'_> {
    fn advance(&mut self) -> bool {
        if self.tokens.is_empty() {
            return false;
        }
        let token = self.tokens.remove(0);
        self.token.text = token.surface.to_string();
        self.token.offset_from = token.byte_start;
        self.token.offset_to = token.byte_end;
        self.token.position = token.position;
        self.token.position_length = token.position_length;

        true
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
    use super::*;
    use rstest::*;
    use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

    fn test_helper<T: Tokenizer>(tokenizer: &mut T, text: &str) -> Vec<Token> {
        let mut token_stream = tokenizer.token_stream(text);
        let mut tokens: Vec<Token> = vec![];
        while token_stream.advance() {
            tokens.push(token_stream.token().clone());
        }
        tokens
    }

    #[rstest]
    #[case(LinderaChineseTokenizer::new(true), 19)]
    #[case(LinderaChineseTokenizer::new(false), 18)]
    fn test_lindera_chinese_tokenizer(
        #[case] mut tokenizer: LinderaChineseTokenizer,
        #[case] expected_token_count: usize,
    ) {
        let tokens = test_helper(
            &mut tokenizer,
            "地址1，包含無效的字元 (包括符號與不標準的asci阿爾發字元",
        );
        // With keep_whitespace=true (backward compatible behavior), whitespace is included as a token
        assert_eq!(tokens.len(), expected_token_count);
        {
            let token = &tokens[0];
            assert_eq!(token.text, "地址");
            assert_eq!(token.offset_from, 0);
            assert_eq!(token.offset_to, 6);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
        }
    }

    #[rstest]
    #[case(LinderaJapaneseTokenizer::new(true), 8)]
    #[case(LinderaJapaneseTokenizer::new(false), 7)]
    fn test_lindera_japanese_tokenizer(
        #[case] mut tokenizer: LinderaJapaneseTokenizer,
        #[case] expected_token_count: usize,
    ) {
        let tokens = test_helper(&mut tokenizer, "すもも もももももものうち");
        assert_eq!(tokens.len(), expected_token_count);
        {
            let token = &tokens[0];
            assert_eq!(token.text, "すもも");
            assert_eq!(token.offset_from, 0);
            assert_eq!(token.offset_to, 9);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
        }
    }

    #[rstest]
    fn test_lindera_japanese_tokenizer_with_nfkc() {
        let mut tokenizer = LinderaJapaneseTokenizer::with_options(false, true, false);
        let tokens = test_helper(&mut tokenizer, "ＡＢＣ");

        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "ABC");
        assert_eq!(tokens[0].offset_from, 0);
        assert_eq!(tokens[0].offset_to, "ＡＢＣ".len());
    }

    #[rstest]
    fn test_lindera_japanese_tokenizer_with_reading_form() {
        let mut tokenizer_plain = LinderaJapaneseTokenizer::with_options(false, false, false);
        let mut tokenizer_reading = LinderaJapaneseTokenizer::with_options(false, false, true);

        let plain_tokens = test_helper(&mut tokenizer_plain, "東京");
        let reading_tokens = test_helper(&mut tokenizer_reading, "東京");

        assert_eq!(plain_tokens.len(), reading_tokens.len());
        assert!(!plain_tokens.is_empty());
        assert!(
            plain_tokens
                .iter()
                .zip(reading_tokens.iter())
                .any(|(plain, reading)| plain.text != reading.text)
        );
    }

    #[rstest]
    #[case(LinderaKoreanTokenizer::new(true), 11)]
    #[case(LinderaKoreanTokenizer::new(false), 8)]
    fn test_lindera_korean_tokenizer(
        #[case] mut tokenizer: LinderaKoreanTokenizer,
        #[case] expected_token_count: usize,
    ) {
        // With keep_whitespace=true (backward compatible behavior), whitespace is included as tokens
        let tokens = test_helper(&mut tokenizer, "일본입니다. 매우 멋진 단어입니다.");
        assert_eq!(tokens.len(), expected_token_count);
        {
            let token = &tokens[0];
            assert_eq!(token.text, "일본");
            assert_eq!(token.offset_from, 0);
            assert_eq!(token.offset_to, 6);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
        }
    }

    #[rstest]
    fn test_lindera_korean_tokenizer_with_reading_form() {
        let mut tokenizer_plain = LinderaKoreanTokenizer::with_options(false, false, false);
        let mut tokenizer_reading = LinderaKoreanTokenizer::with_options(false, false, true);

        let plain_tokens = test_helper(&mut tokenizer_plain, "大韓民國");
        let reading_tokens = test_helper(&mut tokenizer_reading, "大韓民國");

        assert_eq!(plain_tokens.len(), reading_tokens.len());
        assert!(!plain_tokens.is_empty());
        assert!(
            plain_tokens
                .iter()
                .zip(reading_tokens.iter())
                .any(|(plain, reading)| plain.text != reading.text)
        );
    }

    #[rstest]
    fn test_lindera_chinese_tokenizer_with_empty_string() {
        let mut tokenizer = LinderaChineseTokenizer::default();
        {
            let tokens = test_helper(&mut tokenizer, "");
            assert_eq!(tokens.len(), 0);
        }
        {
            let tokens = test_helper(&mut tokenizer, "    ");
            assert_eq!(tokens.len(), 0);
        }
    }

    #[rstest]
    fn test_japanese_tokenizer_with_empty_string() {
        let mut tokenizer = LinderaJapaneseTokenizer::default();
        {
            let tokens = test_helper(&mut tokenizer, "");
            assert_eq!(tokens.len(), 0);
        }
        {
            let tokens = test_helper(&mut tokenizer, "    ");
            assert_eq!(tokens.len(), 0);
        }
    }

    #[rstest]
    fn test_korean_tokenizer_with_empty_string() {
        let mut tokenizer = LinderaKoreanTokenizer::default();
        {
            let tokens = test_helper(&mut tokenizer, "");
            assert_eq!(tokens.len(), 0);
        }
        {
            let tokens = test_helper(&mut tokenizer, "    ");
            assert_eq!(tokens.len(), 0);
        }
    }
}
