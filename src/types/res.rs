#![allow(dead_code)]

use std::path::Path;

use thiserror::Error;

use crate::fnv1a64;
use crate::raw::root::RED4ext as red;

#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
pub struct ResourcePath(red::ResourcePath);

impl PartialEq for ResourcePath {
    fn eq(&self, other: &Self) -> bool {
        self.0.hash == other.0.hash
    }
}

impl Eq for ResourcePath {}

impl PartialOrd for ResourcePath {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ResourcePath {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.hash.cmp(&other.0.hash)
    }
}

impl ResourcePath {
    pub const MAX_LENGTH: usize = 216;

    /// accepts non-sanitized path of any length,
    /// but final sanitized path length must be equals or inferior to 216 bytes
    fn new(path: &(impl AsRef<Path> + ?Sized)) -> Result<Self, ResourcePathError> {
        let sanitized = path
            .as_ref()
            .to_str()
            .unwrap()
            .trim_start_matches(|c| c == '\'' || c == '\"')
            .trim_end_matches(|c| c == '\'' || c == '\"')
            .trim_start_matches(|c| c == '/' || c == '\\')
            .trim_end_matches(|c| c == '/' || c == '\\')
            .split(|c| c == '/' || c == '\\')
            .filter(|comp| !comp.is_empty())
            .map(str::to_ascii_lowercase)
            .reduce(|mut acc, e| {
                acc.push('\\');
                acc.push_str(&e);
                acc
            })
            .ok_or(ResourcePathError::Empty)?;
        if sanitized.as_bytes().len() > Self::MAX_LENGTH {
            return Err(ResourcePathError::TooLong);
        }
        if Path::new(&sanitized)
            .components()
            .any(|x| !matches!(x, std::path::Component::Normal(_)))
        {
            return Err(ResourcePathError::NotCanonical);
        }
        Ok(Self(red::ResourcePath {
            hash: fnv1a64(&sanitized),
        }))
    }
}

#[derive(Debug, Error)]
pub enum ResourcePathError {
    #[error("resource path should not be empty")]
    Empty,
    #[error(
        "resource path should be less than {} characters",
        ResourcePath::MAX_LENGTH
    )]
    TooLong,
    #[error("resource path should be an absolute canonical path in an archive e.g. 'base\\mod\\character.ent'")]
    NotCanonical,
}

#[cfg(test)]
mod tests {
    use super::{red, ResourcePath};
    use crate::fnv1a64;

    #[test]
    fn resource_path() {
        assert_eq!(
            ResourcePath::default(),
            ResourcePath(red::ResourcePath { hash: 0 })
        );

        const TOO_LONG: &str = "base\\some\\archive\\path\\that\\is\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\long\\and\\above\\216\\bytes";
        assert!(TOO_LONG.as_bytes().len() > ResourcePath::MAX_LENGTH);
        assert!(ResourcePath::new(TOO_LONG).is_err());

        assert_eq!(
            ResourcePath::new("\'base/somewhere/in/archive/\'").unwrap(),
            ResourcePath(red::ResourcePath {
                hash: fnv1a64("base\\somewhere\\in\\archive")
            })
        );
        assert_eq!(
            ResourcePath::new("\"MULTI\\\\SOMEWHERE\\\\IN\\\\ARCHIVE\"").unwrap(),
            ResourcePath(red::ResourcePath {
                hash: fnv1a64("multi\\somewhere\\in\\archive")
            })
        );
        assert!(ResourcePath::new("..\\somewhere\\in\\archive\\custom.ent").is_err());
        assert!(ResourcePath::new("base\\somewhere\\in\\archive\\custom.ent").is_ok());
        assert!(ResourcePath::new("custom.ent").is_ok());
        assert!(ResourcePath::new(".custom.ent").is_ok());
    }
}
