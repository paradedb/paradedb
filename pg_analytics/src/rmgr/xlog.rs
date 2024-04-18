pub static XLOG_INSERT: u8 = 0x00;

#[derive(Debug, Clone)]
pub struct XLogInsertRecord {
    flags: u8,
}

impl XLogInsertRecord {
    pub fn new(flags: u8) -> Self {
        Self { flags }
    }

    pub fn flags(&self) -> u8 {
        self.flags
    }
}
