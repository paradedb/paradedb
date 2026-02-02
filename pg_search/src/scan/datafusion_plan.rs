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

use std::any::Any;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use arrow_array::RecordBatch;
use arrow_schema::SchemaRef;
use datafusion::common::{DataFusionError, Result};
use datafusion::execution::{RecordBatchStream, SendableRecordBatchStream, TaskContext};
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, Partitioning, PlanProperties,
};
use futures::Stream;

use crate::index::fast_fields_helper::FFHelper;
use crate::scan::{Scanner, VisibilityChecker};

/// A wrapper that implements Send + Sync unconditionally.
/// UNSAFE: Only use this when you guarantee single-threaded access or manual synchronization.
#[allow(dead_code)]
struct UnsafeSendSync<T>(T);

unsafe impl<T> Send for UnsafeSendSync<T> {}
unsafe impl<T> Sync for UnsafeSendSync<T> {}

type ScanState = (Scanner, FFHelper, Box<dyn VisibilityChecker>);

/// A DataFusion `ExecutionPlan` for scanning a `pg_search` index.
#[allow(dead_code)]
pub struct ScanPlan {
    // We use a Mutex to allow taking the fields during execute()
    // We wrap the state in UnsafeSendSync to satisfy ExecutionPlan's Send+Sync requirements
    // This is safe because we are running in a single-threaded environment (Postgres)
    state: Mutex<Option<UnsafeSendSync<ScanState>>>,
    properties: PlanProperties,
}

impl std::fmt::Debug for ScanPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScanPlan")
            .field("properties", &self.properties)
            .finish()
    }
}

impl ScanPlan {
    #[allow(dead_code)]
    pub fn new(
        scanner: Scanner,
        ffhelper: FFHelper,
        visibility: Box<dyn VisibilityChecker>,
    ) -> Self {
        let schema = scanner.schema();
        let properties = PlanProperties::new(
            EquivalenceProperties::new(schema),
            Partitioning::UnknownPartitioning(1),
            EmissionType::Incremental,
            Boundedness::Bounded,
        );
        Self {
            state: Mutex::new(Some(UnsafeSendSync((scanner, ffhelper, visibility)))),
            properties,
        }
    }
}

impl DisplayAs for ScanPlan {
    fn fmt_as(&self, _t: DisplayFormatType, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "PgSearchScan")
    }
}

impl ExecutionPlan for ScanPlan {
    fn name(&self) -> &str {
        "PgSearchScan"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn properties(&self) -> &PlanProperties {
        &self.properties
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![]
    }

    fn with_new_children(
        self: Arc<Self>,
        _children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        Ok(self)
    }

    fn execute(
        &self,
        _partition: usize,
        _context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        let mut state = self.state.lock().map_err(|e| {
            DataFusionError::Internal(format!("Failed to lock ScanPlan state: {e}"))
        })?;
        let UnsafeSendSync((scanner, ffhelper, visibility)) = state.take().ok_or_else(|| {
            DataFusionError::Internal("ScanPlan can only be executed once".to_string())
        })?;

        // SAFETY: pg_search operates in a single-threaded Tokio executor within Postgres,
        // so it is safe to wrap !Send types for use within DataFusion.
        let stream = unsafe {
            UnsafeSendStream::new(ScanStream {
                scanner,
                ffhelper,
                visibility,
                schema: self.properties.eq_properties.schema().clone(),
            })
        };
        Ok(Box::pin(stream))
    }
}

struct ScanStream {
    scanner: Scanner,
    ffhelper: FFHelper,
    visibility: Box<dyn VisibilityChecker>,
    schema: SchemaRef,
}

impl Stream for ScanStream {
    type Item = Result<RecordBatch>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match this.scanner.next(&mut this.ffhelper, &mut *this.visibility) {
            Some(batch) => Poll::Ready(Some(Ok(batch.to_record_batch(&this.schema)))),
            None => Poll::Ready(None),
        }
    }
}

impl RecordBatchStream for ScanStream {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
}

/// A wrapper that unsafely implements Send for a Stream.
///
/// This is used to wrap `ScanStream` which is !Send because it contains Tantivy and Postgres
/// state that is not Send. This is safe because pg_search operates in a single-threaded
/// Tokio executor within Postgres, and these objects will never cross thread boundaries.
#[allow(dead_code)]
struct UnsafeSendStream<T>(T);

impl<T> UnsafeSendStream<T> {
    #[allow(dead_code)]
    unsafe fn new(t: T) -> Self {
        Self(t)
    }
}

unsafe impl<T> Send for UnsafeSendStream<T> {}

impl<T: Stream> Stream for UnsafeSendStream<T> {
    type Item = T::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().0).poll_next(cx) }
    }
}

impl<T: RecordBatchStream> RecordBatchStream for UnsafeSendStream<T> {
    fn schema(&self) -> SchemaRef {
        self.0.schema()
    }
}
