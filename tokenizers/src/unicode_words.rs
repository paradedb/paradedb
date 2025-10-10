use tantivy::tokenizer::{Token, TokenStream, Tokenizer};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Default)]
pub struct UnicodeWordsTokenizer {
    token: Token,
    remove_emojis: bool,
}

impl UnicodeWordsTokenizer {
    pub fn new(remove_emojis: bool) -> Self {
        Self {
            token: Default::default(),
            remove_emojis,
        }
    }
}

pub struct UnicodeWordsTokenStream<'a> {
    remove_emojis: bool,
    iter: unicode_segmentation::UWordBounds<'a>,
    token: &'a mut Token,
}

impl Tokenizer for UnicodeWordsTokenizer {
    type TokenStream<'a> = UnicodeWordsTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        UnicodeWordsTokenStream {
            remove_emojis: self.remove_emojis,
            iter: text.split_word_bounds(),
            token: &mut self.token,
        }
    }
}

impl TokenStream for UnicodeWordsTokenStream<'_> {
    fn advance(&mut self) -> bool {
        loop {
            if let Some(next) = self.iter.next() {
                let is_word = next.unicode_words().next().is_some();
                let keep = is_word
                    || (!self.remove_emojis && emoji::lookup_by_glyph::lookup(next).is_some());

                if !keep {
                    continue;
                }
                self.token.position = self.token.position.wrapping_add(1);

                self.token.text.clear();
                self.token.text.push_str(next);
                return true;
            }
            return false;
        }
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
    use crate::unicode_words::UnicodeWordsTokenizer;
    use tantivy::tokenizer::{TokenStream, Tokenizer};

    #[test]
    fn test_unicode_words_with_emojis() {
        let mut tokenizer = UnicodeWordsTokenizer::default();
        let mut stream = tokenizer.token_stream("it's Paul's birthday today!  🎂  hurray!");

        let mut tokens = vec![];
        while stream.advance() {
            let token = stream.token();

            tokens.push((token.text.clone(), token.position));
        }

        assert_eq!(
            tokens,
            vec![
                ("it's".into(), 0),
                ("Paul's".into(), 1),
                ("birthday".into(), 2),
                ("today".into(), 3),
                ("🎂".into(), 4),
                ("hurray".into(), 5)
            ]
        )
    }
    #[test]
    fn test_unicode_words_without_emojis() {
        let mut tokenizer = UnicodeWordsTokenizer::new(true);
        let mut stream = tokenizer.token_stream("it's Paul's birthday today!  🎂  hurray!");

        let mut tokens = vec![];
        while stream.advance() {
            let token = stream.token();

            tokens.push((token.text.clone(), token.position));
        }

        assert_eq!(
            tokens,
            vec![
                ("it's".into(), 0),
                ("Paul's".into(), 1),
                ("birthday".into(), 2),
                ("today".into(), 3),
                ("hurray".into(), 4)
            ]
        )
    }
}
