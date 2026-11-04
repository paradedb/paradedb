"""Instrument a temporary source checkout for cache attribution."""

import json
from pathlib import Path

source = Path("source")
module = source / "pg_search/src/postgres/storage/mutable_cache/mod.rs"
mvcc = source / "pg_search/src/index/directory/mvcc.rs"
names = [
    "exact_opens",
    "exact_hits",
    "prefix_chunks",
    "prefix_docs",
    "private_fallbacks",
    "materializations",
    "ctids",
    "fetched_docs",
    "materialize_us",
    "fetch_us",
    "pack_us",
    "packed_bytes",
    "tail_builds",
    "tail_ctids",
    "arena_full",
    "slot_full",
]
Path("counter-names.json").write_text(json.dumps(names), encoding="utf-8")
s = module.read_text(encoding="utf-8")
needle = "static MUTABLE_CACHE:"
assert s.count(needle) == 1
s = s.replace(
    needle,
    (
        "pub(crate) static DIAGNOSTIC_COUNTERS: [AtomicU64; 16] = [co"
        "nst { AtomicU64::new(0) }; 16];\n\n"
    )
    + needle,
)
s = s.replace("fn register_backend_exit()", "pub(crate) fn register_backend_exit()")
needle = 'unsafe extern "C-unwind" fn release_backend_resources(_code: i32, _arg: pg_sys::Datum) {'
assert s.count(needle) == 1
s = s.replace(
    needle,
    needle
    + (
        '\n    pgrx::log!("CACHE_DIAG pid={} counters={:?}", unsafe { '
        "pg_sys::MyProcPid }, DIAGNOSTIC_COUNTERS.each_ref().map(|v| "
        "v.load(Ordering::Relaxed)));"
    ),
)
pos = s.index(
    "    if !crate::gucs::enable_mutable_segment_cache() {",
    s.index("pub fn get_or_build("),
)
s = s[:pos] + "    DIAGNOSTIC_COUNTERS[0].fetch_add(1, Ordering::Relaxed);\n" + s[pos:]
needle = "    chunks\n}\n\n/// Reuse an exact range"
assert s.count(needle) == 1
s = s.replace(
    needle,
    (
        "    DIAGNOSTIC_COUNTERS[2].fetch_add(chunks.len() as u64, Or"
        "dering::Relaxed);\n    DIAGNOSTIC_COUNTERS[3].fetch_add(start"
        " as u64, Ordering::Relaxed);\n"
    )
    + needle,
)
needle = "                return Ok(directory);"
assert s.count(needle) == 1
s = s.replace(
    needle,
    "                DIAGNOSTIC_COUNTERS[1].fetch_add(1, Ordering::Relaxed);\n"
    + needle,
)
needle = "        if in_flight_building {\n            drop(guard);"
assert s.count(needle) == 1
s = s.replace(
    needle,
    (
        "        if in_flight_building {\n            DIAGNOSTIC_COUNT"
        "ERS[4].fetch_add(1, Ordering::Relaxed);\n            drop(gua"
        "rd);"
    ),
)
needle = "        let packed_bytes = match pack::pack_segment"
assert s.count(needle) == 1
s = s.replace(
    needle, "        let diag_pack_start = std::time::Instant::now();\n" + needle
)
needle = "        let span = match cache.allocate(packed_bytes.len(), &guard) {"
assert s.count(needle) == 1
s = s.replace(
    needle,
    (
        "        DIAGNOSTIC_COUNTERS[10].fetch_add(diag_pack_start.el"
        "apsed().as_micros() as u64, Ordering::Relaxed);\n        DIAG"
        "NOSTIC_COUNTERS[11].fetch_add(packed_bytes.len() as u64, Ord"
        "ering::Relaxed);\n"
    )
    + needle,
)
for counter, message in [(14, "arena full"), (15, "slot table full")]:
    warning = (
        "pgrx::warning!(\n"
        + (" " * (20 if counter == 14 else 16))
        + '"pg_search: mutable segment cache '
        + message
    )
    assert s.count(warning) == 1
    s = s.replace(
        warning,
        f"DIAGNOSTIC_COUNTERS[{counter}].fetch_add(1, Ordering::Relaxed);\n" + warning,
    )
module.write_text(s, encoding="utf-8")
s = mvcc.read_text(encoding="utf-8")
pos = s.index(
    "    let directory = RamDirectory::create();",
    s.index("pub fn index_memory_segment("),
)
s = (
    s[:pos]
    + (
        "    crate::postgres::storage::mutable_cache::register_backen"
        "d_exit();\n    let diag_start = std::time::Instant::now();\n  "
        "  let mut diag_fetched = 0u64;\n    let mut diag_fetch_us = 0"
        "u64;\n"
    )
    + s[pos:]
)
needle = "                let doc = fetcher.fetch_doc(ctid).unwrap_or_default();"
assert s.count(needle) == 1
s = s.replace(
    needle,
    (
        "                let diag_fetch_start = std::time::Instant::n"
        "ow();\n                let fetched = fetcher.fetch_doc(ctid);"
        "\n                diag_fetch_us += diag_fetch_start.elapsed()"
        ".as_micros() as u64;\n                diag_fetched += u64::fr"
        "om(fetched.is_some());\n                let doc = fetched.unw"
        "rap_or_default();"
    ),
)
needle = '    Ok(directory)\n}\n\n#[cfg(any(test, feature = "pg_test"))]'
assert s.count(needle) == 1
s = s.replace(
    needle,
    """    let d = &crate::postgres::storage::mutable_cache::DIAGNOSTIC_COUNTERS;
    let o = std::sync::atomic::Ordering::Relaxed;
    d[5].fetch_add(1, o);
    d[6].fetch_add(ctids.len() as u64, o);
    d[7].fetch_add(diag_fetched, o);
    d[8].fetch_add(diag_start.elapsed().as_micros() as u64, o);
    d[9].fetch_add(diag_fetch_us, o);
    if range.is_some_and(|r| r.start > 0) {
        d[12].fetch_add(1, o);
        d[13].fetch_add(ctids.len() as u64, o);
    }
"""
    + needle,
)
mvcc.write_text(s, encoding="utf-8")
