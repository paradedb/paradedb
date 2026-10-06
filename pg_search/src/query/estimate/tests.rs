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

use super::*;
use tantivy::schema::{FAST, INDEXED, STRING, Schema, TEXT};
use tantivy::{Index, IndexWriter, TantivyDocument, doc};

fn fixture(stats: bool) -> (Index, Field, Field, Field, Field) {
    let mut schema = Schema::builder();
    let text = schema.add_text_field("text", TEXT);
    let literal = schema.add_text_field("literal", STRING);
    let number = schema.add_u64_field("number", FAST);
    let label = schema.add_text_field("label", FAST);
    let mut builder = Index::builder().schema(schema.build());
    if stats {
        builder = builder.register_plugin(std::sync::Arc::new(crate::index::stats::StatsPlugin));
    }
    let index = builder.create_in_ram().unwrap();
    let mut writer: IndexWriter = index.writer_with_num_threads(1, 15_000_000).unwrap();
    for (words, value, name) in [
        ("red cat", 10, "apple"),
        ("red dog", 20, "banana"),
        ("blue cat", 30, "cherry"),
        ("green fish", 40, "date"),
    ] {
        writer
            .add_document(
                doc!(text => words, literal => words, number => value as u64, label => name),
            )
            .unwrap();
    }
    writer.commit().unwrap();
    (index, text, literal, number, label)
}

fn estimate_query(index: &Index, query: Box<dyn Query>) -> Option<(u32, u64)> {
    let reader = index.reader().unwrap();
    MetadataQuery::new(query, None)
        .estimate_docs(&reader.searcher().segment_readers()[0])
        .unwrap()
}

fn term(field: Field, word: &str) -> Box<dyn Query> {
    Box::new(TermQuery::new(
        Term::from_field_text(field, word),
        tantivy::schema::IndexRecordOption::Basic,
    ))
}

#[test]
fn term_counts_do_not_need_frequencies_or_positions() {
    let (index, _, literal, _, _) = fixture(true);
    assert_eq!(
        estimate_query(&index, term(literal, "red cat")),
        Some((1, 1))
    );
    assert_eq!(
        estimate_query(&index, term(literal, "absent")),
        Some((0, 0))
    );
}

#[test]
fn fast_ranges_and_string_endpoints() {
    let (index, _, _, number, label) = fixture(true);
    for (lower, upper, count) in [
        (Bound::Included(20), Bound::Excluded(40), 2),
        (Bound::Excluded(20), Bound::Included(40), 2),
        (Bound::Unbounded, Bound::Excluded(10), 0),
        (Bound::Included(41), Bound::Unbounded, 0),
    ] {
        assert_eq!(
            estimate_query(
                &index,
                Box::new(RangeQuery::new(
                    lower.map(|v| Term::from_field_u64(number, v)),
                    upper.map(|v| Term::from_field_u64(number, v))
                ))
            ),
            Some((count, 4))
        );
    }
    for (lower, upper, count) in [
        (Bound::Included("banana"), Bound::Excluded("date"), 2),
        (Bound::Excluded("banana"), Bound::Included("date"), 2),
        (
            Bound::Included("blueberry"),
            Bound::Included("cranberry"),
            1,
        ),
        (Bound::Included("absent"), Bound::Included("absent"), 0),
    ] {
        assert_eq!(
            estimate_query(
                &index,
                Box::new(FastFieldRangeQuery::new(
                    lower.map(|v| Term::from_field_text(label, v)),
                    upper.map(|v| Term::from_field_text(label, v))
                ))
            ),
            Some((count, 4))
        );
    }
}

#[test]
fn nested_wrappers_and_parsed_queries_use_native_estimates() {
    let (index, text, _, number, _) = fixture(true);
    let query = BooleanQuery::new(vec![
        (Occur::Must, term(text, "red")),
        (
            Occur::Must,
            Box::new(RangeQuery::new(
                Bound::Included(Term::from_field_u64(number, 20)),
                Bound::Unbounded,
            )),
        ),
    ]);
    let plain = estimate_query(&index, Box::new(query.clone()));
    let wrapped = ConstScoreQuery::new(BoostQuery::new(ConstScoreQuery::new(query, 7.0), 2.0), 9.0);
    assert_eq!(estimate_query(&index, Box::new(wrapped)), plain);
    let parser = QueryParser::for_index(&index, vec![text]);
    for expression in [
        "red",
        "red AND cat",
        "red OR blue",
        "\"red cat\"",
        "red AND NOT dog",
        "number:[20 TO 40]",
    ] {
        assert!(
            estimate_query(&index, parser.parse_query(expression).unwrap()).is_some(),
            "{expression}"
        );
    }
}

#[test]
fn boolean_minimum_should_match_and_exclusions() {
    let half = Estimate {
        fraction: 0.5,
        work: 50,
    };
    for (needed, expected) in [(1, 0.875), (2, 0.5), (3, 0.125), (4, 0.0)] {
        let result = combine(vec![(Occur::Should, half); 3], needed, 100);
        assert_eq!(result.fraction, expected);
        assert_eq!(result.work, 150);
    }
    assert_eq!(
        combine([(Occur::Must, half), (Occur::MustNot, half)], 0, 100).fraction,
        0.25
    );
    assert_eq!(
        combine([(Occur::Must, half), (Occur::Should, half)], 0, 100).fraction,
        0.5
    );
    assert_eq!(combine([(Occur::MustNot, half)], 0, 100).fraction, 0.0);
}

#[test]
fn every_native_leaf_has_an_estimate() {
    let (index, text, _, number, label) = fixture(true);
    let words = || {
        vec![
            Term::from_field_text(text, "red"),
            Term::from_field_text(text, "cat"),
        ]
    };
    let queries: Vec<Box<dyn Query>> = vec![
        Box::new(AllQuery),
        Box::new(EmptyQuery),
        Box::new(TermSetQuery::new(words())),
        Box::new(DisjunctionMaxQuery::new(vec![
            term(text, "red"),
            term(text, "cat"),
        ])),
        Box::new(PhraseQuery::new(words())),
        Box::new(PhrasePrefixQuery::new(words())),
        Box::new(PhrasePrefixQuery::new(vec![Term::from_field_text(
            text, "ca",
        )])),
        Box::new(RegexQuery::from_pattern("c.*", text).unwrap()),
        Box::new(RegexPhraseQuery::new(
            text,
            vec!["r.*".into(), "c.*".into()],
        )),
        Box::new(FuzzyTermQuery::new(
            Term::from_field_text(text, "cot"),
            1,
            true,
        )),
        Box::new(ExistsQuery::new("number".into(), false)),
        Box::new(ExistsQuery::new("label".into(), false)),
        Box::new(InvertedIndexRangeQuery::new(
            Bound::Included(Term::from_field_text(text, "cat")),
            Bound::Included(Term::from_field_text(text, "dog")),
        )),
        Box::new(RangeQuery::new(
            Bound::Included(Term::from_field_u64(number, 20)),
            Bound::Unbounded,
        )),
        Box::new(FastFieldRangeQuery::new(
            Bound::Included(Term::from_field_text(label, "banana")),
            Bound::Unbounded,
        )),
        Box::new(crate::query::proximity::query::ProximityQuery::new(
            text,
            crate::query::proximity::ProximityClause::Term("red".into()),
            crate::query::proximity::ProximityDistance::AnyOrder(2),
            crate::query::proximity::ProximityClause::Term("cat".into()),
        )),
        Box::new(crate::query::score::ScoreFilter::new(
            vec![(Bound::Excluded(0.0), Bound::Unbounded)],
            term(text, "red"),
        )),
        Box::new(UnresolvedQuery),
    ];
    for query in queries {
        let display = format!("{query:?}");
        let (count, _) = estimate_query(&index, query).unwrap_or_else(|| panic!("{display}"));
        assert!(count <= 4, "{display}");
    }
}

#[test]
fn missing_statistics_disables_the_whole_new_path() {
    let (index, text, _, _, _) = fixture(false);
    assert_eq!(
        estimate_query(
            &index,
            Box::new(BooleanQuery::new(vec![
                (Occur::Should, term(text, "red")),
                (Occur::Should, Box::new(AllQuery))
            ]))
        ),
        None
    );
}

#[test]
fn arrays_and_json_types_count_documents() {
    let mut schema = Schema::builder();
    let values = schema.add_u64_field("values", FAST);
    let json = schema.add_json_field("json", FAST | TEXT);
    let index = Index::builder()
        .schema(schema.build())
        .register_plugin(std::sync::Arc::new(crate::index::stats::StatsPlugin))
        .create_in_ram()
        .unwrap();
    let mut writer: IndexWriter = index.writer_with_num_threads(1, 15_000_000).unwrap();
    for source in [
        r#"{"values":[1,1,2],"json":{"a":[1,2],"b":"x"}}"#,
        r#"{"values":[3],"json":{"a":"one"}}"#,
        r#"{"json":{"b":"y"}}"#,
    ] {
        writer
            .add_document(TantivyDocument::parse_json(&index.schema(), source).unwrap())
            .unwrap();
    }
    writer.commit().unwrap();
    assert_eq!(
        estimate_query(
            &index,
            Box::new(RangeQuery::new(
                Bound::Included(Term::from_field_u64(values, 1)),
                Bound::Included(Term::from_field_u64(values, 2))
            ))
        )
        .unwrap()
        .0,
        1
    );
    assert_eq!(
        estimate_query(&index, Box::new(ExistsQuery::new("json.a".into(), false)))
            .unwrap()
            .0,
        2
    );
    assert_eq!(
        estimate_query(&index, Box::new(ExistsQuery::new("json".into(), true)))
            .unwrap()
            .0,
        3
    );
    let mut bound = Term::from_field_json_path(json, "a", false);
    bound.append_type_and_fast_value(1u64);
    assert_eq!(
        estimate_query(
            &index,
            Box::new(FastFieldRangeQuery::new(
                Bound::Included(bound.clone()),
                Bound::Included(bound)
            ))
        )
        .unwrap()
        .0,
        1
    );
}

#[test]
fn broad_regex_stops_at_the_dictionary_budget() {
    let mut schema = Schema::builder();
    let text = schema.add_text_field("text", STRING);
    let _id = schema.add_u64_field("id", FAST | INDEXED);
    let index = Index::builder()
        .schema(schema.build())
        .register_plugin(std::sync::Arc::new(crate::index::stats::StatsPlugin))
        .create_in_ram()
        .unwrap();
    let mut writer: IndexWriter = index.writer_with_num_threads(1, 15_000_000).unwrap();
    for i in 0..5000 {
        writer
            .add_document(doc!(text => format!("word{i:05}")))
            .unwrap();
    }
    writer.commit().unwrap();
    assert_eq!(
        estimate_query(
            &index,
            Box::new(RegexQuery::from_pattern(".*", text).unwrap())
        ),
        Some((50, 5000))
    );
}

#[test]
fn missing_required_summary_does_not_produce_a_partial_estimate() {
    let mut schema = Schema::builder();
    let text = schema.add_text_field("text", STRING);
    let values = schema.add_u64_field("values", FAST);
    let index = Index::builder()
        .schema(schema.build())
        .register_plugin(std::sync::Arc::new(crate::index::stats::StatsPlugin))
        .create_in_ram()
        .unwrap();
    let mut writer: IndexWriter = index.writer_with_num_threads(1, 15_000_000).unwrap();
    let mut document = doc!(text => "red");
    for value in 0..20000 {
        document.add_u64(values, value);
    }
    writer.add_document(document).unwrap();
    writer.commit().unwrap();
    assert_eq!(estimate_query(&index, term(text, "red")), Some((1, 1)));
    let query = BooleanQuery::new(vec![
        (Occur::Must, term(text, "red")),
        (
            Occur::Must,
            Box::new(RangeQuery::new(
                Bound::Included(Term::from_field_u64(values, 1)),
                Bound::Unbounded,
            )),
        ),
    ]);
    assert_eq!(estimate_query(&index, Box::new(query)), None);
}

#[test]
fn estimation_does_not_build_weights() {
    #[derive(Debug, Clone)]
    struct MetadataOnly;
    impl Query for MetadataOnly {
        fn weight(&self, _: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
            panic!("estimation constructed a weight")
        }
        fn estimate_docs(&self, _: &SegmentReader) -> tantivy::Result<Option<(u32, u64)>> {
            Ok(Some((2, 3)))
        }
    }
    let (index, _, _, _, _) = fixture(true);
    assert_eq!(
        estimate_query(
            &index,
            Box::new(BoostQuery::new(
                ConstScoreQuery::new(MetadataOnly, 1.0),
                2.0
            ))
        ),
        Some((2, 3))
    );
}

#[test]
fn fast_range_value_types_preserve_their_order() {
    let mut schema = Schema::builder();
    let signed = schema.add_i64_field("signed", FAST);
    let float = schema.add_f64_field("float", FAST);
    let boolean = schema.add_bool_field("boolean", FAST);
    let date = schema.add_date_field("date", FAST);
    let ip = schema.add_ip_addr_field("ip", FAST);
    let bytes = schema.add_bytes_field("bytes", FAST);
    let facet = schema.add_facet_field("facet", tantivy::schema::FacetOptions::default());
    let index = Index::builder()
        .schema(schema.build())
        .register_plugin(std::sync::Arc::new(crate::index::stats::StatsPlugin))
        .create_in_ram()
        .unwrap();
    let mut writer: IndexWriter = index.writer_with_num_threads(1, 15_000_000).unwrap();
    for i in 0..4 {
        writer
            .add_document(doc!(signed => i - 2, float => i as f64 - 1.5,
            boolean => i % 2 == 0, date => tantivy::DateTime::from_timestamp_secs(i),
            ip => std::net::Ipv6Addr::from(i as u128), bytes => vec![i as u8],
            facet => tantivy::schema::Facet::from(&format!("/value{i}"))))
            .unwrap();
    }
    writer.commit().unwrap();
    for bound in [
        Term::from_field_i64(signed, 0),
        Term::from_field_f64(float, 0.5),
        Term::from_field_date(date, tantivy::DateTime::from_timestamp_secs(2)),
        Term::from_field_ip_addr(ip, std::net::Ipv6Addr::from(2)),
        Term::from_field_bytes(bytes, &[2]),
    ] {
        assert_eq!(
            estimate_query(
                &index,
                Box::new(RangeQuery::new(Bound::Included(bound), Bound::Unbounded))
            )
            .unwrap()
            .0,
            2
        );
    }
    let bound = Term::from_field_bool(boolean, true);
    assert_eq!(
        estimate_query(
            &index,
            Box::new(RangeQuery::new(
                Bound::Included(bound.clone()),
                Bound::Included(bound)
            ))
        )
        .unwrap()
        .0,
        2
    );
    let bound = Term::from_facet(facet, &tantivy::schema::Facet::from("/value2"));
    assert_eq!(
        estimate_query(
            &index,
            Box::new(RangeQuery::new(
                Bound::Included(bound.clone()),
                Bound::Included(bound)
            ))
        )
        .unwrap()
        .0,
        1
    );
}
