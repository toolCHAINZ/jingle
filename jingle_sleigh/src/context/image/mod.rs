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
    fn image_sections(&self) -> ImageSectionIterator<'_>;

    /// Returns an iterator over all addresses in the image sections.
    /// Empty sections are excluded from iteration.
    fn addresses(&self) -> impl Iterator<Item = u64> + '_
    where
        Self: Sized,
    {
        self.image_sections()
            .filter(|s| !s.data.is_empty())
            .flat_map(|s|(s.base_address as u64)..((s.base_address + s.data.len()) as u64))
    }

    /// Returns an iterator over the address ranges of all sections.
    /// Empty sections are excluded from iteration.
    fn ranges(&self) -> impl Iterator<Item = Range<u64>> + '_
    where
        Self: Sized,
    {
        self.image_sections()
            .filter(|s| !s.data.is_empty())
            .map(|s| (s.base_address as u64)..((s.base_address + s.data.len()) as u64))
    }

    /// Returns an iterator over addresses in sections matching the required permissions.
    /// Only sections whose permissions satisfy the required permissions are included.
    /// Empty sections are excluded from iteration.
    fn addresses_with_perms(&self, required: &Perms) -> impl Iterator<Item = u64> + '_
    where
        Self: Sized,
    {
        let required = required.clone();
        self.image_sections()
            .filter(move |s| !s.data.is_empty() && s.perms.satisfies(&required))
            .flat_map(|s|(s.base_address as u64)..((s.base_address + s.data.len()) as u64))
    }

    /// Returns an iterator over ranges in sections matching the required permissions.
    /// Only sections whose permissions satisfy the required permissions are included.
    /// Empty sections are excluded from iteration.
    fn ranges_with_perms(&self, required: &Perms) -> impl Iterator<Item = Range<u64>> + '_
    where
        Self: Sized,
    {
        let required = required.clone();
        self.image_sections()
            .filter(move |s| !s.data.is_empty() && s.perms.satisfies(&required))
            .map(|s| (s.base_address as u64)..((s.base_address + s.data.len()) as u64))
    }
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
    fn image_sections(&self) -> ImageSectionIterator<'_> {
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
    fn image_sections(&self) -> ImageSectionIterator<'_> {
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
    fn image_sections(&self) -> ImageSectionIterator<'_> {
        (*self).image_sections()
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

    /// Check if these permissions satisfy the required permissions.
    /// Returns `true` if all required permissions are present in `self`.
    pub fn satisfies(&self, required: &Perms) -> bool {
        (!required.read || self.read)
            && (!required.write || self.write)
            && (!required.exec || self.exec)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageSection<'a> {
    pub data: &'a [u8],
    pub base_address: usize,
    pub perms: Perms,
}

#[cfg(test)]
mod tests {
    use crate::context::image::{ImageSection, ImageSections, Perms};

    #[test]
    fn test_vec_sections() {
        let data: Vec<u8> = vec![1, 2, 3];
        let sections: Vec<ImageSection> = data.image_sections().collect();
        assert_ne!(sections, vec![])
    }

    #[test]
    fn test_perms_satisfies() {
        // Test exact match
        assert!(Perms::R.satisfies(&Perms::R));
        assert!(Perms::RW.satisfies(&Perms::RW));
        assert!(Perms::RX.satisfies(&Perms::RX));
        assert!(Perms::RWX.satisfies(&Perms::RWX));

        // Test superset satisfies subset
        assert!(Perms::RWX.satisfies(&Perms::R));
        assert!(Perms::RWX.satisfies(&Perms::RW));
        assert!(Perms::RWX.satisfies(&Perms::RX));
        assert!(Perms::RW.satisfies(&Perms::R));
        assert!(Perms::RX.satisfies(&Perms::R));

        // Test subset does not satisfy superset
        assert!(!Perms::R.satisfies(&Perms::RW));
        assert!(!Perms::R.satisfies(&Perms::RX));
        assert!(!Perms::R.satisfies(&Perms::RWX));
        assert!(!Perms::RW.satisfies(&Perms::RWX));
        assert!(!Perms::RX.satisfies(&Perms::RWX));

        // Test NONE
        assert!(Perms::NONE.satisfies(&Perms::NONE));
        assert!(Perms::R.satisfies(&Perms::NONE));
        assert!(Perms::RWX.satisfies(&Perms::NONE));
        assert!(!Perms::NONE.satisfies(&Perms::R));
    }

    #[test]
    fn test_addresses_iteration() {
        let data: Vec<u8> = vec![0xAA, 0xBB, 0xCC];
        let addresses: Vec<u64> = data.addresses().collect();
        assert_eq!(addresses, vec![0, 1, 2]);
    }

    #[test]
    fn test_ranges_iteration() {
        let data: Vec<u8> = vec![0xAA, 0xBB, 0xCC];
        let ranges: Vec<_> = data.ranges().collect();
        assert_eq!(ranges, vec![0..3]);
    }

    #[test]
    fn test_addresses_with_perms() {
        let data: Vec<u8> = vec![0xAA, 0xBB, 0xCC];

        // Should get addresses when permissions match
        let addresses: Vec<u64> = data.addresses_with_perms(&Perms::R).collect();
        assert_eq!(addresses, vec![0, 1, 2]);

        // Should get addresses when asking for subset of available perms
        let addresses: Vec<u64> = data.addresses_with_perms(&Perms::RX).collect();
        assert_eq!(addresses, vec![0, 1, 2]);

        // Should get no addresses when asking for write permission (not available)
        let addresses: Vec<u64> = data.addresses_with_perms(&Perms::RW).collect();
        assert_eq!(addresses.len(), 0);
    }

    #[test]
    fn test_ranges_with_perms() {
        let data: Vec<u8> = vec![0xAA, 0xBB, 0xCC];

        // Should get range when permissions match
        let ranges: Vec<_> = data.ranges_with_perms(&Perms::R).collect();
        assert_eq!(ranges, vec![0..3]);

        // Should get no ranges when asking for write permission
        let ranges: Vec<_> = data.ranges_with_perms(&Perms::RW).collect();
        assert_eq!(ranges.len(), 0);
    }

    // Test with a custom multi-section image
    struct MultiSectionImage {
        sections: Vec<(Vec<u8>, usize, Perms)>,
    }

    impl ImageSections for MultiSectionImage {
        fn image_sections(&self) -> crate::context::image::ImageSectionIterator<'_> {
            crate::context::image::ImageSectionIterator::new(self.sections.iter().map(
                |(data, base, perms)| ImageSection {
                    data: data.as_slice(),
                    base_address: *base,
                    perms: perms.clone(),
                },
            ))
        }
    }

    #[test]
    fn test_multi_section_addresses() {
        let img = MultiSectionImage {
            sections: vec![
                (vec![0x01, 0x02], 0x1000, Perms::R),
                (vec![0x03, 0x04, 0x05], 0x2000, Perms::RW),
                (vec![0x06], 0x3000, Perms::RX),
            ],
        };

        let addresses: Vec<u64> = img.addresses().collect();
        assert_eq!(
            addresses,
            vec![0x1000, 0x1001, 0x2000, 0x2001, 0x2002, 0x3000]
        );
    }

    #[test]
    fn test_multi_section_ranges() {
        let img = MultiSectionImage {
            sections: vec![
                (vec![0x01, 0x02], 0x1000, Perms::R),
                (vec![0x03, 0x04, 0x05], 0x2000, Perms::RW),
                (vec![0x06], 0x3000, Perms::RX),
            ],
        };

        let ranges: Vec<_> = img.ranges().collect();
        assert_eq!(ranges, vec![0x1000..0x1002, 0x2000..0x2003, 0x3000..0x3001]);
    }

    #[test]
    fn test_multi_section_addresses_with_perms() {
        let img = MultiSectionImage {
            sections: vec![
                (vec![0x01, 0x02], 0x1000, Perms::R),
                (vec![0x03, 0x04, 0x05], 0x2000, Perms::RW),
                (vec![0x06], 0x3000, Perms::RX),
            ],
        };

        // Only read-only sections
        let addresses: Vec<u64> = img.addresses_with_perms(&Perms::R).collect();
        assert_eq!(
            addresses,
            vec![0x1000, 0x1001, 0x2000, 0x2001, 0x2002, 0x3000]
        );

        // Only writable sections
        let addresses: Vec<u64> = img.addresses_with_perms(&Perms::RW).collect();
        assert_eq!(addresses, vec![0x2000, 0x2001, 0x2002]);

        // Only executable sections
        let addresses: Vec<u64> = img.addresses_with_perms(&Perms::RX).collect();
        assert_eq!(addresses, vec![0x3000]);
    }

    #[test]
    fn test_multi_section_ranges_with_perms() {
        let img = MultiSectionImage {
            sections: vec![
                (vec![0x01, 0x02], 0x1000, Perms::R),
                (vec![0x03, 0x04, 0x05], 0x2000, Perms::RW),
                (vec![0x06], 0x3000, Perms::RX),
            ],
        };

        // Only writable sections
        let ranges: Vec<_> = img.ranges_with_perms(&Perms::RW).collect();
        assert_eq!(ranges, vec![0x2000..0x2003]);

        // Only executable sections
        let ranges: Vec<_> = img.ranges_with_perms(&Perms::RX).collect();
        assert_eq!(ranges, vec![0x3000..0x3001]);
    }

    #[test]
    fn test_empty_sections_excluded() {
        let img = MultiSectionImage {
            sections: vec![
                (vec![0x01, 0x02], 0x1000, Perms::R),
                (vec![], 0x2000, Perms::RW), // Empty section
                (vec![0x06], 0x3000, Perms::RX),
            ],
        };

        // Empty section should be excluded from addresses
        let addresses: Vec<u64> = img.addresses().collect();
        assert_eq!(addresses, vec![0x1000, 0x1001, 0x3000]);

        // Empty section should be excluded from ranges
        let ranges: Vec<_> = img.ranges().collect();
        assert_eq!(ranges, vec![0x1000..0x1002, 0x3000..0x3001]);

        // Empty section should be excluded even if permissions match
        let addresses: Vec<u64> = img.addresses_with_perms(&Perms::RW).collect();
        assert_eq!(addresses.len(), 0);
    }
}
