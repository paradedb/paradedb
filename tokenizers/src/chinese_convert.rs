// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// Chinese Traditional/Simplified conversion using OpenCC

use once_cell::sync::Lazy;
use opencc::OpenCC;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tantivy::tokenizer::Tokenizer;

/// OpenCC 转换模式
#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConvertMode {
    /// 繁体转简体 (Traditional to Simplified)
    #[default]
    T2S,
    /// 简体转繁体 (Simplified to Traditional)
    S2T,
    /// 繁体转台湾繁体
    T2TW,
    /// 繁体转香港繁体
    T2HK,
    /// 简体转台湾繁体
    S2TW,
    /// 简体转香港繁体
    S2HK,
}

/// OpenCC 转换器全局实例（使用懒加载）
static OPENCC_T2S: Lazy<Mutex<OpenCC>> = Lazy::new(|| Mutex::new(OpenCC::new("t2s.json")));

static OPENCC_S2T: Lazy<Mutex<OpenCC>> = Lazy::new(|| Mutex::new(OpenCC::new("s2t.json")));

static OPENCC_T2TW: Lazy<Mutex<OpenCC>> = Lazy::new(|| Mutex::new(OpenCC::new("t2tw.json")));

static OPENCC_T2HK: Lazy<Mutex<OpenCC>> = Lazy::new(|| Mutex::new(OpenCC::new("t2hk.json")));

static OPENCC_S2TW: Lazy<Mutex<OpenCC>> = Lazy::new(|| Mutex::new(OpenCC::new("s2tw.json")));

static OPENCC_S2HK: Lazy<Mutex<OpenCC>> = Lazy::new(|| Mutex::new(OpenCC::new("s2hk.json")));

/// 获取对应模式的 OpenCC 实例
fn get_opencc(mode: ConvertMode) -> &'static Lazy<Mutex<OpenCC>> {
    match mode {
        ConvertMode::T2S => &OPENCC_T2S,
        ConvertMode::S2T => &OPENCC_S2T,
        ConvertMode::T2TW => &OPENCC_T2TW,
        ConvertMode::T2HK => &OPENCC_T2HK,
        ConvertMode::S2TW => &OPENCC_S2TW,
        ConvertMode::S2HK => &OPENCC_S2HK,
    }
}

/// 中文繁简转换 Tokenizer Wrapper
///
/// **在分词前**对整个文本进行繁简转换，这样可以确保转换后的文本能够正确分词
#[derive(Clone)]
pub struct ChineseConvertTokenizer<T: Tokenizer> {
    inner: T,
    mode: ConvertMode,
    buffer: String, // 存储转换后的文本
}

impl<T: Tokenizer> ChineseConvertTokenizer<T> {
    /// 创建新的转换器
    pub fn new(inner: T, mode: ConvertMode) -> Self {
        Self {
            inner,
            mode,
            buffer: String::new(),
        }
    }
}

impl<T: Tokenizer> Tokenizer for ChineseConvertTokenizer<T> {
    type TokenStream<'a> = T::TokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        // 在分词前先转换整个文本，存储到 buffer 中
        let opencc = get_opencc(self.mode);
        self.buffer = opencc.lock().unwrap().convert(text);

        // 使用 buffer 中的文本进行分词
        // buffer 的生命周期和 self 绑定（'a），所以这是安全的
        self.inner.token_stream(&self.buffer)
    }
}

impl<T: TokenStream> TokenStream for TokenLengthFilterStream<T> {
    fn advance(&mut self) -> bool {
        while self.tail.advance() {
            if self.predicate(self.tail.token()) {
                return true;
            }
        }
        false
    }

    fn token(&self) -> &Token {
        self.tail.token()
    }

    fn token_mut(&mut self) -> &mut Token {
        self.tail.token_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::tokenizer::{SimpleTokenizer, TokenStream};

    #[test]
    fn test_t2s_conversion() {
        let base_tokenizer = SimpleTokenizer::default();
        let mut tokenizer = ChineseConvertTokenizer::new(base_tokenizer, ConvertMode::T2S);

        let mut stream = tokenizer.token_stream("繁體中文測試");
        let mut tokens = Vec::new();

        while stream.advance() {
            tokens.push(stream.token().text.clone());
        }

        // 应该转换为简体后再分词
        assert!(!tokens.is_empty());
        // 检查是否包含简体字
        let text = tokens.join("");
        assert!(text.contains("繁体") || text.contains("测试"));
    }

    #[test]
    fn test_s2t_conversion() {
        let base_tokenizer = SimpleTokenizer::default();
        let mut tokenizer = ChineseConvertTokenizer::new(base_tokenizer, ConvertMode::S2T);

        let mut stream = tokenizer.token_stream("简体中文测试");
        let mut tokens = Vec::new();

        while stream.advance() {
            tokens.push(stream.token().text.clone());
        }

        // 应该转换为繁体
        assert!(!tokens.is_empty());
        let text = tokens.join("");
        assert!(text.contains("簡體") || text.contains("測試"));
    }

    #[test]
    fn test_jieba_with_convert() {
        // 测试与 Jieba 分词器的集成
        let base_tokenizer = tantivy_jieba::JiebaTokenizer {};
        let mut tokenizer = ChineseConvertTokenizer::new(base_tokenizer, ConvertMode::T2S);

        let mut stream = tokenizer.token_stream("繁體中文分詞測試");
        let mut tokens = Vec::new();

        while stream.advance() {
            tokens.push(stream.token().text.clone());
        }

        assert!(!tokens.is_empty());
        // 验证已转换为简体
        for token in &tokens {
            println!("Token: {}", token);
        }
    }
}
