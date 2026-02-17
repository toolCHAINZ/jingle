use crate::{JingleSleighError, VarNode};
use std::cmp::min;
use std::iter::once;
use std::ops::Range;

#[cfg(feature = "gimli")]
pub mod gimli;

pub type SymbolLocation = u64;

#[derive(Clone, Debug)]
pub struct SymbolInfo {
    pub location: SymbolLocation, // todo: maybe other info goes in here later
}

/// Core trait for types to communicate with Sleigh over FFI.
///
/// This trait contains only the essential methods required for the FFI boundary:
/// - `load`: Read bytes from a varnode location into a buffer
/// - `has_full_range`: Check if a varnode range is fully available
///
/// These methods are required for all image types that interact with Sleigh.
pub trait SleighImageCore {
    /// Load bytes from the given varnode location into the output buffer.
    /// Returns the number of bytes successfully written.
    fn load(&self, vn: &VarNode, output: &mut [u8]) -> usize;

    /// Check if the full range specified by the varnode is available in this image.
    fn has_full_range(&self, vn: &VarNode) -> bool;
}

/// Extension trait providing convenient byte-reading functionality.
///
/// This trait is automatically implemented for all types implementing `SleighImageCore`,
/// providing a higher-level `get_bytes` method that allocates and returns a `Vec<u8>`.
pub trait ImageBytes: SleighImageCore {
    /// Read the byte range specified by the varnode, returning a vector.
    /// Returns `None` if the full range is not available.
    fn get_bytes(&self, vn: &VarNode) -> Option<Vec<u8>> {
        let mut vec = vec![0u8; vn.size];
        let size = self.load(vn, &mut vec);
        if size < vn.size { None } else { Some(vec) }
    }
}

// Blanket implementation: all SleighImageCore types automatically get ImageBytes
impl<T: SleighImageCore> ImageBytes for T {}

/// Trait for image types that can provide section/segment information.
///
/// This trait is separate from `SleighImageCore` because not all image sources
/// have meaningful section boundaries (e.g., raw byte slices may represent a
/// single contiguous region).
pub trait ImageSections {
    /// Returns an iterator over the sections/segments in this image.
    fn get_section_info(&self) -> ImageSectionIterator<'_>;
}

/// Trait for image types that support symbol resolution.
///
/// **Note**: This trait is currently experimental and not widely used in the codebase.
/// It is provided for future extensibility and integration with symbol tables or
/// debugging information.
pub trait SymbolResolver {
    /// Resolve a symbol name to its location information.
    /// Returns `None` if the symbol is not found.
    fn resolve(&self, name: &str) -> Option<SymbolInfo>;
}

/// An image that can also inform sleigh about its architecture.
///
/// This trait extends `SleighImage` (which includes core FFI, sections, and byte reading)
/// and adds architecture identification capability.
pub trait SleighArchImage: SleighImage {
    /// Returns the Sleigh architecture identifier string (e.g., "x86:LE:64:default").
    fn architecture_id(&self) -> Result<&str, JingleSleighError>;
}

/// Combined trait for complete image support including core FFI, sections, and byte reading.
///
/// This trait is used as a convenience bound for APIs that need full image functionality.
/// It combines `SleighImageCore` (FFI essentials), `ImageSections` (section information),
/// and `ImageBytes` (convenient byte reading via blanket impl).
pub trait SleighImage: SleighImageCore + ImageSections + ImageBytes {}

// Blanket implementation: any type implementing core traits gets SleighImage
impl<T: SleighImageCore + ImageSections> SleighImage for T {}

pub struct ImageSectionIterator<'a> {
    iter: Box<dyn Iterator<Item = ImageSection<'a>> + 'a>,
}

impl<'a> ImageSectionIterator<'a> {
    pub fn new<T: Iterator<Item = ImageSection<'a>> + 'a>(iter: T) -> Self {
        Self {
            iter: Box::new(iter),
        }
    }
}

impl<'a> Iterator for ImageSectionIterator<'a> {
    type Item = ImageSection<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}
impl SleighImageCore for &[u8] {
    fn load(&self, vn: &VarNode, output: &mut [u8]) -> usize {
        //todo: check the space. Ignoring for now
        let vn_range: Range<usize> = Range::from(vn);
        let vn_range = Range {
            start: vn_range.start,
            end: min(vn_range.end, self.len()),
        };
        if let Some(s) = self.get(vn_range) {
            if let Some(o) = output.get_mut(0..s.len()) {
                o.copy_from_slice(s)
            }
            let o_len = output.len();
            if let Some(o) = output.get_mut(s.len()..o_len) {
                o.fill(0);
            }
            s.len()
        } else {
            output.fill(0);
            0
        }
    }

    fn has_full_range(&self, vn: &VarNode) -> bool {
        let vn_range: Range<usize> = Range::from(vn);
        vn_range.start < self.len() && vn_range.end <= self.len()
    }
}

impl ImageSections for &[u8] {
    fn get_section_info(&self) -> ImageSectionIterator<'_> {
        ImageSectionIterator::new(once(ImageSection {
            data: self,
            base_address: 0,
            perms: Perms {
                read: true,
                write: false,
                exec: true,
            },
        }))
    }
}

impl SleighImageCore for Vec<u8> {
    fn load(&self, vn: &VarNode, output: &mut [u8]) -> usize {
        self.as_slice().load(vn, output)
    }

    fn has_full_range(&self, vn: &VarNode) -> bool {
        self.as_slice().has_full_range(vn)
    }
}

impl ImageSections for Vec<u8> {
    fn get_section_info(&self) -> ImageSectionIterator<'_> {
        ImageSectionIterator::new(once(ImageSection {
            data: self,
            base_address: 0,
            perms: Perms {
                read: true,
                write: false,
                exec: true,
            },
        }))
    }
}

impl<T: SleighImageCore> SleighImageCore for &T {
    fn load(&self, vn: &VarNode, output: &mut [u8]) -> usize {
        (*self).load(vn, output)
    }

    fn has_full_range(&self, vn: &VarNode) -> bool {
        (*self).has_full_range(vn)
    }
}

impl<T: ImageSections> ImageSections for &T {
    fn get_section_info(&self) -> ImageSectionIterator<'_> {
        (*self).get_section_info()
    }
}

impl<T: SymbolResolver> SymbolResolver for &T {
    fn resolve(&self, t: &str) -> Option<SymbolInfo> {
        (*self).resolve(t)
    }
}

impl<T: SleighArchImage> SleighArchImage for &T {
    fn architecture_id(&self) -> Result<&str, JingleSleighError> {
        (*self).architecture_id()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Perms {
    pub read: bool,
    pub write: bool,
    pub exec: bool,
}

impl Perms {
    pub const RWX: Perms = Perms {
        read: true,
        write: true,
        exec: true,
    };
    pub const RX: Perms = Perms {
        read: true,
        write: false,
        exec: true,
    };

    pub const RW: Perms = Perms {
        read: true,
        write: true,
        exec: false,
    };
    pub const R: Perms = Perms {
        read: true,
        write: false,
        exec: false,
    };

    pub const NONE: Perms = Perms {
        read: false,
        write: false,
        exec: false,
    };
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageSection<'a> {
    pub data: &'a [u8],
    pub base_address: usize,
    pub perms: Perms,
}

#[cfg(test)]
mod tests {
    use crate::context::image::{ImageSection, ImageSections};
    #[test]
    fn test_vec_sections() {
        let data: Vec<u8> = vec![1, 2, 3];
        let sections: Vec<ImageSection> = data.get_section_info().collect();
        assert_ne!(sections, vec![])
    }
}
