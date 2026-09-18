use crate::index::fast_fields_helper::FFType;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::bm25_max_free_space;
use anyhow::{Context, Result, ensure};
use pgrx::prelude::*;
use pgrx::{JsonB, PgRelation};
use serde_json::{Value, json};
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::Path;
use tantivy::directory::{CompositeFile, FileSlice};
use tantivy::index::SegmentComponent;
use tantivy::postings::Postings;
use tantivy::query::{Bm25StatisticsProvider, Bm25Weight};
use tantivy::schema::IndexRecordOption;
use tantivy::{DocSet, HasLen, TERMINATED, Term};

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    Ok(())
}

fn export_footer(file: &FileSlice, path: &Path) -> Result<usize> {
    let len = file.len();
    let tail = file.read_bytes_slice(len - 4..len)?;
    let footer_len = u32::from_le_bytes(tail.as_slice().try_into()?) as usize;
    let footer = file.read_bytes_slice(len - 4 - footer_len..len)?;
    write_new(path, footer.as_slice())?;
    Ok(len)
}

#[pg_extern]
fn postings_oracle(
    index: PgRelation,
    field_name: &str,
    terms: Vec<String>,
    output_directory: &str,
    segment_ordinal: default!(Option<i32>, "NULL"),
) -> Result<JsonB> {
    ensure!(
        unsafe { pg_sys::superuser() },
        "postings_oracle requires superuser"
    );
    ensure!(
        !terms.is_empty() && terms.len() <= 32,
        "supply 1 to 32 literal analyzed terms"
    );
    ensure!(
        terms.iter().all(|term| !term.is_empty()),
        "terms cannot be empty"
    );
    let directory = Path::new(output_directory);
    ensure!(directory.is_absolute(), "output directory must be absolute");
    std::fs::create_dir_all(directory)?;
    ensure!(
        !directory.join("manifest.json").exists(),
        "export already exists"
    );
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    let reader = SearchIndexReader::empty(&index, MvccSatisfies::Snapshot)?;
    let field = reader.schema().tantivy_schema().get_field(field_name)?;
    let searcher = reader.searcher();
    let term_objects: Vec<_> = terms
        .iter()
        .map(|text| Term::from_field_text(field, text))
        .collect();
    let weights: Vec<_> = term_objects
        .iter()
        .map(|term| Bm25Weight::for_terms(searcher, std::slice::from_ref(term)))
        .collect::<tantivy::Result<_>>()?;
    let global_doc_freqs: Vec<_> = term_objects
        .iter()
        .map(|term| searcher.doc_freq(term))
        .collect::<tantivy::Result<_>>()?;
    let mut segments: Vec<Value> = Vec::new();
    let mut total_postings = 0usize;

    for (ordinal, segment) in reader.segment_readers().iter().enumerate() {
        if segment_ordinal.is_some_and(|requested| requested != ordinal as i32) {
            continue;
        }
        pgrx::check_for_interrupts!();
        let stem = format!("segment-{ordinal}");
        let norm_file = segment
            .fieldnorms_readers()
            .get_inner_file()
            .open_read(field)
            .context("field has no fieldnorm data")?;
        let norms = norm_file.read_bytes()?;
        ensure!(
            norms.len() == segment.max_doc() as usize,
            "unexpected fieldnorm length"
        );
        write_new(&directory.join(format!("{stem}.norms")), norms.as_slice())?;
        let norms_component = segment.open_read(SegmentComponent::FieldNorms)?;
        let norms_component_len = export_footer(
            &norms_component,
            &directory.join(format!("{stem}.norms-footer")),
        )?;
        let postings_component = segment.open_read(SegmentComponent::Postings)?;
        let postings_component_len = export_footer(
            &postings_component,
            &directory.join(format!("{stem}.postings-footer")),
        )?;
        let postings_file = CompositeFile::open(&postings_component)?
            .open_read(field)
            .context("field has no postings data")?
            .slice_from(8);
        let inverted = segment.inverted_index(field)?;
        let ctids = FFType::new_ctid(segment.fast_fields());
        let mut term_entries = Vec::new();

        for (term_index, term) in term_objects.iter().enumerate() {
            let Some(info) = inverted.get_term_info(term)? else {
                term_entries.push(json!({"term_index": term_index, "doc_freq": 0}));
                continue;
            };
            total_postings += info.doc_freq as usize;
            ensure!(
                total_postings <= 20_000_000,
                "export exceeds 20 million postings; select fewer terms or one segment"
            );
            let term_stem = format!("{stem}-term-{term_index}");
            let compressed = postings_file.read_bytes_slice(info.postings_range.clone())?;
            write_new(
                &directory.join(format!("{term_stem}.raw")),
                compressed.as_slice(),
            )?;
            let records_file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(directory.join(format!("{term_stem}.records")))?;
            let mut records = BufWriter::with_capacity(1024 * 1024, records_file);
            let mut postings =
                inverted.read_postings_from_terminfo(&info, IndexRecordOption::WithFreqs)?;
            let mut count = 0usize;
            while postings.doc() != TERMINATED {
                let doc = postings.doc();
                let freq = postings.term_freq();
                let norm = norms[doc as usize];
                let ctid = ctids.as_u64(doc).context("missing CTID")?;
                let score = weights[term_index].score(norm, freq);
                records.write_all(&doc.to_le_bytes())?;
                records.write_all(&freq.to_le_bytes())?;
                records.write_all(&ctid.to_le_bytes())?;
                records.write_all(&score.to_le_bytes())?;
                records.write_all(&[norm, u8::from(!segment.is_deleted(doc)), 0, 0])?;
                count += 1;
                if count % 16384 == 0 {
                    pgrx::check_for_interrupts!();
                }
                postings.advance();
            }
            records.flush()?;
            ensure!(count == info.doc_freq as usize, "posting count mismatch");
            term_entries.push(json!({
                "term_index": term_index,
                "doc_freq": info.doc_freq,
                "postings_start": info.postings_range.start,
                "postings_end": info.postings_range.end,
                "positions_start": info.positions_range.start,
                "positions_end": info.positions_range.end,
                "stem": term_stem,
            }));
        }
        segments.push(json!({
            "ordinal": ordinal, "id": segment.segment_id().uuid_string(), "stem": stem,
            "max_doc": segment.max_doc(), "num_docs": segment.num_docs(),
            "postings_component_len": postings_component_len, "norms_component_len": norms_component_len,
            "total_tokens": inverted.total_num_tokens(), "terms": term_entries,
        }));
    }
    ensure!(!segments.is_empty(), "no selected segment");
    let manifest = json!({
        "format": 1, "record_format": "little-endian u32 doc, u32 tf, u64 ctid, f32 score, u8 norm, u8 alive, u16 padding",
        "field": field_name, "field_id": field.field_id(), "terms": terms,
        "record_option": format!("{:?}", searcher.schema().get_field_entry(field).field_type().get_index_record_option()),
        "global_doc_freqs": global_doc_freqs, "global_num_docs": searcher.total_num_docs()?,
        "global_total_tokens": searcher.total_num_tokens(field)?, "segments": segments,
        "storage_page_bytes": pg_sys::BLCKSZ, "storage_payload_bytes": bm25_max_free_space(),
        "total_postings": total_postings,
    });
    write_new(
        &directory.join("manifest.json"),
        &serde_json::to_vec_pretty(&manifest)?,
    )?;
    Ok(JsonB(manifest))
}
