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

//! Golden token-stream test for the Lindera tokenizers.
//!
//! The unit tests in `tokenizers::lindera` assert a handful of tokens each,
//! which is enough to prove an option is wired up but not enough to notice a
//! dictionary or segmentation change. This test records the *entire* token
//! stream — text, byte offsets, position and position length — for every
//! tokenizer and option combination over a fixed CJK corpus, and compares it
//! against a checked-in golden file.
//!
//! The point is index compatibility. A lindera bump, a dictionary rebuild or a
//! change to the filter chain that shifts tokenization silently invalidates
//! every BM25 index built with the previous version, and the diff of this file
//! is the evidence of what moved. A change here is not automatically a bug —
//! but it must be a deliberate, reviewed decision rather than a surprise.
//!
//! Regenerate after an intended change with:
//!
//! ```sh
//! UPDATE_LINDERA_GOLDEN=1 cargo test -p tokenizers --test lindera_golden
//! ```

use std::fmt::Write as _;
use std::path::PathBuf;

use tantivy::tokenizer::{TokenStream, Tokenizer};
use tokenizers::lindera::{
    LinderaChineseTokenizer, LinderaJapaneseTokenizer, LinderaKoreanTokenizer,
};

/// Inputs chosen to cover the cases where tokenization is most likely to move:
/// out-of-vocabulary words, script boundaries, width variants, and long
/// compounds where the Viterbi path has room to pick differently.
const CORPUS: &[&str] = &[
    // The inputs used by the unit tests in `tokenizers::lindera`.
    "地址1，包含無效的字元 (包括符號與不標準的asci阿爾發字元",
    "すもも もももももものうち",
    "일본입니다. 매우 멋진 단어입니다.",
    "ＡＢＣ１２３",
    "日本語",
    "韓國",
    // Mixed script, latin, digits, punctuation.
    "ParadeDB は Postgres 用の検索エンジンです。",
    "我们在2026年发布了ParadeDB 0.25.1版本。",
    "한국어 검색 엔진 ParadeDB, 버전 0.25.1 출시!",
    // Whitespace shapes.
    "  leading and trailing  ",
    "日本語\tタブ\n改行",
    // Numerals, units, half and full width.
    "１２３４５６７８９０",
    "３．１４１５９",
    "２０２６年８月４日",
    // Hanja and kanji shared across dictionaries.
    "大韓民國 서울特別市",
    "東京都渋谷区",
    "中華人民共和國",
    // Long compounds, where the Viterbi path is most likely to shift.
    "国際連合教育科学文化機関",
    "情報処理推進機構セキュリティセンター",
    "서울대학교병원어린이병원",
    "中华人民共和国国务院新闻办公室",
    // Emoji and symbols interleaved with CJK.
    "検索🔍エンジン",
    "検索…エンジン",
    "①②③",
    // Rare and out-of-vocabulary words, which exercise the unknown-word handler
    // and therefore the reading-form filter's treatment of it.
    "ヴァイオリン奏者ヴィヴァルディ",
    "쀍쀎쀏",
    "𠮷野家",
    // Empty and whitespace-only.
    "",
    "   ",
];

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/lindera_tokens.txt")
}

/// Append every token of `text` to `out`, one per line, prefixed by `label` so
/// each record identifies the tokenizer and option set that produced it.
fn dump<T: Tokenizer>(out: &mut String, label: &str, tokenizer: &mut T, text: &str) {
    if text.trim().is_empty() {
        // Matches the short-circuit in the tokenizer itself.
        writeln!(out, "{label}\t{text:?}\t<empty>").unwrap();
        return;
    }

    let mut stream = tokenizer.token_stream(text);
    let mut count = 0;
    while stream.advance() {
        let token = stream.token();
        writeln!(
            out,
            "{label}\t{text:?}\t{count}\t{:?}\t{}\t{}\t{}\t{}",
            token.text, token.offset_from, token.offset_to, token.position, token.position_length
        )
        .unwrap();
        count += 1;
    }
    writeln!(out, "{label}\t{text:?}\tcount={count}").unwrap();
}

fn token_dump() -> String {
    let mut out = String::new();

    for &keep_whitespace in &[false, true] {
        for &nfkc in &[false, true] {
            for &reading_form in &[false, true] {
                let opts = format!("ws={keep_whitespace},nfkc={nfkc},rf={reading_form}");

                // Chinese (cc-cedict) has no reading field, so the Chinese
                // tokenizer takes no reading_form option; it is dumped under
                // every option set anyway, which asserts that it does not move
                // when the others do.
                let mut zh = LinderaChineseTokenizer::with_options(keep_whitespace, nfkc);
                let mut ja =
                    LinderaJapaneseTokenizer::with_options(keep_whitespace, nfkc, reading_form);
                let mut ko =
                    LinderaKoreanTokenizer::with_options(keep_whitespace, nfkc, reading_form);

                for text in CORPUS {
                    dump(&mut out, &format!("zh[{opts}]"), &mut zh, text);
                    dump(&mut out, &format!("ja[{opts}]"), &mut ja, text);
                    dump(&mut out, &format!("ko[{opts}]"), &mut ko, text);
                }
            }
        }
    }

    out
}

/// The first few differing lines, with their line numbers, so a failure names
/// what moved instead of dumping four thousand lines into the test output.
fn describe_difference(expected: &str, actual: &str) -> String {
    const MAX_REPORTED: usize = 10;

    let expected: Vec<&str> = expected.lines().collect();
    let actual: Vec<&str> = actual.lines().collect();

    let mut report = String::new();
    let mut differing = 0;

    for (line, (expected, actual)) in expected.iter().zip(actual.iter()).enumerate() {
        if expected == actual {
            continue;
        }
        differing += 1;
        if differing <= MAX_REPORTED {
            writeln!(
                report,
                "line {}:\n  golden: {expected}\n  actual: {actual}",
                line + 1
            )
            .unwrap();
        }
    }

    if differing > MAX_REPORTED {
        writeln!(
            report,
            "... and {} more differing lines",
            differing - MAX_REPORTED
        )
        .unwrap();
    }

    if expected.len() != actual.len() {
        writeln!(
            report,
            "token record count changed: golden has {} lines, actual has {}",
            expected.len(),
            actual.len()
        )
        .unwrap();
    }

    report
}

#[test]
fn lindera_token_streams_match_golden() {
    let actual = token_dump();
    let path = golden_path();

    if std::env::var_os("UPDATE_LINDERA_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().expect("golden path has a parent"))
            .expect("golden directory must be creatable");
        std::fs::write(&path, &actual).expect("golden file must be writable");
        return;
    }

    let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "golden file {} could not be read ({e}); regenerate it with \
             UPDATE_LINDERA_GOLDEN=1 cargo test -p tokenizers --test lindera_golden",
            path.display()
        )
    });

    assert!(
        expected == actual,
        "Lindera tokenization no longer matches the golden file.\n\n{}\n\
         Indexes built with the previous tokenization will not match queries \
         parsed with this one. If the change is intended, regenerate the golden \
         file with:\n\n    UPDATE_LINDERA_GOLDEN=1 cargo test -p tokenizers \
         --test lindera_golden\n\nand review the resulting diff as part of the change.",
        describe_difference(&expected, &actual)
    );
}
