use crate::VarNode;
use std::cmp::min;
use std::iter::once;
use std::ops::Range;

#[cfg(feature = "gimli")]
pub mod gimli;

pub type SymbolLocation = u64;

pub struct SymbolInfo {
    pub location: SymbolLocation, // todo: maybe other info goes in here later
}

pub trait ImageProvider {
    fn load(&self, vn: &VarNode, output: &mut [u8]) -> usize;

    fn has_full_range(&self, vn: &VarNode) -> bool;
    fn get_section_info(&self) -> ImageSectionIterator<'_>;

    fn get_bytes(&self, vn: &VarNode) -> Option<Vec<u8>> {
        let mut vec = vec![0u8; vn.size];
        let size = self.load(vn, &mut vec);
        if size < vn.size { None } else { Some(vec) }
    }

    fn resolve<T: AsRef<str>>(&self, t: T) -> Option<SymbolInfo> {
        None
    }
}

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
impl ImageProvider for &[u8] {
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

impl ImageProvider for Vec<u8> {
    fn load(&self, vn: &VarNode, output: &mut [u8]) -> usize {
        self.as_slice().load(vn, output)
    }

    fn has_full_range(&self, vn: &VarNode) -> bool {
        self.as_slice().has_full_range(vn)
    }

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

impl<T: ImageProvider> ImageProvider for &T {
    fn load(&self, vn: &VarNode, output: &mut [u8]) -> usize {
        (*self).load(vn, output)
    }

    fn has_full_range(&self, vn: &VarNode) -> bool {
        (*self).has_full_range(vn)
    }

    fn get_section_info(&self) -> ImageSectionIterator<'_> {
        (*self).get_section_info()
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
    use crate::context::image::{ImageProvider, ImageSection};
    #[test]
    fn test_vec_sections() {
        let data: Vec<u8> = vec![1, 2, 3];
        let sections: Vec<ImageSection> = data.get_section_info().collect();
        assert_ne!(sections, vec![])
    }
}
