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

use pgrx::pg_sys;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ptr::addr_of;
use std::rc::Rc;
use tantivy::index::SegmentComponent;

type SharedData = Rc<RefCell<Data>>;

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
struct Counts {
    hits: u64,
    reads: u64,
}

impl Counts {
    fn since(self, before: Self) -> Self {
        Self {
            hits: self.hits.saturating_sub(before.hits),
            reads: self.reads.saturating_sub(before.reads),
        }
    }

    fn add(&mut self, other: Self) {
        self.hits += other.hits;
        self.reads += other.reads;
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Data {
    total: Counts,
    components: BTreeMap<String, Counts>,
    workers: BTreeMap<usize, Data>,
    count_segments: BTreeMap<String, u64>,
}

impl Data {
    fn values(&self, select: impl Fn(Counts) -> u64) -> Vec<(String, u64)> {
        let total = select(self.total);
        let mut values = vec![("Total".into(), total)];
        values.extend(self.components.iter().filter_map(|(component, counts)| {
            let value = select(*counts);
            (value > 0).then(|| (component.clone(), value))
        }));
        let other = total.saturating_sub(self.components.values().copied().map(select).sum());
        if other > 0 {
            values.push(("Other".into(), other));
        }
        values
    }

    pub fn hits(&self) -> Vec<(String, u64)> {
        self.values(|counts| counts.hits)
    }

    pub fn reads(&self) -> Vec<(String, u64)> {
        self.values(|counts| counts.reads)
    }

    pub fn count_segments(&self) -> Vec<(String, u64)> {
        self.count_segments
            .iter()
            .map(|(label, count)| (label.clone(), *count))
            .collect()
    }

    fn merge(&mut self, other: Self) {
        self.total.add(other.total);
        for (component, counts) in other.components {
            self.components.entry(component).or_default().add(counts);
        }
        for (worker, data) in other.workers {
            self.workers.entry(worker).or_default().merge(data);
        }
        for (label, count) in other.count_segments {
            *self.count_segments.entry(label).or_default() += count;
        }
    }
}

#[derive(Clone)]
struct Context {
    data: SharedData,
    component: String,
    columnar: &'static str,
}

thread_local! {
    static ACTIVE: RefCell<Option<Context>> = const { RefCell::new(None) };
}

#[derive(Default)]
pub struct Trace(SharedData);

pub struct Scope {
    previous: Option<Context>,
    total: Option<(SharedData, Counts)>,
}

pub struct External {
    context: Option<Context>,
    before: Counts,
    name: &'static str,
}

impl Drop for External {
    fn drop(&mut self) {
        if let Some(context) = &self.context {
            context
                .data
                .borrow_mut()
                .components
                .entry(self.name.into())
                .or_default()
                .add(snapshot().since(self.before));
        }
        ACTIVE.set(self.context.take());
    }
}

pub fn external(name: &'static str) -> External {
    External {
        context: ACTIVE.take(),
        before: snapshot(),
        name,
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        if let Some((data, before)) = &self.total {
            data.borrow_mut().total.add(snapshot().since(*before));
        }
        ACTIVE.set(self.previous.take());
    }
}

fn snapshot() -> Counts {
    unsafe {
        let usage = addr_of!(pg_sys::pgBufferUsage).read();
        Counts {
            hits: usage.shared_blks_hit as u64,
            reads: usage.shared_blks_read as u64,
        }
    }
}

impl Trace {
    pub fn enter(&self) -> Scope {
        let previous = ACTIVE.replace(Some(Context {
            data: self.0.clone(),
            component: "Metadata".into(),
            columnar: "Columnar Fields",
        }));
        Scope {
            previous,
            total: Some((self.0.clone(), snapshot())),
        }
    }

    pub fn hits(&self) -> Vec<(String, u64)> {
        self.0.borrow().hits()
    }

    pub fn reads(&self) -> Vec<(String, u64)> {
        self.0.borrow().reads()
    }

    pub fn data(&self) -> Data {
        self.0.borrow().clone()
    }

    pub fn count_segments(&self) -> Vec<(String, u64)> {
        self.0.borrow().count_segments()
    }

    pub fn workers(&self) -> BTreeMap<usize, Data> {
        self.0.borrow().workers.clone()
    }
}

pub fn add_worker(worker: usize, data: Data) {
    ACTIVE.with_borrow(|active| {
        if let Some(context) = active {
            context
                .data
                .borrow_mut()
                .workers
                .entry(worker)
                .or_default()
                .merge(data);
        }
    });
}

pub fn count_segment(all_visible: bool) {
    ACTIVE.with_borrow(|active| {
        if let Some(context) = active {
            let label = if all_visible {
                "All Visible"
            } else {
                "MVCC Checked"
            };
            *context
                .data
                .borrow_mut()
                .count_segments
                .entry(label.into())
                .or_default() += 1;
        }
    });
}

pub fn columnar(name: &'static str) -> Scope {
    let previous = ACTIVE.with_borrow_mut(|active| {
        let previous = active.clone();
        if let Some(context) = active {
            context.columnar = name;
        }
        previous
    });
    Scope {
        previous,
        total: None,
    }
}

pub fn buffer<R>(read: impl FnOnce() -> R) -> R {
    let context = ACTIVE.with_borrow(Clone::clone);
    let Some(context) = context else {
        return read();
    };
    let before = snapshot();
    let result = read();
    context
        .data
        .borrow_mut()
        .components
        .entry(context.component)
        .or_default()
        .add(snapshot().since(before));
    result
}

pub fn file_read(component: &SegmentComponent) -> Scope {
    let previous = ACTIVE.with_borrow_mut(|active| {
        let previous = active.clone();
        if let Some(context) = active {
            context.component = match component.to_string().as_str() {
                "idx" => "Postings",
                "pos" => "Positions",
                "term" => "Term Dictionary",
                "fieldnorm" => "Field Norms",
                "pnorm" => "Posting Norms",
                "bpnorm" => "Packed Posting Norms",
                "fast" => context.columnar,
                "store" => "Document Store",
                "temp" => "Temporary Store",
                "del" => "Liveness Bitmap",
                "stats" => "Segment Statistics",
                "vec" => "Vectors",
                "centroids" => "Centroids",
                other => other,
            }
            .to_owned();
        }
        previous
    });
    Scope {
        previous,
        total: None,
    }
}
