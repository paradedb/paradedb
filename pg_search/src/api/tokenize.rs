// Copyright (c) 2023-2024 Retake, Inc.
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

use pgrx::{iter::TableIterator, name, pg_extern, JsonB};
use tokenizers::SearchTokenizer;

#[pg_extern]
pub fn tokenize(
    tokenizer_setting: JsonB,
    input_text: &str,
) -> TableIterator<(name!(token, String), name!(position, i32))> {
    let tokenizer_setting = serde_json::to_value(tokenizer_setting)
        .expect("invalid tokenizer setting, expected paradedb.tokenizer()");
    let tokenizer = SearchTokenizer::from_json_value(&tokenizer_setting)
        .expect("invalid tokenizer setting, expected paradedb.tokenizer()");

    let mut analyzer = tokenizer
        .to_tantivy_tokenizer()
        .expect("failed to convert tokenizer to tantivy tokenizer");

    let mut stream = analyzer.token_stream(input_text);

    let mut result = Vec::new();
    while stream.advance() {
        let token = stream.token();
        result.push((token.text.to_string(), token.position as i32));
    }

    TableIterator::new(result)
}
