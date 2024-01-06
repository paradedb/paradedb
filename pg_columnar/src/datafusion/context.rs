use datafusion::prelude::SessionContext;
use lazy_static::lazy_static;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

lazy_static! {
    pub static ref CONTEXT: RwLock<Option<SessionContext>> = RwLock::new(None);
}

pub struct DatafusionContext;

impl<'a> DatafusionContext {
    pub fn with_read<F, R>(f: F) -> R
    where
        F: FnOnce(&SessionContext) -> R,
    {
        let context_lock = CONTEXT.read();
        let context = context_lock
            .as_ref()
            .ok_or("Run SELECT paradedb.init(); first.")
            .expect("No columnar context found");
        f(context)
    }

    #[allow(dead_code)]
    pub fn with_write<F, R>(f: F) -> R
    where
        F: FnOnce(&mut SessionContext) -> R,
    {
        let mut context_lock = CONTEXT.write();
        let context = context_lock
            .as_mut()
            .ok_or("Run SELECT paradedb.init(); first.")
            .expect("No columnar context found");
        f(context)
    }

    #[allow(dead_code)]
    pub fn read_lock() -> Result<RwLockReadGuard<'a, Option<SessionContext>>, String> {
        Ok(CONTEXT.read())
    }

    pub fn write_lock() -> Result<RwLockWriteGuard<'a, Option<SessionContext>>, String> {
        Ok(CONTEXT.write())
    }
}
