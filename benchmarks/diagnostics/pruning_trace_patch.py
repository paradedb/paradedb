"""Instrument only the separate diagnostic library, never the timed builds."""

from pathlib import Path

reader = Path('pg_search/src/index/reader/index.rs')
text = reader.read_text()
old = '''    fn is_candidate(&self, ord: SegmentOrdinal) -> bool {
        self.searcher.segment_reader(ord).num_docs() > 0
            && self
                .pruning_query
                .as_ref()
                .is_none_or(|query| self.pruner().can_match(ord, query))
    }'''
new = '''    fn is_candidate(&self, ord: SegmentOrdinal) -> bool {
        let candidate = self.searcher.segment_reader(ord).num_docs() > 0
            && self
                .pruning_query
                .as_ref()
                .is_none_or(|query| self.pruner().can_match(ord, query));
        eprintln!("PDB_PRUNING_TRACE pid={} segment={:?} num_docs={} eligible={} candidate={}",
            std::process::id(), self.searcher.segment_reader(ord).segment_id(),
            self.searcher.segment_reader(ord).num_docs(), self.pruning_query.is_some(), candidate);
        candidate
    }'''
assert text.count(old) == 1
reader.write_text(text.replace(old, new))
scorer = Path('pg_search/src/index/reader/scorer.rs')
text = scorer.read_text()
old = '        self.scorer.get_or_init(|| {\n'
new = old + '''            eprintln!("PDB_SCORER_TRACE pid={} segment={:?}",
                std::process::id(), self.segment_reader.segment_id());
'''
assert text.count(old) == 1
scorer.write_text(text.replace(old, new))
