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
    type TokenStream<'a> = LinderaTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        LinderaTokenStream {
            tokens: CMN_TOKENIZER
                .tokenize(text)
                .expect("Lindera Chinese tokenizer failed"),
            token: &mut self.token,
        }
    }
}

impl Tokenizer for LinderaJapaneseTokenizer {
    type TokenStream<'a> = LinderaTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        LinderaTokenStream {
            tokens: JPN_TOKENIZER
                .tokenize(text)
                .expect("Lindera Japanese tokenizer failed"),
            token: &mut self.token,
        }
    }
}

impl Tokenizer for LinderaKoreanTokenizer {
    type TokenStream<'a> = LinderaTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        LinderaTokenStream {
            tokens: KOR_TOKENIZER
                .tokenize(text)
                .expect("Lindera Korean tokenizer failed"),
            token: &mut self.token,
        }
    }
}

pub struct LinderaTokenStream<'a> {
    pub tokens: Vec<LinderaToken<'a>>,
    pub token: &'a mut Token,
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

#[cfg(test)]
mod tests {
    use super::*;
    use pgrx::*;
    use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

    fn test_helper<T: Tokenizer>(tokenizer: &mut T, text: &str) -> Vec<Token> {
        let mut token_stream = tokenizer.token_stream(text);
        let mut tokens: Vec<Token> = vec![];
        while token_stream.advance() {
            tokens.push(token_stream.token().clone());
        }
        tokens
    }

    #[test]
    fn test_chinese_tokenizer() {
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

    #[test]
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

    #[test]
    fn test_korean_tokenizer() {
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

    #[test]
    fn test_chinese_tokenizer_with_empty_string() {
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

    #[test]
    fn test_japanese_tokenizer_with_empty_string() {
        let mut tokenizer = LinderaJapaneseTokenizer::default();
        {
            let tokens = test_helper(&mut tokenizer, "");
            info!("TOKEN LEN {:?}", tokens.len());
            assert_eq!(tokens.len(), 0);
        }
        {
            let tokens = test_helper(&mut tokenizer, "    ");
            assert_eq!(tokens.len(), 0);
        }
    }

    #[test]
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
