use deltalake::datafusion::logical_expr::LogicalPlan;

pub struct LogicalPlanDetails {
    logical_plan: LogicalPlan,
    includes_udf: bool,
}

impl LogicalPlanDetails {
    pub fn new(logical_plan: LogicalPlan, includes_udf: bool) -> Self {
        Self {
            logical_plan,
            includes_udf,
        }
    }

    pub fn logical_plan(&self) -> LogicalPlan {
        self.logical_plan.clone()
    }

    pub fn includes_udf(&self) -> bool {
        self.includes_udf
    }
}
