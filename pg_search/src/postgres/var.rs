use crate::api::FieldName;
use crate::api::HashSet;
use crate::customscan::operator_oid;
use crate::nodecast;
use pgrx::pg_sys::NodeTag::{T_CoerceViaIO, T_Const, T_OpExpr, T_RelabelType, T_Var};
use pgrx::pg_sys::{expression_tree_walker, CoerceViaIO, Const, OpExpr, RelabelType, Var};
use pgrx::PgOid;
use pgrx::{is_a, pg_guard, pg_sys, FromDatum, PgList, PgRelation};
use std::ffi::CStr;
use std::ptr::addr_of_mut;
use std::sync::OnceLock;

#[derive(Clone, Copy)]
pub enum VarContext {
    Planner(*mut pg_sys::PlannerInfo),
    Query(*mut pg_sys::Query),
    Exec(pg_sys::Oid),
}

impl VarContext {
    pub fn from_planner(root: *mut pg_sys::PlannerInfo) -> Self {
        Self::Planner(root)
    }

    pub fn from_query(parse: *mut pg_sys::Query) -> Self {
        Self::Query(parse)
    }

    pub fn from_exec(heaprelid: pg_sys::Oid) -> Self {
        Self::Exec(heaprelid)
    }

    pub fn var_relation(&self, var: *mut pg_sys::Var) -> (pg_sys::Oid, pg_sys::AttrNumber) {
        match self {
            Self::Planner(root) => {
                let (heaprelid, varattno, _) = unsafe { find_var_relation(var, *root) };
                (heaprelid, varattno)
            }
            Self::Query(parse) => unsafe {
                // Early return for null pointers
                if var.is_null() || parse.is_null() {
                    return (pg_sys::InvalidOid, (*var).varattno);
                }

                let query_ptr = *parse;
                let varno = (*var).varno;
                let rtable = (*query_ptr).rtable;

                // Early return for invalid rtable or varno
                if rtable.is_null() || varno <= 0 {
                    return (pg_sys::InvalidOid, (*var).varattno);
                }

                let rtable_list = PgList::<pg_sys::RangeTblEntry>::from_pg(rtable);
                let rte_index = (varno - 1) as usize;

                // Early return for out of bounds index
                if rte_index >= rtable_list.len() {
                    return (pg_sys::InvalidOid, (*var).varattno);
                }

                let rte = rtable_list.get_ptr(rte_index).unwrap();
                let varattno = (*var).varattno;

                if (*rte).rtekind == pg_sys::RTEKind::RTE_RELATION {
                    return ((*rte).relid, varattno);
                } else if (*rte).rtekind == pg_sys::RTEKind::RTE_SUBQUERY {
                    let subquery = (*rte).subquery;
                    if !subquery.is_null() {
                        let targetlist =
                            PgList::<pg_sys::TargetEntry>::from_pg((*subquery).targetList);
                        if varattno > 0 && (varattno as usize) <= targetlist.len() {
                            if let Some(te) = targetlist.get_ptr(varattno as usize - 1) {
                                if (*te).resorigtbl != pg_sys::InvalidOid {
                                    return ((*te).resorigtbl, (*te).resorigcol);
                                }
                            }
                        }
                    }
                }

                (pg_sys::InvalidOid, varattno)
            },
            Self::Exec(heaprelid) => (*heaprelid, unsafe { (*var).varattno }),
        }
    }
}

/// Given a [`pg_sys::Var`] and a [`pg_sys::PlannerInfo`], attempt to find the relation Oid that
/// contains the var.
///
/// It's possible the returned Oid will be [`pg_sys::Oid::INVALID`] if the Var doesn't eventually
/// come from a relation.
///
/// The returned [`pg_sys::AttrNumber`] is the physical attribute number in the relation the Var
/// is from.
pub unsafe fn find_var_relation(
    var: *mut pg_sys::Var,
    root: *mut pg_sys::PlannerInfo,
) -> (
    pg_sys::Oid,
    pg_sys::AttrNumber,
    Option<PgList<pg_sys::TargetEntry>>,
) {
    let query = (*root).parse;
    let varno = (*var).varno as pg_sys::Index;
    let rtable = (*query).rtable;
    let rtable_size = if !rtable.is_null() {
        pgrx::PgList::<pg_sys::RangeTblEntry>::from_pg(rtable).len()
    } else {
        0
    };

    // Bounds check: varno is 1-indexed, so it must be between 1 and rtable_size
    if varno == 0 || varno as usize > rtable_size {
        // This Var references an RTE that doesn't exist in the current context
        // This can happen with OR EXISTS subqueries where the Var comes from a subquery context
        // Return invalid values to signal this var cannot be processed
        return (pg_sys::InvalidOid, 0, None);
    }

    let rte = pg_sys::rt_fetch(varno, rtable);

    match (*rte).rtekind {
        // the Var comes from a relation
        pg_sys::RTEKind::RTE_RELATION => ((*rte).relid, (*var).varattno, None),

        // the Var comes from a subquery, so dig into its target list and find the original
        // table it comes from along with its original column AttributeNumber
        pg_sys::RTEKind::RTE_SUBQUERY => {
            if (*rte).subquery.is_null() {
                panic!("unable to determine Var relation as it belongs to a NULL subquery");
            }
            let targetlist = PgList::<pg_sys::TargetEntry>::from_pg((*(*rte).subquery).targetList);
            let te = targetlist
                .get_ptr((*var).varattno as usize - 1)
                .expect("var should exist in subquery TargetList");
            ((*te).resorigtbl, (*te).resorigcol, Some(targetlist))
        }

        // the Var comes from a CTE, so lookup that CTE and find it in the CTE's target list
        pg_sys::RTEKind::RTE_CTE => {
            let mut levelsup = (*rte).ctelevelsup;
            let mut cteroot = root;
            while levelsup > 0 {
                cteroot = (*cteroot).parent_root;
                if cteroot.is_null() {
                    // shouldn't happen
                    panic!(
                        "bad levelsup for CTE \"{}\"",
                        CStr::from_ptr((*rte).ctename).to_string_lossy()
                    )
                }
                levelsup -= 1;
            }

            let rte_ctename = CStr::from_ptr((*rte).ctename);
            let ctelist = PgList::<pg_sys::CommonTableExpr>::from_pg((*(*cteroot).parse).cteList);
            let mut matching_cte = None;
            for cte in ctelist.iter_ptr() {
                let ctename = CStr::from_ptr((*cte).ctename);

                if ctename == rte_ctename {
                    matching_cte = Some(cte);
                    break;
                }
            }

            let cte = matching_cte.unwrap_or_else(|| {
                panic!(
                    "unable to find cte named \"{}\"",
                    rte_ctename.to_string_lossy()
                )
            });

            if !is_a((*cte).ctequery, pg_sys::NodeTag::T_Query) {
                panic!("CTE is not a query")
            }
            let query = (*cte).ctequery.cast::<pg_sys::Query>();
            let targetlist = if !(*query).returningList.is_null() {
                PgList::<pg_sys::TargetEntry>::from_pg((*query).returningList)
            } else {
                PgList::<pg_sys::TargetEntry>::from_pg((*query).targetList)
            };
            let te = targetlist
                .get_ptr((*var).varattno as usize - 1)
                .expect("var should exist in cte TargetList");

            ((*te).resorigtbl, (*te).resorigcol, Some(targetlist))
        }

        // Custom scans involving named tuple stores (such as those created by `pg_ivm`) are not
        // supported.
        pg_sys::RTEKind::RTE_NAMEDTUPLESTORE => {
            (pg_sys::Oid::INVALID, pg_sys::InvalidAttrNumber as i16, None)
        }

        // Likewise, the safest bet for any other RTEKind that we do not recognize is to ignore it.
        rtekind => {
            pgrx::debug1!("Unsupported RTEKind in `find_var_relation`: {rtekind}");
            (pg_sys::Oid::INVALID, pg_sys::InvalidAttrNumber as i16, None)
        }
    }
}

/// Find all the Vars referenced in the specified node
pub unsafe fn find_vars(node: *mut pg_sys::Node) -> Vec<*mut pg_sys::Var> {
    #[pg_guard]
    unsafe extern "C-unwind" fn walker(
        node: *mut pg_sys::Node,
        data: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }

        if let Some(var) = nodecast!(Var, T_Var, node) {
            let data = data.cast::<Data>();
            (*data).vars.push(var);
        }

        expression_tree_walker(node, Some(walker), data)
    }

    struct Data {
        vars: Vec<*mut pg_sys::Var>,
    }

    let mut data = Data { vars: Vec::new() };

    walker(node, addr_of_mut!(data).cast());
    data.vars
}

/// Given a [`pg_sys::Var`], attempt to find the [`FieldName`] that it references.
pub unsafe fn fieldname_from_var(
    heaprelid: pg_sys::Oid,
    var: *mut pg_sys::Var,
    varattno: pg_sys::AttrNumber,
) -> Option<FieldName> {
    if (*var).varattno == 0 || heaprelid == pg_sys::Oid::INVALID {
        return None;
    }
    // Check for InvalidOid before trying to open the relation
    if heaprelid == pg_sys::InvalidOid {
        return None;
    }
    let heaprel = PgRelation::open(heaprelid);
    let tupdesc = heaprel.tuple_desc();
    if varattno == pg_sys::SelfItemPointerAttributeNumber as pg_sys::AttrNumber {
        Some("ctid".into())
    } else {
        tupdesc
            .get(varattno as usize - 1)
            .map(|attribute| attribute.name().into())
    }
}

/// Given a [`pg_sys::Node`], attempt to find the [`pg_sys::Var`] that it references.
///
/// If there is not exactly one Var in the node, then this function will return `None`.
pub unsafe fn find_one_var(node: *mut pg_sys::Node) -> Option<*mut pg_sys::Var> {
    let mut vars = find_vars(node);
    if vars.len() == 1 {
        Some(vars.pop().unwrap())
    } else {
        None
    }
}

/// Given a [`pg_sys::Node`] and a [`pg_sys::PlannerInfo`], attempt to find the [`pg_sys::Var`] and
/// the [`FieldName`] that it references.
///
/// It is the caller's responsibility to ensure that the node contains a Var with a valid FieldName.
pub unsafe fn find_one_var_and_fieldname(
    context: VarContext,
    node: *mut pg_sys::Node,
) -> Option<(*mut pg_sys::Var, FieldName)> {
    if is_a(node, T_OpExpr) {
        let opexpr = node.cast::<OpExpr>();
        static JSON_OPERATOR_LOOKUP: OnceLock<HashSet<pg_sys::Oid>> = OnceLock::new();
        if JSON_OPERATOR_LOOKUP
            .get_or_init(|| initialize_json_operator_lookup())
            .contains(&(*opexpr).opno)
        {
            let var = find_one_var(node)?;
            let path = find_json_path(&context, node);
            return Some((var, path.join(".").into()));
        }
        None
    } else if is_a(node, pg_sys::NodeTag::T_SubscriptingRef) {
        // Handle PostgreSQL 14+ bracket notation: json['key']
        let var = find_one_var(node)?;
        let path = find_json_path(&context, node);
        Some((var, path.join(".").into()))
    } else if is_a(node, T_Var) {
        let var = node.cast::<Var>();
        let (heaprelid, varattno) = context.var_relation(var);
        Some((var, fieldname_from_var(heaprelid, var, varattno)?))
    } else if is_a(node, T_CoerceViaIO) {
        let expr = node.cast::<CoerceViaIO>();
        let arg = (*expr).arg;
        find_one_var_and_fieldname(context, arg as *mut pg_sys::Node)
    } else if is_a(node, T_RelabelType) {
        let relabel_type = node.cast::<RelabelType>();
        let arg = (*relabel_type).arg;
        find_one_var_and_fieldname(context, arg as *mut pg_sys::Node)
    } else {
        None
    }
}

/// Given a [`pg_sys::Node`] and a [`pg_sys::PlannerInfo`], attempt to find the JSON path that the
/// node references.
///
/// It is the caller's responsibility to ensure that the node is a JSON path expression.
#[inline(always)]
pub unsafe fn find_json_path(context: &VarContext, node: *mut pg_sys::Node) -> Vec<String> {
    let mut path = Vec::new();

    if is_a(node, T_Var) {
        let node = node as *mut Var;
        let (heaprelid, varattno) = context.var_relation(node);
        // Return empty path if we can't get a valid field name (e.g., due to out-of-bounds varno)
        if let Some(field_name) = fieldname_from_var(heaprelid, node, varattno) {
            path.push(field_name.root());
        }
        return path;
    } else if is_a(node, T_Const) {
        let node = node as *mut Const;
        if let PgOid::BuiltIn(oid) = PgOid::from((*node).consttype) {
            match oid {
                pg_sys::BuiltinOid::TEXTOID | pg_sys::BuiltinOid::VARCHAROID => {
                    if let Some(s) = String::from_datum((*node).constvalue, (*node).constisnull) {
                        path.push(s);
                    }
                }
                pg_sys::BuiltinOid::TEXTARRAYOID | pg_sys::BuiltinOid::VARCHARARRAYOID => {
                    if let Some(array) =
                        pgrx::Array::<String>::from_datum((*node).constvalue, (*node).constisnull)
                    {
                        path.extend(array.iter().flatten());
                    }
                }
                _ => {}
            }
        }

        return path;
    } else if is_a(node, T_OpExpr) {
        let node = node as *mut OpExpr;
        for expr in PgList::from_pg((*node).args).iter_ptr() {
            path.extend(find_json_path(context, expr));
        }
    } else if is_a(node, pg_sys::NodeTag::T_SubscriptingRef) {
        let node = node as *mut pg_sys::SubscriptingRef;
        // Extract container and subscript expressions for bracket notation
        path.extend(find_json_path(context, (*node).refexpr.cast()));
        for expr in PgList::from_pg((*node).refupperindexpr).iter_ptr() {
            path.extend(find_json_path(context, expr));
        }
    }

    path
}

#[inline(always)]
unsafe fn initialize_json_operator_lookup() -> HashSet<pg_sys::Oid> {
    const OPERATORS: [&str; 2] = ["->", "->>"];
    const TYPE_PAIRS: &[[&str; 2]] = &[
        ["json", "text"],
        ["jsonb", "text"],
        ["json", "int4"],
        ["jsonb", "int4"],
    ];

    let mut lookup = HashSet::default();
    for o in OPERATORS {
        for [l, r] in TYPE_PAIRS {
            lookup.insert(operator_oid(&format!("{o}({l},{r})")));
        }
    }

    lookup.insert(operator_oid("#>(json,text[])"));
    lookup.insert(operator_oid("#>(jsonb,text[])"));
    lookup.insert(operator_oid("#>>(json,text[])"));
    lookup.insert(operator_oid("#>>(jsonb,text[])"));

    lookup
}
