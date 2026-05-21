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
use lindera::dictionary::Dictionary;
use lindera::mode::Mode;
use lindera::token::Token as LinderaToken;
use lindera::token_filter::japanese_reading_form::JapaneseReadingFormTokenFilter;
use lindera::token_filter::korean_reading_form::KoreanReadingFormTokenFilter;
use lindera::token_filter::BoxTokenFilter;
use lindera::tokenizer::Tokenizer as LinderaTokenizer;
use once_cell::sync::{Lazy, OnceCell};
use std::sync::Arc;
use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

/// The set of configurable options that distinguish one cached Lindera
/// tokenizer from another. Each unique combination maps to a single lazily
/// built tokenizer instance per language (see [`LinderaOptions::index`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct LinderaOptions {
    keep_whitespace: bool,
    /// Apply Unicode NFKC normalization as a character filter, before
    /// segmentation.
    nfkc: bool,
    /// Replace each token's surface form with its dictionary reading form, as
    /// a token filter, after segmentation. Only meaningful for Japanese and
    /// Korean.
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

    /// A dense index in `0..8` derived from the three boolean options, used to
    /// address the per-language tokenizer cache.
    const fn index(self) -> usize {
        (self.keep_whitespace as usize)
            | ((self.nfkc as usize) << 1)
            | ((self.reading_form as usize) << 2)
    }
}

/// Which reading-form token filter, if any, a language supports.
#[derive(Clone, Copy)]
enum ReadingForm {
    None,
    Japanese,
    Korean,
}

static CMN_DICTIONARY: Lazy<Dictionary> = Lazy::new(|| load_mmap_dictionary("cc-cedict"));
static JPN_DICTIONARY: Lazy<Dictionary> = Lazy::new(|| load_mmap_dictionary("ipadic"));
static KOR_DICTIONARY: Lazy<Dictionary> = Lazy::new(|| load_mmap_dictionary("ko-dic"));

fn load_mmap_dictionary(name: &str) -> Dictionary {
    crate::lindera_mmap::load_dictionary(name).unwrap_or_else(|err| {
        panic!("Lindera `{name}` dictionary must be installed as mmap component files: {err:#}")
    })
}

/// Build a Lindera tokenizer for `dictionary`, appending the NFKC
/// character filter and the reading-form token filter when the corresponding
/// options are set.
fn build_lindera_tokenizer(
    dictionary: Dictionary,
    options: LinderaOptions,
    reading_form: ReadingForm,
) -> Arc<LinderaTokenizer> {
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
        }
    }

    Arc::new(tokenizer)
}

// One lazily built tokenizer per option combination per language. There are
// only three boolean options, so eight slots cover every combination.
static CMN_TOKENIZERS: Lazy<[OnceCell<Arc<LinderaTokenizer>>; 8]> =
    Lazy::new(|| std::array::from_fn(|_| OnceCell::new()));
static JPN_TOKENIZERS: Lazy<[OnceCell<Arc<LinderaTokenizer>>; 8]> =
    Lazy::new(|| std::array::from_fn(|_| OnceCell::new()));
static KOR_TOKENIZERS: Lazy<[OnceCell<Arc<LinderaTokenizer>>; 8]> =
    Lazy::new(|| std::array::from_fn(|_| OnceCell::new()));

fn chinese_tokenizer(options: LinderaOptions) -> &'static Arc<LinderaTokenizer> {
    CMN_TOKENIZERS[options.index()]
        .get_or_init(|| build_lindera_tokenizer(CMN_DICTIONARY.clone(), options, ReadingForm::None))
}

fn japanese_tokenizer(options: LinderaOptions) -> &'static Arc<LinderaTokenizer> {
    JPN_TOKENIZERS[options.index()].get_or_init(|| {
        build_lindera_tokenizer(JPN_DICTIONARY.clone(), options, ReadingForm::Japanese)
    })
}

fn korean_tokenizer(options: LinderaOptions) -> &'static Arc<LinderaTokenizer> {
    KOR_TOKENIZERS[options.index()].get_or_init(|| {
        build_lindera_tokenizer(KOR_DICTIONARY.clone(), options, ReadingForm::Korean)
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
            // Chinese (cc-cedict) has no reading field, so reading_form is
            // always false here.
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

    fn skip_if_lindera_dictionaries_are_missing() -> bool {
        if crate::lindera_mmap::installed_dictionaries_ready() {
            return false;
        }

        eprintln!(
            "skipping Lindera tokenizer test; set {} to a preinstalled dictionary root",
            crate::lindera_mmap::DICTIONARY_ROOT_ENV
        );
        true
    }

    #[rstest]
    #[case(LinderaChineseTokenizer::new(true), 19)]
    #[case(LinderaChineseTokenizer::new(false), 18)]
    fn test_lindera_chinese_tokenizer(
        #[case] mut tokenizer: LinderaChineseTokenizer,
        #[case] expected_token_count: usize,
    ) {
        if skip_if_lindera_dictionaries_are_missing() {
            return;
        }

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
        if skip_if_lindera_dictionaries_are_missing() {
            return;
        }

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
    #[case(LinderaKoreanTokenizer::new(true), 11)]
    #[case(LinderaKoreanTokenizer::new(false), 8)]
    fn test_lindera_korean_tokenizer(
        #[case] mut tokenizer: LinderaKoreanTokenizer,
        #[case] expected_token_count: usize,
    ) {
        if skip_if_lindera_dictionaries_are_missing() {
            return;
        }

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

    fn token_texts<T: Tokenizer>(tokenizer: &mut T, text: &str) -> Vec<String> {
        test_helper(tokenizer, text)
            .iter()
            .map(|token| token.text.clone())
            .collect()
    }

    // NFKC is a character filter that runs before segmentation. Enabling it
    // normalizes full-width compatibility characters (e.g. "ＡＢＣ１２３") to
    // their canonical half-width forms, which then also changes how they
    // segment. The OFF/ON comparison proves the option is not a no-op.
    #[rstest]
    fn test_lindera_japanese_tokenizer_with_nfkc() {
        let input = "ＡＢＣ１２３";

        let mut off = LinderaJapaneseTokenizer::with_options(false, false, false);
        let off_tokens = token_texts(&mut off, input);
        assert_eq!(off_tokens, vec!["ＡＢＣ", "１", "２", "３"]);

        let mut on = LinderaJapaneseTokenizer::with_options(false, true, false);
        let on_tokens = token_texts(&mut on, input);
        assert_eq!(on_tokens, vec!["ABC", "123"]);

        assert_ne!(
            off_tokens, on_tokens,
            "nfkc must change the token stream; otherwise the option is a no-op"
        );
    }

    // The Japanese reading-form token filter runs after segmentation and
    // replaces each recognized token's surface form with its IPADIC reading
    // (katakana). "日本語" -> "ニホンゴ".
    #[rstest]
    fn test_lindera_japanese_tokenizer_with_reading_form() {
        let input = "日本語";

        let mut off = LinderaJapaneseTokenizer::with_options(false, false, false);
        let off_tokens = token_texts(&mut off, input);
        assert_eq!(off_tokens, vec!["日本語"]);

        let mut on = LinderaJapaneseTokenizer::with_options(false, false, true);
        let on_tokens = token_texts(&mut on, input);
        assert_eq!(on_tokens, vec!["ニホンゴ"]);

        assert_ne!(
            off_tokens, on_tokens,
            "reading_form must change the token stream; otherwise the option is a no-op"
        );
    }

    // The Korean reading-form token filter replaces Hanja (Chinese-character)
    // tokens with their ko-dic Hangul reading. "韓國" -> "한국".
    #[rstest]
    fn test_lindera_korean_tokenizer_with_reading_form() {
        let input = "韓國";

        let mut off = LinderaKoreanTokenizer::with_options(false, false, false);
        let off_tokens = token_texts(&mut off, input);
        assert_eq!(off_tokens, vec!["韓國"]);

        let mut on = LinderaKoreanTokenizer::with_options(false, false, true);
        let on_tokens = token_texts(&mut on, input);
        assert_eq!(on_tokens, vec!["한국"]);

        assert_ne!(
            off_tokens, on_tokens,
            "reading_form must change the token stream; otherwise the option is a no-op"
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
