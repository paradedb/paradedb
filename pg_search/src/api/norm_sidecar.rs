use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::index::reader::segment_component::SegmentComponentReader;
use crate::index::writer::segment_component::SegmentComponentWriter;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::FileEntry;
use anyhow::{Context, Result, ensure};
use pgrx::prelude::*;
use pgrx::{GucContext, GucFlags, GucRegistry, GucSetting, JsonB, PgRelation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::ffi::CString;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Weak};
use tantivy::directory::{FileHandle, FileSlice, TerminatingWrite};
use tantivy::index::{SegmentComponent, SegmentId};
use tantivy::schema::IndexRecordOption;
use tantivy::{DocSet, TERMINATED, Term};

const MAX_POSTINGS: usize = 20_000_000;
const MAX_MANIFEST_BYTES: usize = 1_048_576;
static SIDECAR: GucSetting<Option<CString>> = GucSetting::<Option<CString>>::new(None);

#[derive(Clone, Deserialize, Serialize)]
struct Descriptor {
    auxiliary_index: u32,
    manifest: FileEntry,
}

#[derive(Deserialize, Serialize)]
struct Lane {
    segment: String,
    field: u32,
    term: String,
    start: usize,
    len: usize,
    postings_start: usize,
    postings_end: usize,
}

#[derive(Deserialize, Serialize)]
struct Manifest {
    format: u32,
    source_index: u32,
    data: FileEntry,
    lanes: Vec<Lane>,
}

type LaneKey = (String, u32, Vec<u8>);

struct QueryState {
    descriptor: Descriptor,
    source_index: u32,
    manifest: Option<Manifest>,
    lanes: HashMap<LaneKey, (usize, usize)>,
    data: Option<Weak<dyn FileHandle>>,
    hits: usize,
    misses: usize,
    manifest_reads: usize,
}

thread_local! {
    static STATE: RefCell<Option<QueryState>> = const { RefCell::new(None) };
}

pub(crate) fn register_guc() {
    GucRegistry::define_string_guc(
        c"paradedb.experiment_norm_sidecar",
        c"PG-backed exact posting norm sidecar descriptor for diagnostics.",
        c"Empty disables the experiment. The auxiliary relation must remain immutable.",
        &SIDECAR,
        GucContext::Suset,
        GucFlags::default(),
    );
}

pub(crate) fn reset_for_query(source_index: pg_sys::Oid) -> Result<()> {
    tantivy::postings::set_norm_sidecar_provider(None);
    STATE.with(|state| *state.borrow_mut() = None);
    let Some(setting) = SIDECAR.get() else {
        return Ok(());
    };
    let value = setting.to_str()?;
    if value.is_empty() {
        return Ok(());
    }
    ensure!(
        unsafe { pg_sys::superuser() },
        "norm sidecar requires superuser"
    );
    ensure!(value.len() <= 4096, "sidecar descriptor is too large");
    let descriptor: Descriptor = serde_json::from_str(value)?;
    ensure!(
        descriptor.auxiliary_index != 0,
        "invalid auxiliary index OID"
    );
    ensure!(
        descriptor.manifest.total_bytes > 0
            && descriptor.manifest.total_bytes <= MAX_MANIFEST_BYTES,
        "invalid sidecar manifest length"
    );
    STATE.with(|state| {
        *state.borrow_mut() = Some(QueryState {
            descriptor,
            source_index: source_index.to_u32(),
            manifest: None,
            lanes: HashMap::new(),
            data: None,
            hits: 0,
            misses: 0,
            manifest_reads: 0,
        });
    });
    tantivy::postings::set_norm_sidecar_provider(Some(provide));
    Ok(())
}

fn open_component(index: &PgSearchRelation, entry: FileEntry) -> Result<Arc<dyn FileHandle>> {
    let blocks =
        unsafe { pg_sys::RelationGetNumberOfBlocksInFork(index.as_ptr(), index.fork_number()) };
    ensure!(
        entry.starting_block < blocks,
        "sidecar block is outside auxiliary relation"
    );
    ensure!(
        entry.total_bytes <= MAX_POSTINGS + MAX_MANIFEST_BYTES,
        "sidecar exceeds byte limit"
    );
    Ok(Arc::new(unsafe {
        SegmentComponentReader::new(index, entry, Some(SegmentComponent::FieldNorms))
    }))
}

impl QueryState {
    fn lookup(&mut self, segment: SegmentId, term: &Term) -> Result<Option<FileSlice>> {
        if self.manifest.is_none() {
            let index = PgSearchRelation::with_lock(
                pg_sys::Oid::from(self.descriptor.auxiliary_index),
                pg_sys::AccessShareLock as _,
            );
            index.schema()?;
            let bytes =
                FileSlice::new(open_component(&index, self.descriptor.manifest)?).read_bytes()?;
            let manifest: Manifest = serde_json::from_slice(bytes.as_slice())?;
            ensure!(manifest.format == 1, "unsupported norm sidecar format");
            ensure!(
                manifest.data.total_bytes <= MAX_POSTINGS,
                "sidecar exceeds posting limit"
            );
            ensure!(manifest.lanes.len() <= 65_536, "sidecar has too many lanes");
            for lane in &manifest.lanes {
                ensure!(
                    lane.start
                        .checked_add(lane.len)
                        .is_some_and(|end| end <= manifest.data.total_bytes),
                    "sidecar lane is out of bounds"
                );
                let key = (
                    lane.segment.clone(),
                    lane.field,
                    lane.term.as_bytes().to_vec(),
                );
                ensure!(
                    self.lanes.insert(key, (lane.start, lane.len)).is_none(),
                    "duplicate sidecar lane"
                );
            }
            self.manifest = Some(manifest);
            self.manifest_reads += 1;
        }
        let manifest = self.manifest.as_ref().unwrap();
        let key = (
            segment.uuid_string(),
            term.field().field_id(),
            term.serialized_value_bytes().to_vec(),
        );
        let lane = self
            .lanes
            .get(&key)
            .copied()
            .filter(|_| manifest.source_index == self.source_index);
        let Some((start, len)) = lane else {
            self.misses += 1;
            return Ok(None);
        };
        let data = match self.data.as_ref().and_then(Weak::upgrade) {
            Some(data) => data,
            None => {
                let index = PgSearchRelation::with_lock(
                    pg_sys::Oid::from(self.descriptor.auxiliary_index),
                    pg_sys::AccessShareLock as _,
                );
                let data = open_component(&index, manifest.data)?;
                self.data = Some(Arc::downgrade(&data));
                data
            }
        };
        self.hits += 1;
        Ok(Some(FileSlice::new(data).slice(start..start + len)))
    }
}

fn provide(segment: SegmentId, term: &Term) -> Option<FileSlice> {
    STATE.with(|state| {
        state.borrow_mut().as_mut().and_then(|state| {
            state
                .lookup(segment, term)
                .unwrap_or_else(|error| panic!("norm sidecar: {error:#}"))
        })
    })
}

#[pg_extern]
fn norm_sidecar_status() -> JsonB {
    STATE.with(|state| {
        let state = state.borrow();
        JsonB(match state.as_ref() {
            Some(state) => json!({"enabled": true, "hits": state.hits, "misses": state.misses,
                "manifest_reads": state.manifest_reads, "auxiliary_index": state.descriptor.auxiliary_index}),
            None => json!({"enabled": false}),
        })
    })
}

#[pg_extern]
fn norm_sidecar_build(
    source_index: PgRelation,
    auxiliary_index: PgRelation,
    field_name: &str,
    terms: Vec<String>,
) -> Result<JsonB> {
    ensure!(
        unsafe { pg_sys::superuser() },
        "norm_sidecar_build requires superuser"
    );
    ensure!(
        source_index.oid() != auxiliary_index.oid(),
        "use a separate auxiliary BM25 index"
    );
    ensure!(
        !terms.is_empty() && terms.len() <= 32,
        "supply 1 to 32 literal analyzed terms"
    );
    ensure!(
        terms
            .iter()
            .all(|term| !term.is_empty() && term.len() <= 4096),
        "invalid literal term"
    );
    let terms: BTreeSet<_> = terms.into_iter().collect();
    let source = PgSearchRelation::with_lock(source_index.oid(), pg_sys::AccessShareLock as _);
    let auxiliary =
        PgSearchRelation::with_lock(auxiliary_index.oid(), pg_sys::AccessExclusiveLock as _);
    auxiliary.schema()?;
    let reader = SearchIndexReader::empty(&source, MvccSatisfies::Snapshot)?;
    let field = reader.schema().tantivy_schema().get_field(field_name)?;
    let mut writer =
        unsafe { SegmentComponentWriter::new(&auxiliary, Path::new("norm-sidecar.idx")) };
    let mut lanes = Vec::new();
    let mut total = 0usize;
    for segment in reader.segment_readers() {
        pgrx::check_for_interrupts!();
        let norms = segment
            .fieldnorms_readers()
            .get_inner_file()
            .open_read(field)
            .context("field has no fieldnorm data")?
            .read_bytes()?;
        ensure!(
            norms.len() == segment.max_doc() as usize,
            "unexpected fieldnorm length"
        );
        let inverted = segment.inverted_index(field)?;
        for text in &terms {
            let term = Term::from_field_text(field, text);
            let Some(info) = inverted.get_term_info(&term)? else {
                continue;
            };
            ensure!(
                total + info.doc_freq as usize <= MAX_POSTINGS,
                "sidecar exceeds 20 million postings"
            );
            let mut postings =
                inverted.read_postings_from_terminfo(&info, IndexRecordOption::WithFreqs)?;
            let mut lane = Vec::with_capacity(info.doc_freq as usize);
            while postings.doc() != TERMINATED {
                lane.push(norms[postings.doc() as usize]);
                if lane.len() % 16384 == 0 {
                    pgrx::check_for_interrupts!();
                }
                postings.advance();
            }
            ensure!(
                lane.len() == info.doc_freq as usize,
                "posting count mismatch"
            );
            writer.write_all(&lane)?;
            lanes.push(Lane {
                segment: segment.segment_id().uuid_string(),
                field: field.field_id(),
                term: text.clone(),
                start: total,
                len: lane.len(),
                postings_start: info.postings_range.start,
                postings_end: info.postings_range.end,
            });
            total += lane.len();
        }
    }
    ensure!(total > 0, "selected terms have no postings");
    let data = writer.file_entry();
    writer.terminate()?;
    let lane_count = lanes.len();
    let manifest = Manifest {
        format: 1,
        source_index: source.oid().to_u32(),
        data,
        lanes,
    };
    let bytes = serde_json::to_vec(&manifest)?;
    ensure!(
        bytes.len() <= MAX_MANIFEST_BYTES,
        "sidecar manifest exceeds one MiB"
    );
    let mut writer =
        unsafe { SegmentComponentWriter::new(&auxiliary, Path::new("norm-sidecar-manifest.idx")) };
    writer.write_all(&bytes)?;
    let manifest_entry = writer.file_entry();
    writer.terminate()?;
    let descriptor = Descriptor {
        auxiliary_index: auxiliary.oid().to_u32(),
        manifest: manifest_entry,
    };
    Ok(JsonB(
        json!({"descriptor": descriptor, "data": data, "norm_bytes": total,
        "lanes": lane_count, "manifest_bytes": bytes.len(), "source_index": source.oid().to_u32(),
        "auxiliary_index": auxiliary.oid().to_u32()}),
    ))
}
