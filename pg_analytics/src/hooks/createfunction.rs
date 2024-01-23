use async_std::task;
use pgrx::*;

use crate::datafusion::context::DatafusionContext;
use crate::errors::ParadeError;
use crate::hooks::handler::DeltaHandler;

/*

Example from datafusion:

// First, declare the actual implementation of the calculation
let pow = Arc::new(|args: &[ColumnarValue]| {
    // in DataFusion, all `args` and output are dynamically-typed arrays, which means that we need to:
    // 1. cast the values to the type we want
    // 2. perform the computation for every element in the array (using a loop or SIMD) and construct the result

    // this is guaranteed by DataFusion based on the function's signature.
    assert_eq!(args.len(), 2);

    // Try to obtain row number
    let len = args
        .iter()
        .fold(Option::<usize>::None, |acc, arg| match arg {
            ColumnarValue::Scalar(_) => acc,
            ColumnarValue::Array(a) => Some(a.len()),
        });

    let inferred_length = len.unwrap_or(1);

    let arg0 = args[0].clone().into_array(inferred_length)?;
    let arg1 = args[1].clone().into_array(inferred_length)?;

    // 1. cast both arguments to f64. These casts MUST be aligned with the signature or this function panics!
    let base = as_float64_array(&arg0).expect("cast failed");
    let exponent = as_float64_array(&arg1).expect("cast failed");

    // this is guaranteed by DataFusion. We place it just to make it obvious.
    assert_eq!(exponent.len(), base.len());

    // 2. perform the computation
    let array = base
        .iter()
        .zip(exponent.iter())
        .map(|(base, exponent)| {
            match (base, exponent) {
                // in arrow, any value can be null.
                // Here we decide to make our UDF to return null when either base or exponent is null.
                (Some(base), Some(exponent)) => Some(base.powf(exponent)),
                _ => None,
            }
        })
        .collect::<Float64Array>();

    // `Ok` because no error occurred during the calculation (we should add one if exponent was [0, 1[ and the base < 0 because that panics!)
    // `Arc` because arrays are immutable, thread-safe, trait objects.
    Ok(ColumnarValue::from(Arc::new(array) as ArrayRef))
});

*/

pub unsafe fn createfunction(createfunction_stmt: *mut pg_sys::CreateFunctionStmt) -> Result<(), ParadeError> {
    // Get function name
    info!("funcname len: {:?}", (*(*createfunction_stmt).funcname).length);
    let funcname_list = (*createfunction_stmt).funcname;

    // funcname_list, parameters, returnType

    let mut funcname;
    let mut schemaname;

    if ((*funcname_list).length == 2) {
        schemaname = (*funcname_list).elements.add(0)
    }

    // OidFunctionCall0Coll for args 0 len

    // TODO: create function that calls the postgres function and assign this to the udf like above
    // Arc::new(|args: &[ColumnarValue]| {


    // }

    Ok(())
}
