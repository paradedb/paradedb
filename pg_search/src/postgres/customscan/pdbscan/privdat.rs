use crate::api::operator::anyelement_jsonb_opoid;
use crate::postgres::customscan::pdbscan::qual_inspect::{extract_quals, Qual};
use crate::postgres::customscan::pdbscan::SortDirection;
use pgrx::{pg_sys, PgList};

#[derive(Default, Debug)]
pub struct PrivateData {
    heaprelid: Option<pg_sys::Oid>,
    indexrelid: Option<pg_sys::Oid>,
    range_table_index: Option<pg_sys::Index>,
    restrict_info: Option<*mut pg_sys::List>,
    limit: Option<usize>,
    sort_direction: Option<SortDirection>,
    var_attname_lookup: Option<*mut pg_sys::List>,
}

impl From<*mut pg_sys::List> for PrivateData {
    fn from(list: *mut pg_sys::List) -> Self {
        unsafe { deserialize::deserialize(list) }
    }
}

impl From<PrivateData> for *mut pg_sys::List {
    fn from(value: PrivateData) -> Self {
        unsafe { serialize::serialize(value).into_pg() }
    }
}

//
// setter functions
//

impl PrivateData {
    pub fn set_heaprelid(&mut self, oid: pg_sys::Oid) {
        self.heaprelid = Some(oid);
    }

    pub fn set_indexrelid(&mut self, oid: pg_sys::Oid) {
        self.indexrelid = Some(oid);
    }

    pub fn set_range_table_index(&mut self, rti: pg_sys::Index) {
        self.range_table_index = Some(rti);
    }

    pub fn set_quals(&mut self, quals: PgList<pg_sys::RestrictInfo>) {
        self.restrict_info = Some(quals.into_pg())
    }

    pub fn set_limit(&mut self, limit: pg_sys::Cardinality) {
        if limit == -1.0 {
            self.limit = None;
        } else {
            self.limit = Some(limit.round() as usize);
        }
    }

    pub fn set_sort_direction(&mut self, direction: Option<SortDirection>) {
        self.sort_direction = direction;
    }

    pub fn set_var_attname_lookup(&mut self, var_attname_lookup: *mut pg_sys::List) {
        self.var_attname_lookup = Some(var_attname_lookup);
    }
}

//
// getter functions
//

impl PrivateData {
    pub fn heaprelid(&self) -> Option<pg_sys::Oid> {
        self.heaprelid
    }

    pub fn indexrelid(&self) -> Option<pg_sys::Oid> {
        self.indexrelid
    }

    pub fn range_table_index(&self) -> Option<pg_sys::Index> {
        self.range_table_index
    }

    pub fn quals(&self) -> Option<Qual> {
        unsafe {
            self.restrict_info
                .and_then(|ri| extract_quals(ri.cast(), anyelement_jsonb_opoid()))
        }
    }

    pub fn limit(&self) -> Option<usize> {
        self.limit
    }

    pub fn sort_direction(&self) -> Option<SortDirection> {
        self.sort_direction
    }

    pub fn var_attname_lookup(&self) -> Option<PgList<pg_sys::Node>> {
        self.var_attname_lookup
            .map(|list| unsafe { PgList::from_pg(list) })
    }
}

#[allow(non_snake_case)]
mod serialize {
    use crate::postgres::customscan::pdbscan::privdat::PrivateData;
    use pgrx::pg_sys::AsPgCStr;
    use pgrx::{pg_sys, PgList};
    use std::fmt::Display;

    unsafe fn makeInteger<T: Into<u32>>(input: Option<T>) -> *mut pg_sys::Node {
        unwrapOrNull(input.map(|i| pg_sys::makeInteger(i.into() as _).cast::<pg_sys::Node>()))
    }

    unsafe fn makeString<T: Display>(input: Option<T>) -> *mut pg_sys::Node {
        unwrapOrNull(
            input.map(|s| pg_sys::makeString(s.to_string().as_pg_cstr()).cast::<pg_sys::Node>()),
        )
    }
    unsafe fn makeBoolean<T: Into<bool>>(input: Option<T>) -> *mut pg_sys::Node {
        unwrapOrNull(input.map(|b| pg_sys::makeBoolean(b.into()).cast::<pg_sys::Node>()))
    }

    unsafe fn unwrapOrNull(node: Option<*mut pg_sys::Node>) -> *mut pg_sys::Node {
        node.unwrap_or_else(|| {
            pg_sys::makeNullConst(pg_sys::OIDOID, -1, pg_sys::Oid::INVALID).cast::<pg_sys::Node>()
        })
    }

    pub unsafe fn serialize(privdat: PrivateData) -> PgList<pg_sys::Node> {
        let mut ser = PgList::new();

        ser.push(makeInteger(privdat.heaprelid));
        ser.push(makeInteger(privdat.indexrelid));
        ser.push(makeInteger(privdat.range_table_index));
        ser.push(unwrapOrNull(privdat.restrict_info.map(|l| l.cast())));
        ser.push(makeString(privdat.limit));
        ser.push(makeInteger(privdat.sort_direction));
        ser.push(unwrapOrNull(
            privdat.var_attname_lookup.map(|v| v.cast::<pg_sys::Node>()),
        ));

        ser
    }
}

#[allow(non_snake_case)]
mod deserialize {
    use crate::api::{AsBool, AsCStr, AsInt};
    use crate::nodecast;
    use crate::postgres::customscan::pdbscan::privdat::PrivateData;
    use pgrx::{pg_sys, PgList};
    use std::str::FromStr;

    unsafe fn decodeInteger<T: From<u32>>(node: *mut pg_sys::Node) -> Option<T> {
        node.as_int().map(|i| (i as u32).into())
    }
    unsafe fn decodeString<T: FromStr>(node: *mut pg_sys::Node) -> Option<T> {
        node.as_c_str().map(|i| {
            let s = i.to_str().expect("string node should be valid utf8");
            T::from_str(s)
                .ok()
                .expect("value should parse from a String")
        })
    }
    unsafe fn decodeBoolean<T: From<bool>>(node: *mut pg_sys::Node) -> Option<T> {
        node.as_bool().map(|b| b.into())
    }

    pub unsafe fn deserialize(input: *mut pg_sys::List) -> PrivateData {
        let input = PgList::<pg_sys::Node>::from_pg(input);
        let mut privdat: PrivateData = Default::default();

        privdat.heaprelid = input.get_ptr(0).and_then(|n| decodeInteger(n));
        privdat.indexrelid = input.get_ptr(1).and_then(|n| decodeInteger(n));
        privdat.range_table_index = input.get_ptr(2).and_then(|n| decodeInteger(n));
        privdat.restrict_info = input.get_ptr(3).and_then(|n| nodecast!(List, T_List, n));
        privdat.limit = input.get_ptr(4).and_then(|n| decodeString(n));
        privdat.sort_direction = input.get_ptr(5).and_then(|n| decodeInteger(n));
        privdat.var_attname_lookup = input
            .get_ptr(6)
            .and_then(|n| nodecast!(List, T_List, n, true));

        privdat
    }
}
