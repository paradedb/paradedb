use datafusion::logical_expr::LogicalPlan;
use pgrx::pg_sys::NodeTag;
use pgrx::*;

use crate::nodes::t_aggref::AggRefNode;
use crate::nodes::t_limit::LimitNode;
use crate::nodes::t_modifytable::ModifyTableNode;
use crate::nodes::t_result::ResultNode;
use crate::nodes::t_seqscan::SeqScanNode;
use crate::nodes::t_sort::SortNode;
use crate::nodes::t_valuesscan::ValuesScanNode;

use crate::nodes::producer::DatafusionPlanProducer;

pub struct RootPlanNode;
impl DatafusionPlanProducer for RootPlanNode {
    unsafe fn datafusion_plan(
        plan: *mut pg_sys::Plan,
        rtable: *mut pg_sys::List,
        _outer_plan: Option<LogicalPlan>,
    ) -> Result<LogicalPlan, String> {
        let node = plan as *mut pg_sys::Node;
        let node_tag = (*node).type_;

        // lefttree is the outer plan - this is what is fed INTO the current plan level
        // TODO: righttree is the inner plan - this is only ever set for JOIN operations, so we'll ignore it for now
        // more info: https://www.pgmustard.com/blog/2019/9/17/postgres-execution-plans-field-glossary
        let mut outer_plan = None;
        let left_tree = (*plan).lefttree;
        if !left_tree.is_null() {
            outer_plan = Some(Self::datafusion_plan(left_tree, rtable, None)?);
        }

        match node_tag {
            NodeTag::T_SeqScan => SeqScanNode::datafusion_plan(plan, rtable, outer_plan),
            NodeTag::T_ModifyTable => ModifyTableNode::datafusion_plan(plan, rtable, outer_plan),
            NodeTag::T_ValuesScan => ValuesScanNode::datafusion_plan(plan, rtable, outer_plan),
            NodeTag::T_Result => ResultNode::datafusion_plan(plan, rtable, outer_plan),
            NodeTag::T_Agg => AggRefNode::datafusion_plan(plan, rtable, outer_plan),
            NodeTag::T_Limit => LimitNode::datafusion_plan(plan, rtable, outer_plan),
            NodeTag::T_Sort => SortNode::datafusion_plan(plan, rtable, outer_plan),
            NodeTag::T_Group => todo!(),
            NodeTag::T_Invalid => todo!(),
            _ => Err(format!("node type {:?} not supported yet", node_tag)),
        }
    }
}
