#[derive(PartialEq)]
pub enum FdwHandler {
    S3,
    LocalFile,
    Other,
}

impl FdwHandler {
    pub fn from(handler: &str) -> Self {
        match handler {
            "s3_fdw_handler" => Self::S3,
            "local_file_fdw_handler" => Self::LocalFile,
            _ => Self::Other,
        }
    }
}
