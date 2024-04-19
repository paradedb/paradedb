pub static XLOG_INSERT: u8 = 0x00;

#[derive(Debug, Clone)]
pub struct XLogInsertRecord {
    flags: u8,
    row_number: i64,
}

impl XLogInsertRecord {
    pub fn new(flags: u8, row_number: i64) -> Self {
        Self { flags, row_number }
    }

    pub fn flags(&self) -> u8 {
        self.flags
    }

    pub fn row_number(&self) -> i64 {
        self.row_number
    }
}
