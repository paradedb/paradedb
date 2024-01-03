use datafusion::common::DataFusionError;

pub fn datafusion_err_to_string() -> impl Fn(DataFusionError) -> String {
    move |dfe: DataFusionError| -> String { format!("{}", dfe) }
}
