pub mod benchlogs;

use serde::de::DeserializeOwned;
use std::path::PathBuf;

// A trait that abstracts the source of paths
pub trait PathSource {
    fn paths(&self) -> Box<dyn Iterator<Item = PathBuf>>;
}

// Implement PathSource for a glob pattern
impl PathSource for &String {
    fn paths(&self) -> Box<dyn Iterator<Item = PathBuf>> {
        // Use a globbing library to interpret the string as a glob pattern
        // and return an iterator over the matching paths
        Box::new(glob::glob(self).unwrap().filter_map(Result::ok))
    }
}

impl PathSource for &str {
    fn paths(&self) -> Box<dyn Iterator<Item = PathBuf>> {
        // Use a globbing library to interpret the string as a glob pattern
        // and return an iterator over the matching paths
        Box::new(glob::glob(self).unwrap().filter_map(Result::ok))
    }
}

// Implement PathSource for a vector of paths
impl PathSource for Vec<PathBuf> {
    fn paths(&self) -> Box<dyn Iterator<Item = PathBuf>> {
        // Simply iterate over the paths in the vector
        Box::new(self.clone().into_iter())
    }
}

#[allow(clippy::type_complexity)]
pub trait PathReader: DeserializeOwned + 'static {
    type Error;

    fn read_all<S: PathSource>(
        source: S,
    ) -> Result<Box<dyn Iterator<Item = Result<Self, Self::Error>>>, Self::Error>;

    fn read_ok<S: PathSource>(
        source: S,
        offset: usize,
        limit: usize,
    ) -> Result<Box<dyn Iterator<Item = Self>>, Self::Error> {
        Ok(Box::new(
            Self::read_all(source)?
                .filter_map(|r| r.ok())
                .skip(offset)
                .take(limit),
        ))
    }

    fn read_all_ok<S: PathSource>(
        source: S,
    ) -> Result<Box<dyn Iterator<Item = Self>>, Self::Error> {
        Ok(Box::new(Self::read_all(source)?.filter_map(|r| r.ok())))
    }
}
