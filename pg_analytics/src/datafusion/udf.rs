use pgrx::*;
use std::sync::Arc;

use deltalake::arrow::array::ArrayRef;
use deltalake::datafusion::arrow::datatypes::DataType;
use deltalake::datafusion::common::DataFusionError;
use deltalake::datafusion::common::ScalarValue;
use deltalake::datafusion::logical_expr::{create_udf, ColumnarValue, Volatility};
use std::cmp::max;
use std::ffi::CString;

use crate::datafusion::session::Session;
use crate::errors::ParadeError;
use crate::types::array::IntoArrowArray;
use crate::types::datatype::{ArrowDataType, PgAttribute, PgTypeMod};
use crate::types::datum::GetDatum;

// NOTE: because we don't use argtypes yet (see TODO below), using this function on overloaded functions WILL
//       throw a postgres error
unsafe fn func_oid_from_signature(
    funcname: &str,
    _argtypes: *mut pg_sys::Oid,
) -> Result<pg_sys::Oid, ParadeError> {
    let cstr = CString::new(funcname)?;

    let funcname_list: *mut pg_sys::List;
    #[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
    {
        funcname_list = unsafe { pg_sys::stringToQualifiedNameList(cstr.as_ptr()) };
    }
    #[cfg(feature = "pg16")]
    {
        funcname_list =
            unsafe { pg_sys::stringToQualifiedNameList(cstr.as_ptr(), std::ptr::null_mut()) };
    }

    // TODO: Unless we can guarantee the exact matches for arg Oids, we'll have to assume that only one function
    // with this name exists. Non-lossy type conversion will be necessary. When non-lossy type conversion
    // is implemented, we can turn the null_mut into argtypes.
    Ok(pg_sys::LookupFuncName(
        funcname_list,
        -1,
        std::ptr::null_mut(),
        true,
    ))
}

unsafe fn func_list_from_name(funcname: &str) -> Result<pg_sys::FuncCandidateList, ParadeError> {
    let cstr = CString::new(funcname)?;

    let funcname_list: *mut pg_sys::List;
    #[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
    {
        funcname_list = unsafe { pg_sys::stringToQualifiedNameList(cstr.as_ptr()) };
    }
    #[cfg(feature = "pg16")]
    {
        funcname_list =
            unsafe { pg_sys::stringToQualifiedNameList(cstr.as_ptr(), std::ptr::null_mut()) };
    }

    Ok(pg_sys::FuncnameGetCandidates(
        funcname_list,
        -1,
        std::ptr::null_mut(),
        false,
        false,
        #[cfg(any(feature = "pg14", feature = "pg15", feature = "pg16"))]
        false,
        true,
    ))
}

unsafe fn udf_datafusion(args: &[ColumnarValue]) -> Result<ColumnarValue, DataFusionError> {
    let num_args = args.len();
    let mut num_rows = 1;

    let mut memory_context = PgMemoryContexts::new("udf_df_alloc");

    memory_context.switch_to(|_context| {
        let arg_oids =
            pg_sys::palloc0(std::mem::size_of::<pg_sys::Oid>() * num_args) as *mut pg_sys::Oid;
        for (arg_index, arg) in args.iter().enumerate().take(num_args).skip(1) {
            let dt = arg.data_type();
            let PgAttribute(oid, _typmod) = ArrowDataType(dt).try_into()?;
            *(arg_oids.add(arg_index)) = oid.value();

            if let ColumnarValue::Array(arg_arr) = &args[arg_index] {
                if arg_arr.len() > num_rows {
                    num_rows = arg_arr.len();
                }
            }
        }

        // Get function name - will always be a scalar value
        if let ColumnarValue::Scalar(ScalarValue::Utf8(Some(funcname))) = &args[0] {
            // Turn all arguments into arrays of result length
            let arg_arrays: Vec<ArrayRef> = args
                .iter()
                .take(num_args)
                .skip(1)
                .cloned()
                .map(|arg| arg.into_array(num_rows))
                .collect::<Result<Vec<_>, _>>()?;

            // Call function!
            // Follows the internal logic of FunctionCall9Coll
            let func_oid = func_oid_from_signature(funcname, arg_oids)?;

            // Create function call struct
            let flinfo = PgBox::<pg_sys::FmgrInfo>::alloc0();
            pg_sys::fmgr_info(func_oid, flinfo.as_ptr());
            // Prefer to use std::mem::offset_of! but it is still unstable as of rust 1.76.0.
            let fcinfo_size = max(
                std::mem::size_of::<pg_sys::FunctionCallInfoBaseData>(),
                memoffset::offset_of!(pg_sys::FunctionCallInfoBaseData, args)
                    + std::mem::size_of::<pg_sys::NullableDatum>() * (num_args - 1),
            );
            let fcinfo = pg_sys::palloc0(fcinfo_size) as *mut pg_sys::FunctionCallInfoBaseData;
            (*fcinfo).flinfo = flinfo.as_ptr();
            (*fcinfo).context = std::ptr::null_mut();
            (*fcinfo).resultinfo = std::ptr::null_mut();
            (*fcinfo).fncollation = pg_sys::Oid::INVALID;
            (*fcinfo).isnull = false;
            (*fcinfo).nargs = num_args as i16 - 1;

            // Call function on each set of arguments and push to result vector
            let mut result_vec = vec![];
            for row_index in 0..num_rows {
                for arg_index in 1..num_args {
                    let fcinfo_arg = (*fcinfo).args.as_mut_ptr().add(arg_index - 1);
                    (*fcinfo_arg).value = arg_arrays[arg_index - 1]
                        .get_datum(row_index)?
                        .ok_or(DataFusionError::Internal("No datum for type".to_string()))?;
                    (*fcinfo_arg).isnull = false;
                }

                let result_datum = (*(*fcinfo).flinfo)
                    .fn_addr
                    .ok_or(DataFusionError::Internal(
                        "Invalid function address for udf".to_string(),
                    ))?(fcinfo);
                result_vec.push(Some(result_datum));
            }

            // UDF return type
            let rettype = pg_sys::get_func_rettype(func_oid);

            Ok(ColumnarValue::Array(
                result_vec
                    .into_iter()
                    .into_arrow_array(rettype.into(), PgTypeMod(-1))?,
            ))
        } else {
            Err(DataFusionError::Internal("No funcname".to_string()))
        }
    })
}

pub unsafe fn loadfunction(funcname: &str) -> Result<(), ParadeError> {
    // Register all overloads with this function name
    let mut func_candidate = func_list_from_name(funcname)?;
    while !func_candidate.is_null() {
        let func_oid = (*func_candidate).oid;
        let arg_types = (*func_candidate).args.as_mut_ptr();
        let nargs = (*func_candidate).nargs;
        let ret_oid = pg_sys::get_func_rettype(func_oid);

        // Create vector of input types
        let mut input_types = vec![];
        input_types.push(DataType::Utf8); // function name
        for param_index in 0..nargs {
            let arg_oid = arg_types.add(param_index as usize);
            let ArrowDataType(datatype) =
                PgAttribute((*arg_oid).into(), PgTypeMod(-1)).try_into()?;
            input_types.push(datatype);
        }

        let ArrowDataType(return_type) = PgAttribute(ret_oid.into(), PgTypeMod(-1)).try_into()?;

        // Hardcoded typmod of -1 is okay for input and return types because it will immediately get
        //     converted back into an Oid ignoring the typmod in `udf_datafusion` when the function
        //     is called.
        let udf = create_udf(
            funcname,
            input_types,
            Arc::new(return_type),
            Volatility::Immutable,
            Arc::new(|args| unsafe { udf_datafusion(args) }),
        );

        Session::with_session_context(|context| {
            Box::pin(async move {
                context.register_udf(udf);
                Ok(())
            })
        })?;

        func_candidate = (*func_candidate).next;
    }

    Ok(())
}
