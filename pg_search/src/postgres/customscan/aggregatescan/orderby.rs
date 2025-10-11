use crate::api::{FieldName, HashSet, OrderByFeature, OrderByInfo};
use crate::customscan::CustomScan;
use crate::customscan::builders::custom_path::{CustomPathBuilder, OrderByStyle};
use crate::postgres::customscan::aggregatescan::AggregateClause;
use crate::postgres::customscan::pdbscan::extract_pathkey_styles_with_sortability_check;
use crate::postgres::customscan::pdbscan::PathKeyInfo;
use crate::schema::SearchIndexSchema;
use pgrx::pg_sys;

pub struct OrderByClause {
    pathkeys: PathKeyInfo,
}

impl OrderByClause {
    pub fn orderby_info(&self, sort_fields: &HashSet<FieldName>) -> Vec<OrderByInfo> {
        OrderByStyle::extract_orderby_info(self.pathkeys.pathkeys())
            .into_iter()
            .filter(|info| {
                if let OrderByFeature::Field(field_name) = &info.feature {
                    sort_fields.contains(&field_name)
                } else {
                    false
                }
            })
            .collect::<Vec<_>>()
    }
}

impl AggregateClause for OrderByClause {
    fn add_to_builder<CS>(&self, mut builder: CustomPathBuilder<CS>) -> CustomPathBuilder<CS>
    where
        CS: CustomScan,
    {
        if let Some(pathkeys) = self.pathkeys.pathkeys() {
            for pathkey_style in pathkeys {
                builder = builder.add_path_key(pathkey_style);
            }
        };

        builder
    }

    fn from_pg(
        root: *mut pg_sys::PlannerInfo,
        heap_rti: pg_sys::Index,
        schema: &SearchIndexSchema,
    ) -> Option<Self> {
        let pathkeys = unsafe {
            extract_pathkey_styles_with_sortability_check(
                root,
                heap_rti,
                schema,
                |f| f.is_fast(),
                |_| false,
            )
        };

        Some(Self { pathkeys })
    }
}
