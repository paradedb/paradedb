use deltalake::datafusion::logical_expr::LogicalPlan;

pub struct LogicalPlanDetails {
    pub logical_plan: LogicalPlan,
    pub includes_udf: bool,
}
