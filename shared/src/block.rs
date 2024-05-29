#[macro_export]
macro_rules! block_on {
    ($block:expr) => {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => handle.block_on($block),
            Err(err) => {
                panic!(
                    "No tokio runtime started at 'block_on!()' in {}:{}... {err}",
                    file!(),
                    line!()
                )
            }
        }
    };
}
