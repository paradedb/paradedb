use serde::de::DeserializeOwned;
use std::marker::PhantomData;

use crate::writer::Handler;

pub struct TestHandler<T: DeserializeOwned, F: Fn(T)> {
    handler: F,
    marker: PhantomData<T>,
}

impl<T: DeserializeOwned, F: Fn(T)> TestHandler<T, F> {
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            marker: PhantomData,
        }
    }
}

impl<T: DeserializeOwned, F: Fn(T)> Handler<T> for TestHandler<T, F> {
    fn handle(&mut self, request: T) -> Result<(), crate::writer::ServerError> {
        (self.handler)(request);
        Ok(())
    }
}
