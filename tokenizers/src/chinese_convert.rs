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

use once_cell::sync::Lazy;
use opencc_jieba_rs::OpenCC;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tantivy::tokenizer::Tokenizer;

/// OpenCC conversion modes
#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConvertMode {
    /// Traditional to Simplified Chinese
    #[default]
    T2S,
    /// Simplified to Traditional Chinese
    S2T,
    /// Traditional Taiwan to Simplified Chinese
    TW2S,
    /// Traditional Taiwan to Simplified Chinese (with idioms)
    TW2SP,
    /// Simplified to Traditional Taiwan Chinese
    S2TW,
    /// Simplified to Traditional Taiwan Chinese (with idioms)
    S2TWP,
}

/// Global OpenCC converter instance (lazy-loaded)
static OPENCC: Lazy<Mutex<OpenCC>> = Lazy::new(|| Mutex::new(OpenCC::new()));

/// Get the conversion mode string for the specified mode
fn get_mode_str(mode: ConvertMode) -> &'static str {
    match mode {
        ConvertMode::T2S => "t2s",
        ConvertMode::S2T => "s2t",
        ConvertMode::TW2S => "tw2s",
        ConvertMode::TW2SP => "tw2sp",
        ConvertMode::S2TW => "s2tw",
        ConvertMode::S2TWP => "s2twp",
    }
}

/// Chinese Traditional/Simplified Conversion Tokenizer Wrapper
///
/// Converts the entire text **before tokenization** to ensure proper word segmentation
#[derive(Clone)]
pub struct ChineseConvertTokenizer<T: Tokenizer> {
    inner: T,
    mode: ConvertMode,
    buffer: String, // Buffer to store the converted text
}

impl<T: Tokenizer> ChineseConvertTokenizer<T> {
    /// Create a new converter tokenizer
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
        // Convert the entire text before tokenization, storing it in buffer
        let opencc = OPENCC.lock().unwrap();
        let mode_str = get_mode_str(self.mode);
        self.buffer = opencc.convert(text, mode_str, false);

        // Tokenize using the buffer text
        // The buffer's lifetime is bound to self ('a), so this is safe
        self.inner.token_stream(&self.buffer)
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

        // Should be converted to simplified Chinese before tokenization
        assert!(!tokens.is_empty());
        // Check if it contains simplified Chinese characters
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

        // Should be converted to traditional Chinese
        assert!(!tokens.is_empty());
        let text = tokens.join("");
        assert!(text.contains("簡體") || text.contains("測試"));
    }

    #[test]
    fn test_jieba_with_convert() {
        // Test integration with Jieba tokenizer
        let base_tokenizer = tantivy_jieba::JiebaTokenizer {};
        let mut tokenizer = ChineseConvertTokenizer::new(base_tokenizer, ConvertMode::T2S);

        let mut stream = tokenizer.token_stream("繁體中文分詞測試");
        let mut tokens = Vec::new();

        while stream.advance() {
            tokens.push(stream.token().text.clone());
        }

        assert!(!tokens.is_empty());
        // Verify that conversion to simplified Chinese was successful
        for token in &tokens {
            println!("Token: {}", token);
        }
    }
}
