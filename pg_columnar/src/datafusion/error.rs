use deltalake::arrow::error::ArrowError;
use deltalake::datafusion::common::DataFusionError;
use deltalake::errors::DeltaTableError;

pub fn datafusion_err_to_string() -> impl Fn(DataFusionError) -> String {
    move |dfe: DataFusionError| -> String { format!("{}", dfe) }
}

pub fn delta_err_to_string() -> impl Fn(DeltaTableError) -> String {
    move |dte: DeltaTableError| -> String { format!("{}", dte) }
}

pub fn arrow_err_to_string() -> impl Fn(ArrowError) -> String {
    move |ae: ArrowError| -> String { format!("{}", ae) }
}
