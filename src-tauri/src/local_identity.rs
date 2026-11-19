use std::path::{Component, Path, PathBuf};

pub mod watch;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::LiveIdentity;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::LiveIdentity;

pub(crate) fn canonical_path(path: &Path) -> std::io::Result<PathBuf> {
    let path = std::fs::canonicalize(path)?;
    #[cfg(windows)]
    return windows::ordinary_local_path(&path);
    #[cfg(not(windows))]
    Ok(path)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiveObservation {
    Present(Locator),
    Unavailable,
    Removed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Locator(PathBuf);

impl Locator {
    pub fn new(path: PathBuf) -> Option<Self> {
        let path = crate::filesystem::validate_path(path).ok()?;
        (!path.components().any(|part| part == Component::ParentDir)
            && !path.as_os_str().as_encoded_bytes().contains(&0))
        .then_some(Self(path))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn after_tree_move(&self, source: &Locator, destination: &Locator) -> Option<Self> {
        let relative = self.0.strip_prefix(&source.0).ok()?;
        if relative.as_os_str().is_empty() {
            Some(destination.clone())
        } else {
            Self::new(destination.0.join(relative))
        }
    }

    #[cfg(unix)]
    pub fn encode(&self) -> Vec<u8> {
        use std::os::unix::ffi::OsStrExt;
        self.0.as_os_str().as_bytes().to_vec()
    }

    #[cfg(unix)]
    pub fn decode(bytes: Vec<u8>) -> Option<Self> {
        use std::{ffi::OsString, os::unix::ffi::OsStringExt};
        Self::new(PathBuf::from(OsString::from_vec(bytes)))
    }

    #[cfg(windows)]
    pub fn encode(&self) -> Vec<u8> {
        use std::os::windows::ffi::OsStrExt;
        self.0
            .as_os_str()
            .encode_wide()
            .flat_map(u16::to_le_bytes)
            .collect()
    }

    #[cfg(windows)]
    pub fn decode(bytes: Vec<u8>) -> Option<Self> {
        use std::{ffi::OsString, os::windows::ffi::OsStringExt};
        let (words, remainder) = bytes.as_chunks::<2>();
        if !remainder.is_empty() {
            return None;
        }
        let words: Vec<u16> = words.iter().copied().map(u16::from_le_bytes).collect();
        Self::new(PathBuf::from(OsString::from_wide(&words)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relocated_descendants_are_matched_by_components() {
        let root = std::env::temp_dir();
        let source = Locator::new(root.join("folder")).unwrap();
        let destination = Locator::new(root.join("renamed")).unwrap();
        let child = Locator::new(root.join("folder").join("caffè.txt")).unwrap();
        let neighbor = Locator::new(root.join("folder-copy").join("caffè.txt")).unwrap();
        assert_eq!(
            source.after_tree_move(&source, &destination),
            Some(destination.clone())
        );
        assert_eq!(
            child.after_tree_move(&source, &destination).unwrap().path(),
            root.join("renamed").join("caffè.txt")
        );
        assert_eq!(neighbor.after_tree_move(&source, &destination), None);
        assert!(Locator::new(root.join("folder").join("..").join("elsewhere")).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn encoded_path_preserves_native_bytes() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
        let path = std::env::temp_dir().join(OsStr::from_bytes(b"synthetic-\xff"));
        let locator = Locator::new(path.clone()).unwrap();
        assert_eq!(Locator::decode(locator.encode()).unwrap().path(), path);
        assert!(Locator::decode(b"/with\0nul".to_vec()).is_none());
    }
}
