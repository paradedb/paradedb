use pgrx::pg_sys::*;
use pgrx::*;
use std::slice;

#[pg_guard]
pub unsafe fn detoast(tupdesc: &PgTupleDesc, slot: *mut TupleTableSlot) -> *mut Datum {
    let natts = (*tupdesc).len();
    let tts_isnull = (*slot).tts_isnull;
    let tts_values = (*slot).tts_values;

    // Convert to slice for readability
    let values = slice::from_raw_parts_mut(tts_values, natts);
    let isnull = slice::from_raw_parts_mut(tts_isnull, natts);

    for i in 0..natts {
        info!("At {}", i);
        // Don't detoast if null
        if isnull[i] {
            continue;
        }

        // Don't detoast if not varlena
        if let Some(attribute) = (*tupdesc).get(i) {
            if attribute.attlen != -1 {
                continue;
            }
        }

        let datum = values[i];
        let datum_as_varlena = PgVarlena::<pg_sys::Datum>::from_datum(datum).into_pg();

        // Don't detoast if the varlena is not in an "extended" format
        if varatt_is_4b_u(datum_as_varlena) {
            continue;
        }

        let detoasted_varlena = pg_detoast_datum(datum_as_varlena);
        values[i] = Datum::from(detoasted_varlena);
    }

    values.as_mut_ptr()
}
