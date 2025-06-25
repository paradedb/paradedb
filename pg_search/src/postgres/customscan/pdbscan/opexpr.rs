use crate::nodecast;
use pgrx::{pg_sys, PgList};

pub(crate) enum OpExpr {
    Array(*mut pg_sys::ScalarArrayOpExpr),
    Single(*mut pg_sys::OpExpr),
}

impl OpExpr {
    pub unsafe fn into_scalar_array(node: *mut pg_sys::Node) -> Option<Self> {
        nodecast!(ScalarArrayOpExpr, T_ScalarArrayOpExpr, node).map(OpExpr::Array)
    }

    pub unsafe fn into_op_expr(node: *mut pg_sys::Node) -> Option<Self> {
        nodecast!(OpExpr, T_OpExpr, node).map(OpExpr::Single)
    }

    pub unsafe fn args(&self) -> PgList<pg_sys::Node> {
        match self {
            OpExpr::Array(expr) => PgList::<pg_sys::Node>::from_pg((*(*expr)).args),
            OpExpr::Single(expr) => PgList::<pg_sys::Node>::from_pg((*(*expr)).args),
        }
    }

    pub unsafe fn use_or(&self) -> Option<bool> {
        match self {
            OpExpr::Array(expr) => Some((*(*expr)).useOr),
            OpExpr::Single(_) => None,
        }
    }

    pub unsafe fn opno(&self) -> pg_sys::Oid {
        match self {
            OpExpr::Array(expr) => (*(*expr)).opno,
            OpExpr::Single(expr) => (*(*expr)).opno,
        }
    }

    pub unsafe fn inputcollid(&self) -> pg_sys::Oid {
        match self {
            OpExpr::Array(expr) => (*(*expr)).inputcollid,
            OpExpr::Single(expr) => (*(*expr)).inputcollid,
        }
    }

    pub unsafe fn location(&self) -> pg_sys::int32 {
        match self {
            OpExpr::Array(expr) => (*(*expr)).location,
            OpExpr::Single(expr) => (*(*expr)).location,
        }
    }
}
