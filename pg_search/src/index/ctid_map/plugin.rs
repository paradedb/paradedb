// Copyright (c) 2023-2026 ParadeDB, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::any::Any;
use std::sync::Arc;

use tantivy::directory::CompositeWrite;
use tantivy::index::{Segment, SegmentComponent};
use tantivy::indexer::DocIdMapping;
use tantivy::{Index, PluginMergeContext, PluginWriter, PluginWriterContext, SegmentPlugin};

use crate::postgres::storage::block::CTID_MAP_EXT;

struct CtidMapPlugin;
struct CtidMapWriter;

/// Registers the CTID map for segment writes and merges.
pub(crate) fn register(index: &mut Index) {
    index.register_plugin(Arc::new(CtidMapPlugin));
}

impl SegmentPlugin for CtidMapPlugin {
    fn extensions(&self) -> &[&str] {
        &[CTID_MAP_EXT]
    }

    fn create_writer(&self, _ctx: &PluginWriterContext) -> tantivy::Result<Box<dyn PluginWriter>> {
        Ok(Box::new(CtidMapWriter))
    }

    fn merge(&self, ctx: PluginMergeContext) -> tantivy::Result<()> {
        write_component(ctx.target_segment)
    }
}

impl PluginWriter for CtidMapWriter {
    fn serialize(
        self: Box<Self>,
        segment: &Segment,
        _doc_id_map: Option<&DocIdMapping>,
    ) -> tantivy::Result<()> {
        write_component(segment)
    }

    fn mem_usage(&self) -> usize {
        0
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub(super) fn component() -> SegmentComponent {
    SegmentComponent::Custom(CTID_MAP_EXT.to_string())
}

/// Builds from final CTIDs after sorting or merging has assigned document IDs.
fn write_component(segment: &Segment) -> tantivy::Result<()> {
    let mut output = CompositeWrite::wrap(segment.open_write(component())?);
    super::write(segment, &mut output)?;
    output.close()?;
    Ok(())
}
