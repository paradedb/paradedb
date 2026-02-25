use std::fmt::{self, Debug};
use std::sync::Arc;

use arrow_schema::{Field, Schema};
use datafusion::common::{DFSchema, DFSchemaRef, Result};
use datafusion::logical_expr::{LogicalPlan, UserDefinedLogicalNodeCore};
use datafusion::prelude::Expr;

use crate::scan::tantivy_lookup_exec::DeferredField;
/// The logical representation of the late materialization step.
#[derive(Clone)]
pub struct LogicalTantivyLookup {
    pub input: Arc<LogicalPlan>,
    pub deferred_fields: Vec<DeferredField>,
    /// The updated schema where the deferred fields are now UnionArray type
    pub schema: DFSchemaRef,
}
impl LogicalTantivyLookup {
    pub fn new(input: Arc<LogicalPlan>, deferred_fields: Vec<DeferredField>) -> Result<Self> {
        let input_schema = input.schema().as_ref();

        let mut new_fields = Vec::with_capacity(input_schema.fields().len());
        for field in input_schema.fields() {
            let name = field.name();
            if let Some(deferred) = deferred_fields.iter().find(|d| d.field_name == *name) {
                new_fields.push(
                    Field::new(name, deferred.output_data_type(), true).into(),
                );
            } else {
                new_fields.push(field.clone());
            }
        }

        let new_schema = Arc::new(Schema::new(new_fields));
        let df_schema = DFSchema::try_from(new_schema)?;

        Ok(Self {
            input,
            deferred_fields,
            schema: Arc::new(df_schema),
        })
    }
}
impl PartialEq for LogicalTantivyLookup {
    fn eq(&self, other: &Self) -> bool {
        self.input == other.input && self.deferred_fields == other.deferred_fields
    }
}

impl Eq for LogicalTantivyLookup {}

impl std::hash::Hash for LogicalTantivyLookup {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.input.hash(state);
        self.deferred_fields.hash(state);
    }
}

impl PartialOrd for LogicalTantivyLookup {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.input.partial_cmp(&other.input) {
            Some(std::cmp::Ordering::Equal) => {
                self.deferred_fields.partial_cmp(&other.deferred_fields)
            }
            cmp => cmp,
        }
    }
}
impl Debug for LogicalTantivyLookup {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        UserDefinedLogicalNodeCore::fmt_for_explain(self, f)
    }
}

impl UserDefinedLogicalNodeCore for LogicalTantivyLookup {
    fn name(&self) -> &str {
        "LogicalTantivyLookup"
    }

    fn inputs(&self) -> Vec<&LogicalPlan> {
        vec![&self.input]
    }

    fn schema(&self) -> &DFSchemaRef {
        &self.schema
    }

    fn expressions(&self) -> Vec<Expr> {
        vec![]
    }

    fn fmt_for_explain(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "LogicalTantivyLookup: decode=[{}]",
            self.deferred_fields
                .iter()
                .map(|d| d.field_name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn with_exprs_and_inputs(
        &self,
        _exprs: Vec<Expr>,
        mut inputs: Vec<LogicalPlan>,
    ) -> Result<Self> {
        Ok(Self {
            input: Arc::new(inputs.remove(0)),
            deferred_fields: self.deferred_fields.clone(),
            schema: self.schema.clone(),
        })
    }
}
