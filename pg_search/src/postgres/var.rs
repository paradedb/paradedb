use crate::api::FieldName;
use crate::api::HashSet;
use crate::customscan::operator_oid;
use crate::nodecast;
use pgrx::pg_sys::NodeTag::{T_Const, T_OpExpr, T_Var};
use pgrx::pg_sys::{expression_tree_walker, Const, OpExpr, Var};
use pgrx::PgOid;
use pgrx::{is_a, pg_guard, pg_sys, FromDatum, PgList, PgRelation};
use std::ffi::CStr;
use std::ptr::addr_of_mut;
use std::sync::OnceLock;

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
    let rte = pg_sys::rt_fetch((*var).varno as pg_sys::Index, (*query).rtable);

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
    if (*var).varattno == 0 {
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
    root: *mut pg_sys::PlannerInfo,
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
            let path = find_json_path(root, node);
            return Some((var, path.join(".").into()));
        }
        None
    } else if is_a(node, T_Var) {
        let var = node.cast::<Var>();
        let (heaprelid, varattno, _) = find_var_relation(var, root);
        Some((var, fieldname_from_var(heaprelid, var, varattno)?))
    } else {
        None
    }
}

/// Given a [`pg_sys::Node`] and a [`pg_sys::PlannerInfo`], attempt to find the JSON path that the
/// node references.
///
/// It is the caller's responsibility to ensure that the node is a JSON path expression.
#[inline(always)]
unsafe fn find_json_path(root: *mut pg_sys::PlannerInfo, node: *mut pg_sys::Node) -> Vec<String> {
    let mut path = Vec::new();

    if is_a(node, T_Var) {
        let node = node as *mut Var;
        let (heaprelid, varattno, _) = find_var_relation(node, root);
        let field_name = fieldname_from_var(heaprelid, node, varattno)
            .expect("find_json_path: var should have a valid FieldName");
        path.push(field_name.root());
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
            path.extend(find_json_path(root, expr));
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
