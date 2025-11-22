/*
 *
 * IMPORTANT NOTICE:
 * This file has been copied from Quickwit, an open source project, and is subject to the terms
 * and conditions of the GNU Affero General Public License (AGPL) version 3.0.
 * Please review the full licensing details at <http://www.gnu.org/licenses/>.
 * By using this file, you agree to comply with the AGPL v3.0 terms.
 *
 */
use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::token::Token as LinderaToken;
use lindera::tokenizer::Tokenizer as LinderaTokenizer;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

// Default tokenizers with keep_whitespace=false (new MeCab-compatible behavior)
static CMN_TOKENIZER: Lazy<Arc<LinderaTokenizer>> = Lazy::new(|| {
    let dictionary = load_dictionary("embedded://cc-cedict")
        .expect("Lindera `cc-cedict` dictionary must be present");
    Arc::new(LinderaTokenizer::new(
        lindera::segmenter::Segmenter::new(Mode::Normal, dictionary, None).keep_whitespace(false),
    ))
});

static JPN_TOKENIZER: Lazy<Arc<LinderaTokenizer>> = Lazy::new(|| {
    let dictionary =
        load_dictionary("embedded://ipadic").expect("Lindera `ipadic` dictionary must be present");
    Arc::new(LinderaTokenizer::new(
        lindera::segmenter::Segmenter::new(Mode::Normal, dictionary, None).keep_whitespace(false),
    ))
});

static KOR_TOKENIZER: Lazy<Arc<LinderaTokenizer>> = Lazy::new(|| {
    let dictionary =
        load_dictionary("embedded://ko-dic").expect("Lindera `ko-dic` dictionary must be present");
    Arc::new(LinderaTokenizer::new(
        lindera::segmenter::Segmenter::new(Mode::Normal, dictionary, None).keep_whitespace(false),
    ))
});

// Tokenizers with keep_whitespace=true (backward compatibility)
static CMN_TOKENIZER_WITH_WS: Lazy<Arc<LinderaTokenizer>> = Lazy::new(|| {
    let dictionary = load_dictionary("embedded://cc-cedict")
        .expect("Lindera `cc-cedict` dictionary must be present");
    Arc::new(LinderaTokenizer::new(
        lindera::segmenter::Segmenter::new(Mode::Normal, dictionary, None).keep_whitespace(true),
    ))
});

static JPN_TOKENIZER_WITH_WS: Lazy<Arc<LinderaTokenizer>> = Lazy::new(|| {
    let dictionary =
        load_dictionary("embedded://ipadic").expect("Lindera `ipadic` dictionary must be present");
    Arc::new(LinderaTokenizer::new(
        lindera::segmenter::Segmenter::new(Mode::Normal, dictionary, None).keep_whitespace(true),
    ))
});

static KOR_TOKENIZER_WITH_WS: Lazy<Arc<LinderaTokenizer>> = Lazy::new(|| {
    let dictionary =
        load_dictionary("embedded://ko-dic").expect("Lindera `ko-dic` dictionary must be present");
    Arc::new(LinderaTokenizer::new(
        lindera::segmenter::Segmenter::new(Mode::Normal, dictionary, None).keep_whitespace(true),
    ))
});

#[derive(Clone, Default)]
pub struct LinderaChineseTokenizer {
    token: Token,
    keep_whitespace: bool,
}

impl LinderaChineseTokenizer {
    pub fn new(keep_whitespace: bool) -> Self {
        Self {
            token: Token::default(),
            keep_whitespace,
        }
    }
}

#[derive(Clone, Default)]
pub struct LinderaJapaneseTokenizer {
    token: Token,
    keep_whitespace: bool,
}

impl LinderaJapaneseTokenizer {
    pub fn new(keep_whitespace: bool) -> Self {
        Self {
            token: Token::default(),
            keep_whitespace,
        }
    }
}

#[derive(Clone, Default)]
pub struct LinderaKoreanTokenizer {
    token: Token,
    keep_whitespace: bool,
}

impl LinderaKoreanTokenizer {
    pub fn new(keep_whitespace: bool) -> Self {
        Self {
            token: Token::default(),
            keep_whitespace,
        }
    }
}

impl Tokenizer for LinderaChineseTokenizer {
    type TokenStream<'a> = MultiLanguageTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        if text.trim().is_empty() {
            return MultiLanguageTokenStream::Empty;
        }

        let tokenizer = if self.keep_whitespace {
            &CMN_TOKENIZER_WITH_WS
        } else {
            &CMN_TOKENIZER
        };

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

        let tokenizer = if self.keep_whitespace {
            &JPN_TOKENIZER_WITH_WS
        } else {
            &JPN_TOKENIZER
        };

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

        let tokenizer = if self.keep_whitespace {
            &KOR_TOKENIZER_WITH_WS
        } else {
            &KOR_TOKENIZER
        };

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
    fn test_lindera_chinese_tokenizer() {
        let mut tokenizer = LinderaChineseTokenizer::default();
        let tokens = test_helper(
            &mut tokenizer,
            "地址1，包含無效的字元 (包括符號與不標準的asci阿爾發字元",
        );
        assert_eq!(tokens.len(), 18);
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
    fn test_japanese_tokenizer() {
        let mut tokenizer = LinderaJapaneseTokenizer::default();
        {
            let tokens = test_helper(&mut tokenizer, "すもももももももものうち");
            assert_eq!(tokens.len(), 7);
            {
                let token = &tokens[0];
                assert_eq!(token.text, "すもも");
                assert_eq!(token.offset_from, 0);
                assert_eq!(token.offset_to, 9);
                assert_eq!(token.position, 0);
                assert_eq!(token.position_length, 1);
            }
        }
    }

    #[rstest]
    fn test_korean_tokenizer() {
        let mut tokenizer = LinderaKoreanTokenizer::default();
        {
            let tokens = test_helper(&mut tokenizer, "일본입니다. 매우 멋진 단어입니다.");
            assert_eq!(tokens.len(), 8);
            {
                let token = &tokens[0];
                assert_eq!(token.text, "일본");
                assert_eq!(token.offset_from, 0);
                assert_eq!(token.offset_to, 6);
                assert_eq!(token.position, 0);
                assert_eq!(token.position_length, 1);
            }
        }
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
