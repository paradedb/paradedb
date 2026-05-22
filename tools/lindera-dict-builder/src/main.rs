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

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use lindera_dictionary::assets::{fetch, FetchParams};
use lindera_dictionary::builder::DictionaryBuilder;
use lindera_dictionary::decompress::{Algorithm, CompressedData};
use lindera_dictionary::dictionary::metadata::Metadata;

const LINDERA_VERSION: &str = "1.5.1";
const READY_MARKER: &str = ".paradedb-lindera-mmap-ready";

const COMPONENT_FILES: &[&str] = &[
    "metadata.json",
    "char_def.bin",
    "unk.bin",
    "matrix.mtx",
    "dict.da",
    "dict.vals",
    "dict.wordsidx",
    "dict.words",
];

struct DictionarySpec {
    name: &'static str,
    output_dir: &'static str,
    file_name: &'static str,
    input_dir: &'static str,
    dummy_input: &'static str,
    download_urls: &'static [&'static str],
    md5_hash: &'static str,
    metadata_json: &'static str,
}

const DICTIONARIES: &[DictionarySpec] = &[
    DictionarySpec {
        name: "cc-cedict",
        output_dir: "lindera-cc-cedict",
        file_name: "CC-CEDICT-MeCab-0.1.0-20200409.tar.gz",
        input_dir: "CC-CEDICT-MeCab-0.1.0-20200409",
        dummy_input: "测试,0,0,-1131,*,*,*,*,ce4 shi4,測試,测试,to test (machinery etc)/to test (students)/test/quiz/exam/beta (software)/\n",
        download_urls: &["https://lindera.dev/CC-CEDICT-MeCab-0.1.0-20200409.tar.gz"],
        md5_hash: "aba9748b70f37feede97b70c5d37f8a0",
        metadata_json: r#"{
  "name": "cc-cedict",
  "encoding": "UTF-8",
  "compress_algorithm": "raw",
  "default_word_cost": -10000,
  "default_left_context_id": 0,
  "default_right_context_id": 0,
  "default_field_value": "*",
  "flexible_csv": true,
  "skip_invalid_cost_or_id": true,
  "normalize_details": false,
  "dictionary_schema": {
    "fields": [
      "surface",
      "left_context_id",
      "right_context_id",
      "cost",
      "part_of_speech",
      "part_of_speech_subcategory_1",
      "part_of_speech_subcategory_2",
      "part_of_speech_subcategory_3",
      "pinyin",
      "traditional",
      "simplified",
      "definition"
    ]
  },
  "user_dictionary_schema": {
    "fields": ["surface", "part_of_speech", "pinyin"]
  }
}"#,
    },
    DictionarySpec {
        name: "ipadic",
        output_dir: "lindera-ipadic",
        file_name: "mecab-ipadic-2.7.0-20250920.tar.gz",
        input_dir: "mecab-ipadic-2.7.0-20250920",
        dummy_input: "テスト,1288,1288,-1000,名詞,固有名詞,一般,*,*,*,*,*,*\n",
        download_urls: &["https://lindera.dev/mecab-ipadic-2.7.0-20250920.tar.gz"],
        md5_hash: "a95c409f12f1023fce8ef91f991ef042",
        metadata_json: r#"{
  "name": "ipadic",
  "encoding": "UTF-8",
  "compress_algorithm": "raw",
  "default_word_cost": -10000,
  "default_left_context_id": 0,
  "default_right_context_id": 0,
  "default_field_value": "*",
  "flexible_csv": true,
  "skip_invalid_cost_or_id": false,
  "normalize_details": true,
  "dictionary_schema": {
    "fields": [
      "surface",
      "left_context_id",
      "right_context_id",
      "cost",
      "part_of_speech",
      "part_of_speech_subcategory_1",
      "part_of_speech_subcategory_2",
      "part_of_speech_subcategory_3",
      "conjugation_form",
      "conjugation_type",
      "base_form",
      "reading",
      "pronunciation"
    ]
  },
  "user_dictionary_schema": {
    "fields": ["surface", "part_of_speech", "reading"]
  }
}"#,
    },
    DictionarySpec {
        name: "ko-dic",
        output_dir: "lindera-ko-dic",
        file_name: "mecab-ko-dic-2.1.1-20180720.tar.gz",
        input_dir: "mecab-ko-dic-2.1.1-20180720",
        dummy_input: "테스트,1785,3543,4721,NNG,행위,F,테스트,*,*,*,*\n",
        download_urls: &["https://lindera.dev/mecab-ko-dic-2.1.1-20180720.tar.gz"],
        md5_hash: "b996764e91c96bc89dc32ea208514a96",
        metadata_json: r#"{
  "name": "ko-dic",
  "encoding": "UTF-8",
  "compress_algorithm": "raw",
  "default_word_cost": -10000,
  "default_left_context_id": 0,
  "default_right_context_id": 0,
  "default_field_value": "*",
  "flexible_csv": false,
  "skip_invalid_cost_or_id": false,
  "normalize_details": false,
  "dictionary_schema": {
    "fields": [
      "surface",
      "left_context_id",
      "right_context_id",
      "cost",
      "part_of_speech_tag",
      "meaning",
      "presence_absence",
      "reading",
      "type",
      "first_part_of_speech",
      "last_part_of_speech",
      "expression"
    ]
  },
  "user_dictionary_schema": {
    "fields": ["surface", "part_of_speech_tag", "reading"]
  }
}"#,
    },
];

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let destination = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .context("usage: lindera-dict-builder <destination-root>")?;
    let cache_root = env::var_os("LINDERA_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/lindera-dict-cache"));

    env::set_var("LINDERA_CACHE", &cache_root);
    env::set_var("CARGO_PKG_VERSION", LINDERA_VERSION);

    for dictionary in DICTIONARIES {
        build_dictionary(dictionary, &cache_root, &destination).await?;
    }

    Ok(())
}

async fn build_dictionary(
    dictionary: &DictionarySpec,
    cache_root: &Path,
    destination_root: &Path,
) -> anyhow::Result<()> {
    let cache_dir = cache_root.join(LINDERA_VERSION).join(dictionary.output_dir);
    let already_ready = mmap_dictionary_ready(&cache_dir)?;
    if !already_ready {
        let _ = fs::remove_dir_all(&cache_dir);
    }

    let mut metadata: Metadata = serde_json::from_str(dictionary.metadata_json)
        .with_context(|| format!("failed to parse {} metadata", dictionary.name))?;
    metadata.compress_algorithm = Algorithm::Raw;

    let builder = DictionaryBuilder::new(metadata);
    fetch(
        FetchParams {
            file_name: dictionary.file_name,
            input_dir: dictionary.input_dir,
            output_dir: dictionary.output_dir,
            dummy_input: dictionary.dummy_input,
            download_urls: dictionary.download_urls,
            md5_hash: dictionary.md5_hash,
        },
        builder,
    )
    .await
    .with_context(|| format!("failed to build {} dictionary", dictionary.name))?;

    if !already_ready {
        prepare_for_lindera_mmap(&cache_dir)?;
    }

    if !mmap_dictionary_ready(&cache_dir)? {
        bail!(
            "built {} dictionary at {} is incomplete or not mmap-ready",
            dictionary.name,
            cache_dir.display()
        );
    }

    copy_dictionary(&cache_dir, &destination_root.join(dictionary.name))
        .with_context(|| format!("failed to install {} dictionary", dictionary.name))
}

fn mmap_dictionary_ready(path: &Path) -> anyhow::Result<bool> {
    if !path.join(READY_MARKER).is_file()
        || !path.is_dir()
        || !COMPONENT_FILES.iter().all(|file| path.join(file).is_file())
    {
        return Ok(false);
    }

    let metadata = fs::read(path.join("metadata.json"))
        .with_context(|| format!("failed to read metadata from {}", path.display()))?;
    let metadata: Metadata = serde_json::from_slice(&metadata)
        .with_context(|| format!("failed to parse metadata from {}", path.display()))?;

    Ok(metadata.compress_algorithm == Algorithm::Raw)
}

fn prepare_for_lindera_mmap(path: &Path) -> anyhow::Result<()> {
    // Lindera's mmap path only maps the large matrix/prefix-dictionary files.
    // Character definitions and unknown-word data still use the compression-aware
    // loader, so wrap those two files as raw CompressedData and leave the mmap
    // files in their original raw format.
    wrap_for_compress_loader(&path.join("char_def.bin"))?;
    wrap_for_compress_loader(&path.join("unk.bin"))?;
    fs::write(path.join(READY_MARKER), [])
        .with_context(|| format!("failed to write marker in {}", path.display()))
}

fn wrap_for_compress_loader(path: &Path) -> anyhow::Result<()> {
    let data =
        fs::read(path).with_context(|| format!("failed to read component {}", path.display()))?;
    let wrapped = CompressedData::new(Algorithm::Raw, data);
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&wrapped)
        .with_context(|| format!("failed to serialize component {}", path.display()))?;

    fs::write(path, bytes).with_context(|| format!("failed to write component {}", path.display()))
}

fn copy_dictionary(source: &Path, destination: &Path) -> anyhow::Result<()> {
    let _ = fs::remove_dir_all(destination);
    fs::create_dir_all(destination)
        .with_context(|| format!("failed to create {}", destination.display()))?;

    for component in COMPONENT_FILES {
        fs::copy(source.join(component), destination.join(component)).with_context(|| {
            format!(
                "failed to copy {} to {}",
                source.join(component).display(),
                destination.join(component).display()
            )
        })?;
    }

    Ok(())
}
