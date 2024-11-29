use anyhow::{anyhow, bail};
use std::path::PathBuf;
use tantivy::index::SegmentId;

// Converts a SegmentID + SegmentComponent into a PathBuf
pub struct SegmentComponentPath(pub PathBuf);
pub struct SegmentComponentId(pub SegmentId);

impl TryFrom<SegmentComponentPath> for SegmentComponentId {
    type Error = anyhow::Error;

    fn try_from(val: SegmentComponentPath) -> Result<Self, Self::Error> {
        let path_str = val
            .0
            .to_str()
            .ok_or_else(|| anyhow!("Invalid segment path: {:?}", val.0.to_str().unwrap()))?;
        if let Some(pos) = path_str.find('.') {
            Ok(SegmentComponentId(SegmentId::from_uuid_string(
                &path_str[..pos],
            )?))
        } else {
            bail!("Invalid segment path: {}", path_str);
        }
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use tantivy::index::SegmentId;

    #[pg_test]
    fn test_segment_component_path_to_id() {
        let path = SegmentComponentPath(PathBuf::from("00000000-0000-0000-0000-000000000000.ext"));
        let id = SegmentComponentId::try_from(path).unwrap();
        assert_eq!(
            id.0,
            SegmentId::from_uuid_string("00000000-0000-0000-0000-000000000000").unwrap()
        );

        let path = SegmentComponentPath(PathBuf::from(
            "00000000-0000-0000-0000-000000000000.123.del",
        ));
        let id = SegmentComponentId::try_from(path).unwrap();
        assert_eq!(
            id.0,
            SegmentId::from_uuid_string("00000000-0000-0000-0000-000000000000").unwrap()
        );
    }
}
