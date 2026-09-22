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
use serde::Serialize;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

type Page = (u32, i32, u32);
type SharedData = Rc<RefCell<Data>>;

#[derive(Default, Serialize)]
struct Counters {
    hits: u64,
    reads: u64,
}

impl Counters {
    fn add(&mut self, before: (i64, i64), after: (i64, i64)) {
        self.hits += after.0.saturating_sub(before.0) as u64;
        self.reads += after.1.saturating_sub(before.1) as u64;
    }
}

#[derive(Default)]
struct Data {
    total: Counters,
    file_reads: BTreeMap<String, u64>,
    buckets: BTreeMap<String, Counters>,
    pages: BTreeMap<String, BTreeSet<Page>>,
}

#[derive(Clone)]
struct Context {
    data: SharedData,
    phase: &'static str,
    component: String,
    storage: &'static str,
}

thread_local! {
    static ACTIVE: RefCell<Option<Context>> = const { RefCell::new(None) };
}

#[derive(Default)]
pub struct Trace(SharedData);

pub struct Scope {
    previous: Option<Context>,
    total: Option<(SharedData, (i64, i64))>,
}

pub struct External {
    context: Option<Context>,
    before: (i64, i64),
    name: &'static str,
}

impl Drop for External {
    fn drop(&mut self) {
        if let Some(context) = &self.context {
            context
                .data
                .borrow_mut()
                .buckets
                .entry(self.name.into())
                .or_default()
                .add(self.before, snapshot());
        }
    }
}

pub fn external(name: &'static str) -> External {
    External {
        context: ACTIVE.with_borrow(Clone::clone),
        before: snapshot(),
        name,
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        if let Some((data, before)) = &self.total {
            data.borrow_mut().total.add(*before, snapshot());
        }
        ACTIVE.set(self.previous.take());
    }
}

fn snapshot() -> (i64, i64) {
    unsafe {
        let usage = std::ptr::addr_of!(pg_sys::pgBufferUsage).read();
        (usage.shared_blks_hit, usage.shared_blks_read)
    }
}

impl Trace {
    pub fn enter(&self) -> Scope {
        let previous = ACTIVE.replace(Some(Context {
            data: self.0.clone(),
            phase: "execute",
            component: "metadata".into(),
            storage: "direct",
        }));
        Scope {
            previous,
            total: Some((self.0.clone(), snapshot())),
        }
    }

    pub fn json(&self) -> serde_json::Value {
        let data = self.0.borrow();
        let tracked_hits: u64 = data.buckets.values().map(|v| v.hits).sum();
        let tracked_reads: u64 = data.buckets.values().map(|v| v.reads).sum();
        let unique: BTreeSet<_> = data.pages.values().flatten().copied().collect();
        let mut components = BTreeMap::<&str, Counters>::new();
        for (key, value) in &data.buckets {
            let component = key.split('/').nth(1).unwrap();
            let counter = components.entry(component).or_default();
            counter.hits += value.hits;
            counter.reads += value.reads;
        }
        serde_json::json!({
            "scope": "this backend; ExecCustomScan calls only",
            "total": data.total,
            "components": components,
            "file_read_calls": data.file_reads,
            "unique_pages_scope": "RelationBufferAccess only; excludes heap pages",
            "tracked": {"hits": tracked_hits, "reads": tracked_reads, "unique_pages": unique.len()},
            "untracked": {"hits": data.total.hits.saturating_sub(tracked_hits), "reads": data.total.reads.saturating_sub(tracked_reads)},
            "buckets": data.buckets,
            "unique_pages_by_bucket": data.pages.iter().map(|(k,v)| (k,v.len())).collect::<BTreeMap<_,_>>()
        })
    }
}

pub fn label(
    phase: Option<&'static str>,
    component: Option<String>,
    storage: Option<&'static str>,
) -> Scope {
    let previous = ACTIVE.with_borrow_mut(|active| {
        let previous = active.clone();
        if let Some(context) = active {
            if let Some(phase) = phase {
                context.phase = phase;
            }
            if let Some(component) = component {
                context.component = component;
            }
            if let Some(storage) = storage {
                context.storage = storage;
            }
        }
        previous
    });
    Scope {
        previous,
        total: None,
    }
}

pub fn buffer<R>(relation: u32, fork: i32, block: u32, read: impl FnOnce() -> R) -> R {
    let context = ACTIVE.with_borrow(Clone::clone);
    let Some(context) = context else {
        return read();
    };
    let before = snapshot();
    let result = read();
    let after = snapshot();
    let key = format!(
        "{}/{}/{}",
        context.phase, context.component, context.storage
    );
    let mut data = context.data.borrow_mut();
    data.buckets
        .entry(key.clone())
        .or_default()
        .add(before, after);
    data.pages
        .entry(key)
        .or_default()
        .insert((relation, fork, block));
    result
}

pub fn file_read(component: String, access: &'static str) -> Scope {
    ACTIVE.with_borrow(|active| {
        if let Some(context) = active {
            *context
                .data
                .borrow_mut()
                .file_reads
                .entry(format!("{component}/{access}"))
                .or_default() += 1;
        }
    });
    label(None, Some(component), Some(access))
}
