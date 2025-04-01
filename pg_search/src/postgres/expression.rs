use pgrx::{
    pg_sys::{self, AsPgCStr},
    PgList, PgRelation,
};

// find_funcexpr_idx returns the attribute number of the
// node in the index.
pub fn find_funcexpr_attnum(indexrel: &PgRelation, node: *mut pg_sys::Node) -> Option<i32> {
    let index_info = unsafe { *pg_sys::BuildIndexInfo(indexrel.as_ptr()) };
    let heaprel = indexrel
        .heap_relation()
        .expect("index relation should have a heap relation");

    let expressions = unsafe { PgList::<pg_sys::Expr>::from_pg(index_info.ii_Expressions) };
    let ref_str = unsafe { get_expr_str(node, &heaprel) };
    let mut expressions_iter = expressions.iter_ptr();

    for i in 0..index_info.ii_NumIndexAttrs {
        let heap_attno = index_info.ii_IndexAttrNumbers[i as usize];
        if heap_attno == 0 {
            let Some(expression) = expressions_iter.next() else {
                panic!("Expected expression for index attribute {i}.");
            };
            let node = expression.cast();

            let expr_str = unsafe { get_expr_str(node, &heaprel) };
            if expr_str == ref_str {
                return Some(i);
            }
        }
    }
    None
}

unsafe fn get_expr_str(node: *mut pg_sys::Node, heaprel: &PgRelation) -> String {
    let pg_cstr = pg_sys::deparse_expression(
        node,
        pg_sys::deparse_context_for(heaprel.name().as_pg_cstr(), heaprel.oid()),
        false,
        false,
    );
    let ref_str = core::ffi::CStr::from_ptr(pg_cstr)
        .to_str()
        .expect("Invalid UTF8 in result of deparse_expression.")
        .to_owned();

    pg_sys::pfree(pg_cstr.cast());
    ref_str
}
