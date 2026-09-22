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
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use tantivy::index::SegmentComponent;

type SharedData = Rc<RefCell<Data>>;

#[derive(Default)]
struct Data {
    total: u64,
    components: BTreeMap<String, u64>,
}

#[derive(Clone)]
struct Context {
    data: SharedData,
    component: String,
}

thread_local! {
    static ACTIVE: RefCell<Option<Context>> = const { RefCell::new(None) };
}

#[derive(Default)]
pub struct Trace(SharedData);

pub struct Scope {
    previous: Option<Context>,
    total: Option<(SharedData, i64)>,
}

pub struct External {
    context: Option<Context>,
    before: i64,
    name: &'static str,
}

impl Drop for External {
    fn drop(&mut self) {
        if let Some(context) = &self.context {
            *context
                .data
                .borrow_mut()
                .components
                .entry(self.name.into())
                .or_default() += snapshot().saturating_sub(self.before) as u64;
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
            data.borrow_mut().total += snapshot().saturating_sub(*before) as u64;
        }
        ACTIVE.set(self.previous.take());
    }
}

fn snapshot() -> i64 {
    unsafe { std::ptr::addr_of!(pg_sys::pgBufferUsage.shared_blks_hit).read() }
}

impl Trace {
    pub fn enter(&self) -> Scope {
        let previous = ACTIVE.replace(Some(Context {
            data: self.0.clone(),
            component: "Metadata".into(),
        }));
        Scope {
            previous,
            total: Some((self.0.clone(), snapshot())),
        }
    }

    pub fn hits(&self) -> Vec<(String, u64)> {
        let data = self.0.borrow();
        let mut hits = vec![("Total".into(), data.total)];
        hits.extend(
            data.components
                .iter()
                .filter(|(_, hits)| **hits > 0)
                .map(|(component, hits)| (component.clone(), *hits)),
        );
        let other = data.total.saturating_sub(data.components.values().sum());
        if other > 0 {
            hits.push(("Other".into(), other));
        }
        hits
    }
}

pub fn buffer<R>(read: impl FnOnce() -> R) -> R {
    let context = ACTIVE.with_borrow(Clone::clone);
    let Some(context) = context else {
        return read();
    };
    let before = snapshot();
    let result = read();
    *context
        .data
        .borrow_mut()
        .components
        .entry(context.component)
        .or_default() += snapshot().saturating_sub(before) as u64;
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
                "fast" => "Columnar Fields",
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
