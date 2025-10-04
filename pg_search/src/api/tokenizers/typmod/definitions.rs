use crate::api::tokenizers::typmod;
use crate::api::tokenizers::typmod::{load_typmod, ParsedTypmod};
use tantivy::tokenizer::Language;
use tokenizers::manager::{LinderaStyle, SearchTokenizerFilters};

pub struct GenericTypmod {
    parsed: ParsedTypmod,
    pub filters: SearchTokenizerFilters,
}

impl GenericTypmod {
    pub fn alias(&self) -> Option<String> {
        self.parsed
            .get("alias")
            .map(|p| p.as_str().unwrap().to_string())
    }
}

pub fn lookup_generic_typmod(typmod: i32) -> typmod::Result<GenericTypmod> {
    let parsed = load_typmod(typmod)?;
    let filters = SearchTokenizerFilters::from(&parsed);

    Ok(GenericTypmod { parsed, filters })
}

pub struct NgramTypmod {
    pub min_gram: usize,
    pub max_gram: usize,
    pub prefix_only: bool,
    pub filters: SearchTokenizerFilters,
}

pub fn lookup_ngram_typmod(typmod: i32) -> typmod::Result<NgramTypmod> {
    let parsed = load_typmod(typmod)?;
    let filters = SearchTokenizerFilters::from(&parsed);

    let min_gram = parsed
        .try_get("min", 0)
        .and_then(|p| p.as_usize())
        .ok_or(typmod::Error::MissingKey("min"))?;
    let max_gram = parsed
        .try_get("max", 1)
        .and_then(|p| p.as_usize())
        .ok_or(typmod::Error::MissingKey("max"))?;
    let prefix_only = parsed
        .try_get("prefix_only", 2)
        .and_then(|p| p.as_bool())
        .unwrap_or(false);

    Ok(NgramTypmod {
        min_gram,
        max_gram,
        prefix_only,
        filters,
    })
}

pub struct RegexTypmod {
    pub pattern: regex::Regex,
    pub filters: SearchTokenizerFilters,
}

pub fn lookup_regex_typmod(typmod: i32) -> typmod::Result<RegexTypmod> {
    let parsed = load_typmod(typmod)?;
    let filters = SearchTokenizerFilters::from(&parsed);
    let pattern = parsed
        .try_get("pattern", 0)
        .and_then(|p| p.as_regex())
        .ok_or(typmod::Error::MissingKey("pattern"))??;

    Ok(RegexTypmod { pattern, filters })
}

pub struct StemmedTypmod {
    pub language: Language,
    pub filters: SearchTokenizerFilters,
}

pub fn lookup_stemmed_typmod(typmod: i32) -> typmod::Result<StemmedTypmod> {
    let parsed = load_typmod(typmod)?;
    let filters = SearchTokenizerFilters::from(&parsed);
    let language = parsed
        .try_get("language", 0)
        .map(|p| p.as_language())
        .ok_or(typmod::Error::MissingKey("language"))??;
    Ok(StemmedTypmod { language, filters })
}

pub struct LinderaTypmod {
    pub style: LinderaStyle,
    pub filters: SearchTokenizerFilters,
}

pub fn lookup_lindera_typmod(typmod: i32) -> typmod::Result<LinderaTypmod> {
    let parsed = load_typmod(typmod)?;
    let filters = SearchTokenizerFilters::from(&parsed);
    let style = parsed
        .try_get("style", 0)
        .map(|p| match p.as_str() {
            None => panic!("missing style"),
            Some(s) => {
                let lcase = s.to_lowercase();
                match lcase.as_str() {
                    "chinese" => LinderaStyle::Chinese,
                    "japanese" => LinderaStyle::Japanese,
                    "korean" => LinderaStyle::Korean,
                    other => panic!("unknown lindera style: {other}"),
                }
            }
        })
        .ok_or(typmod::Error::MissingKey("style"))?;
    Ok(LinderaTypmod { style, filters })
}
