use pgrx::*;
use std::sync::Arc;

use deltalake::datafusion::arrow::datatypes::DataType;
use deltalake::datafusion::common::DataFusionError;
use deltalake::datafusion::common::ScalarValue;
use deltalake::datafusion::logical_expr::{create_udf, ColumnarValue, Volatility};
use std::cmp::max;
use std::ffi::{CStr, CString};

use crate::datafusion::datatype::DatafusionMapProducer;
use crate::datafusion::datatype::DatafusionTypeTranslator;
use crate::datafusion::datatype::PostgresTypeTranslator;
use crate::datafusion::session::Session;
use crate::errors::ParadeError;

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
        #[cfg(any(feature = "pg15", feature = "pg16"))]
        false,
        true,
    ))
}

unsafe fn udf_datafusion(args: &[ColumnarValue]) -> Result<ColumnarValue, DataFusionError> {
    let num_args = args.len();
    let mut num_rows = 1;

    let mut memory_context = PgMemoryContexts::new("udf_df_alloc");

    let ret = memory_context.switch_to(|_context| {
        let arg_oids =
            pg_sys::palloc0(std::mem::size_of::<pg_sys::Oid>() * num_args) as *mut pg_sys::Oid;
        for (arg_index, arg) in args.iter().enumerate().take(num_args).skip(1) {
            let dt = arg.data_type();
            let (oid, _typmod): (PgOid, Option<i32>) =
                PostgresTypeTranslator::from_sql_data_type(dt.to_sql_data_type()?)?;
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
            let mut arg_arrays = vec![];
            for arg in args.iter().take(num_args).skip(1) {
                arg_arrays.push(arg.clone().into_array(num_rows)?);
            }

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
                    // (*fcinfo_arg).value = DatafusionMapProducer::index_datum(
                    //     arg_arrays[arg_index - 1].data_type().to_sql_data_type()?,
                    //     &arg_arrays[arg_index - 1],
                    //     row_index,
                    // )?
                    (*fcinfo_arg).value = arg_arrays[arg_index - 1].get_datum(row_index)?;
                    .ok_or(DataFusionError::Internal("No datum for type".to_string()))?;
                    (*fcinfo_arg).isnull = false;
                }

                let result_datum = (*(*fcinfo).flinfo)
                    .fn_addr
                    .ok_or(DataFusionError::Internal(
                        "Invalid function address for udf".to_string(),
                    ))?(fcinfo);
                result_vec.push(result_datum);
            }

            // UDF return type
            let rettype = pg_sys::get_func_rettype(func_oid);

            // Ok(ColumnarValue::Array(DatafusionMapProducer::array(
            //     DatafusionTypeTranslator::from_sql_data_type(
            //         PgOid::from(rettype).to_sql_data_type(None)?,
            //     )?,
            //     None,
            //     None,
            //     Some(&result_vec),
            //     0,
            // )?))
            Ok(result_vec.into_arrow_array()?)
        } else {
            Err(DataFusionError::Internal("No funcname".to_string()))
        }
    });

    drop(memory_context);

    ret
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
            input_types.push(ArrowDataType::try_from(PgAttribute(arg_oid, PgTypeMod(-1)))?);
            // input_types.push(DatafusionTypeTranslator::from_sql_data_type(
            //     PgOid::from(*arg_oid).to_sql_data_type(None)?,
            // )?);
        }

        // let return_type = DatafusionTypeTranslator::from_sql_data_type(
        //     PgOid::from(ret_oid).to_sql_data_type(None)?,
        // )?;

        let return_type = ArrowDataType::try_from(PgAttribute(arg_oid, PgTypeMod(-1)))?

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

pub unsafe fn createfunction(
    createfunction_stmt: *mut pg_sys::CreateFunctionStmt,
) -> Result<(), ParadeError> {
    // Drop any functions with the same name from the context and allow the next call to the UDF load them back

    let funcname = pg_sys::NameListToString((*createfunction_stmt).funcname);
    let _funcname_cstr = CStr::from_ptr(funcname);

    Session::with_session_context(|_context| {
        // TODO: need to deregister all UDFs of the same name from context
        //       this is necessary because the function signature might have changed
        //       and it will only be updated in the context if we deregister it first.
        // https://github.com/apache/arrow-datafusion/pull/9239

        Box::pin(async move { Ok(()) })
    })?;

    Ok(())
}