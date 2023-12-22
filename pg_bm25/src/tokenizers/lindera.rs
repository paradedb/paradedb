/*
 *
 * IMPORTANT NOTICE:
 * This file has been copied from Quickwit, an open source project, and is subject to the terms
 * and conditions of the GNU Affero General Public License (AGPL) version 3.0.
 * Please review the full licensing details at <http://www.gnu.org/licenses/>.
 * By using this file, you agree to comply with the AGPL v3.0 terms.
 *
 */

use lindera_core::mode::Mode;
use lindera_dictionary::{load_dictionary_from_config, DictionaryConfig, DictionaryKind};
use lindera_tokenizer::token::Token as LinderaToken;
use lindera_tokenizer::tokenizer::Tokenizer as LinderaTokenizer;
use once_cell::sync::Lazy;
use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

static CMN_TOKENIZER: Lazy<LinderaTokenizer> = Lazy::new(|| {
    let dictionary_config = DictionaryConfig {
        kind: Some(DictionaryKind::CcCedict),
        path: None,
    };
    let dictionary = load_dictionary_from_config(dictionary_config)
        .expect("Lindera `CcCedict` dictionary must be present");
    LinderaTokenizer::new(dictionary, None, Mode::Normal)
});

static JPN_TOKENIZER: Lazy<LinderaTokenizer> = Lazy::new(|| {
    let dictionary_config = DictionaryConfig {
        kind: Some(DictionaryKind::IPADIC),
        path: None,
    };
    let dictionary = load_dictionary_from_config(dictionary_config)
        .expect("Lindera `IPADIC` dictionary must be present");
    LinderaTokenizer::new(dictionary, None, Mode::Normal)
});

static KOR_TOKENIZER: Lazy<LinderaTokenizer> = Lazy::new(|| {
    let dictionary_config = DictionaryConfig {
        kind: Some(DictionaryKind::KoDic),
        path: None,
    };
    let dictionary = load_dictionary_from_config(dictionary_config)
        .expect("Lindera `KoDic` dictionary must be present");
    LinderaTokenizer::new(dictionary, None, Mode::Normal)
});

#[derive(Clone, Default)]
pub struct LinderaChineseTokenizer {
    token: Token,
}
#[derive(Clone, Default)]
pub struct LinderaJapaneseTokenizer {
    token: Token,
}
#[derive(Clone, Default)]
pub struct LinderaKoreanTokenizer {
    token: Token,
}

impl Tokenizer for LinderaChineseTokenizer {
    type TokenStream<'a> = MultiLanguageTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        if text.trim().is_empty() {
            return MultiLanguageTokenStream::Empty;
        }

        let lindera_token_stream = LinderaTokenStream {
            tokens: CMN_TOKENIZER
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

        let lindera_token_stream = LinderaTokenStream {
            tokens: JPN_TOKENIZER
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

        let lindera_token_stream = LinderaTokenStream {
            tokens: KOR_TOKENIZER
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

impl<'a> TokenStream for MultiLanguageTokenStream<'a> {
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

impl<'a> TokenStream for LinderaTokenStream<'a> {
    fn advance(&mut self) -> bool {
        if self.tokens.is_empty() {
            return false;
        }
        let token = self.tokens.remove(0);
        self.token.text = token.text.to_string();
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

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::*;
    use tantivy::tokenizer::{Token, TokenStream, Tokenizer};
    use shared::testing::SETUP_SQL;

    fn test_helper<T: Tokenizer>(tokenizer: &mut T, text: &str) -> Vec<Token> {
        let mut token_stream = tokenizer.token_stream(text);
        let mut tokens: Vec<Token> = vec![];
        while token_stream.advance() {
            tokens.push(token_stream.token().clone());
        }
        tokens
    }

    #[pg_test]
    fn test_lindera_chinese_tokenizer() {
        let mut tokenizer = LinderaChineseTokenizer::default();
        let tokens = test_helper(
            &mut tokenizer,
            "地址1，包含無效的字元 (包括符號與不標準的asci阿爾發字元",
        );
        assert_eq!(tokens.len(), 19);
        {
            let token = &tokens[0];
            assert_eq!(token.text, "地址");
            assert_eq!(token.offset_from, 0);
            assert_eq!(token.offset_to, 6);
            assert_eq!(token.position, 0);
            assert_eq!(token.position_length, 1);
        }
    }

    #[pg_test]
    fn test_lindera_japanese_tokenizer() {
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

    #[pg_test]
    fn test_lindera_korean_tokenizer() {
        let mut tokenizer = LinderaKoreanTokenizer::default();
        {
            let tokens = test_helper(&mut tokenizer, "일본입니다. 매우 멋진 단어입니다.");
            assert_eq!(tokens.len(), 11);
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

    #[pg_test]
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

    #[pg_test]
    fn test_lindera_japanese_tokenizer_with_empty_string() {
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

    #[pg_test]
    fn test_lindera_korean_tokenizer_with_empty_string() {
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

    #[pg_test]
    fn test_lindera_chinese_tokenizer_with_sql() {
        Spi::run(SETUP_SQL).expect("failed to setup index");

        let query1: &str = Spi::get_one("SELECT title FROM chinese.search('author:华')").expect("failed to query").unwrap();
        assert_eq!(query1, "北京的新餐馆");

        let query2: &str = Spi::get_one("SELECT message FROM chinese.search('title:北京')").expect("failed to query").unwrap();
        assert_eq!(query2, "北京市中心新开了一家餐馆，以其现代设计和独特的菜肴选择而闻名。");

        let query3: &str = Spi::get_one("SELECT author FROM chinese.search('message:文化节')").expect("failed to query").unwrap();
        assert_eq!(query3, "王芳");
    }

    #[pg_test]
    fn test_lindera_japanese_tokenizer_with_sql() {
        Spi::run(SETUP_SQL).expect("failed to setup index");

        let query1: &str = Spi::get_one("SELECT title FROM japanese.search('author:佐藤')").expect("failed to query").unwrap();
        assert_eq!(query1, "東京の新しいカフェ");

        let query2: &str = Spi::get_one("SELECT message FROM japanese.search('title:サッカー')").expect("failed to query").unwrap();
        assert_eq!(query2, "昨日のサッカー試合では素晴らしいゴールが見られました。終了間際のドラマチックな展開がハイライトでした。");

        let query3: &str = Spi::get_one("SELECT author FROM japanese.search('message:祭り')").expect("failed to query").unwrap();
        assert_eq!(query3, "高橋花子");
    }

    #[pg_test]
    fn test_lindera_korean_tokenizer_with_sql() {
        Spi::run(SETUP_SQL).expect("failed to setup index");

        let query1: &str = Spi::get_one("SELECT title FROM korean.search('author:김민준')").expect("failed to query").unwrap();
        assert_eq!(query1, "서울의 새로운 카페");

        let query2: &str = Spi::get_one("SELECT message FROM korean.search('title:경기')").expect("failed to query").unwrap();
        assert_eq!(query2, "어제 열린 축구 경기에서 화려한 골이 터졌습니다. 마지막 순간의 반전이 경기의 하이라이트였습니다.");

        let query3: &str = Spi::get_one("SELECT author FROM korean.search('message:지역 축제')").expect("failed to query").unwrap();
        assert_eq!(query3, "박지후");
    }
}
