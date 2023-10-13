use pgrx::*;

#[pg_extern(sql = "
CREATE FUNCTION sparse_hnsw_handler(internal) RETURNS index_am_handler PARALLEL SAFE IMMUTABLE STRICT LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
CREATE ACCESS METHOD sparse_hnsw TYPE INDEX HANDLER sparse_hnsw_handler;
")]
fn sparse_hnsw_handler(_fcinfo: pg_sys::FunctionCallInfo) -> PgBox<pg_sys::IndexAmRoutine> {
    let mut amroutine =
        unsafe { PgBox::<pg_sys::IndexAmRoutine>::alloc_node(pg_sys::NodeTag_T_IndexAmRoutine) };

    amroutine.amstrategies = 0;
    amroutine.amsupport = 1;
	amroutine.amcanorder = false;
	amroutine.amcanorderbyop = true;
	amroutine.amcanbackward = false;
	amroutine.amcanunique = false;
	amroutine.amcanmulticol = false;
	amroutine.amoptionalkey = true;
	amroutine.amsearcharray = false;
	amroutine.amsearchnulls = false;
	amroutine.amstorage = false;
	amroutine.amclusterable = false;
	amroutine.ampredlocks = false;
	amroutine.amcanparallel = false;
	amroutine.amcaninclude = false;
	amroutine.amkeytype = pg_sys::InvalidOid;
	amroutine.ambuild = None;
	amroutine.ambuildempty = None;
	amroutine.aminsert = None;
	amroutine.ambulkdelete = None;
	amroutine.amvacuumcleanup = None;
	amroutine.amcanreturn = None;
	amroutine.amcostestimate = None;
	amroutine.amoptions = None;
	amroutine.amproperty = None;
	amroutine.ambuildphasename = None;
	amroutine.amvalidate = None;
	amroutine.ambeginscan = None;
	amroutine.amrescan = None;
	amroutine.amgettuple = None;
	amroutine.amgetbitmap = None;
	amroutine.amendscan = None;
	amroutine.ammarkpos = None;
	amroutine.amrestrpos = None;
	amroutine.amestimateparallelscan = None;
	amroutine.aminitparallelscan = None;
	amroutine.amparallelrescan = None;

	#[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
	{
		amroutine.amoptsprocnum = 0;
		amroutine.amusemaintenanceworkmem = false;
		amroutine.amparallelvacuumoptions = pg_sys::VACUUM_OPTION_PARALLEL_BULKDEL as u8;
	}

	#[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
	{
		amroutine.amadjustmembers = None;
	}
    
    amroutine.into_pg_boxed()
}
