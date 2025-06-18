use pgrx::pg_sys;

pub struct Latch(*mut pg_sys::Latch);

impl Latch {
    pub unsafe fn new() -> Self {
        Self(pg_sys::MyLatch)
    }

    pub unsafe fn wait(&self, ms: i64) {
        let events = pg_sys::WL_LATCH_SET as i32
            | pg_sys::WL_TIMEOUT as i32
            | pg_sys::WL_EXIT_ON_PM_DEATH as i32;
        pg_sys::WaitLatch(self.0, events, ms, pg_sys::PG_WAIT_EXTENSION);
        pg_sys::ResetLatch(self.0);
    }
}
