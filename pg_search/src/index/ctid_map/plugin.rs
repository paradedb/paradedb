// Copyright (c) 2023-2026 ParadeDB, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::any::Any;
use std::io;
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
        let mut output = CompositeWrite::wrap(ctx.target_segment.open_write(component())?);
        super::write(ctx.target_segment, &mut output).map_err(io::Error::other)?;
        output.close()?;
        Ok(())
    }
}

impl PluginWriter for CtidMapWriter {
    fn serialize(
        self: Box<Self>,
        segment: &Segment,
        _doc_id_map: Option<&DocIdMapping>,
    ) -> tantivy::Result<()> {
        let mut output = CompositeWrite::wrap(segment.open_write(component())?);
        super::write(segment, &mut output).map_err(io::Error::other)?;
        output.close()?;
        Ok(())
    }

    // No buffered state during indexing; temporary column buffers exist only during serialization.
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
