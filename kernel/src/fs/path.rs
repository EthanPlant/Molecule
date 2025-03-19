//! Abstraction around file paths

use super::vfs::{VfsError, VfsResult};

/// Maximum path size
const PATH_MAX: usize = 4096;

/// Path delimeter
const PATH_DELIMETER: u8 = b'/';

/// Borroed file path
#[repr(transparent)]
#[derive(Debug)]
pub struct Path([u8]);

impl Path {
    /// Creates a path to root
    pub fn root() -> &'static Self {
        Self::new_unbounded(b"/")
    }

    /// Creates a new path from a given string
    pub fn new<T: AsRef<[u8]> + ?Sized>(s: &T) -> VfsResult<&Self> {
        let slice = s.as_ref();
        if slice.len() < PATH_MAX {
            Ok(Self::new_unbounded(slice))
        } else {
            Err(VfsError::NameTooLong)
        }
    }

    /// Create a new instance of the given path without checking its length
    pub fn new_unbounded<T: AsRef<[u8]> + ?Sized>(s: &T) -> &Self {
        // Safety: s can be safely casted to as u8 slice
        unsafe { &*(s.as_ref() as *const [u8] as *const Self) }
    }

    /// Returns slice of the bytes of the path
    pub const fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Tells whether the path is empty
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Tells whether a path is an absolute path
    pub fn is_absolute(&self) -> bool {
        self.0.first().cloned() == Some(PATH_DELIMETER)
    }

    /// Strips the path from the given prefix and returns the remaining components.
    pub fn strip_prefix<T: AsRef<Path>>(&self, prefix: T) -> Option<&Path> {
        let prefix = prefix.as_ref();
        let slice = self.0.strip_prefix(&prefix.0)?;
        Some(Self::new_unbounded(slice))
    }

    /// Returns an iterator over the path's components
    pub fn components(&self) -> Components {
        Components {
            path: self,
            front: 0,
            back: self.0.len(),
        }
    }

    /// Returns the final component of the path.
    pub fn file_name(&self) -> Option<&[u8]> {
        let comp = self.components().next_back()?;
        match comp {
            Component::RootDir => None,
            Component::CurDir => Some(b"."),
            Component::ParentDir => Some(b".."),
            Component::Normal(name) => Some(name),
        }
    }

    /// Returns the path without its final component
    pub fn parent(&self) -> Option<&Path> {
        let mut comps = self.components();
        let last = comps.next_back();
        last.and_then(move |p| match p {
            Component::RootDir => None,
            _ => Some(comps.as_path()),
        })
    }
}

impl AsRef<Path> for Path {
    fn as_ref(&self) -> &Path {
        self
    }
}

/// A component of a path.
pub enum Component<'a> {
    /// The root directory (`/`)
    RootDir,
    /// The current directory (`.`)
    CurDir,
    /// The parent directory (`..`)
    ParentDir,
    /// A normal component
    Normal(&'a [u8]),
}

impl<'a> From<&'a [u8]> for Component<'a> {
    fn from(value: &'a [u8]) -> Self {
        match value {
            b"." => Self::CurDir,
            b".." => Self::ParentDir,
            name => Self::Normal(name),
        }
    }
}

/// Iterator over a path's components
pub struct Components<'a> {
    path: &'a Path,
    front: usize,
    back: usize,
}

impl<'a> Components<'a> {
    fn is_finished(&self) -> bool {
        self.front >= self.back
    }

    fn as_slice(&self) -> &'a [u8] {
        &self.path.as_bytes()[self.front..self.back]
    }

    fn as_path(&self) -> &'a Path {
        Path::new_unbounded(self.as_slice())
    }

    fn next_inner(&mut self, backwards: bool) -> Option<Component<'a>> {
        let len = loop {
            if self.is_finished() {
                return None;
            }
            let slice = self.as_slice();
            let len = if !backwards {
                slice.iter().position(|c| *c == PATH_DELIMETER)
            } else {
                slice.iter().rev().position(|c| *c == PATH_DELIMETER)
            }
            .unwrap_or(slice.len());
            if len > 0 {
                break len;
            }
            let root = if !backwards {
                self.front += 1;
                self.front == 1
            } else {
                self.back -= 1;
                self.back == 0
            };
            if root {
                return Some(Component::RootDir);
            }
        };
        let slice = self.as_slice();
        let comp_slice = if !backwards {
            self.front += len;
            &slice[..len]
        } else {
            self.back -= len;
            &slice[(slice.len() - len)..]
        };
        Some(Component::from(comp_slice))
    }
}

impl<'a> Iterator for Components<'a> {
    type Item = Component<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_inner(false)
    }
}

impl<'a> DoubleEndedIterator for Components<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.next_inner(true)
    }
}
