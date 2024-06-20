// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

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
