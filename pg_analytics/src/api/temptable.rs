// use pgrx::*;

// unsafe {
//     let temp_schema_oid = direct_function_call::<pg_sys::Oid>(pg_sys::pg_my_temp_schema, &[]).expect("could not unwrap");
//     let temp_schema_name =
//         CStr::from_ptr(pg_sys::get_namespace_name(temp_schema_oid)).to_str()?;
// }
