#[cfg(feature = "elf")]
pub mod elf;
#[cfg(feature = "gimli")]
pub mod gimli;

pub use crate::ffi::image::bridge::{Image, ImageSection, Perms};
use std::ops::Range;

impl Image {
    pub fn get_range(&self) -> Option<Range<usize>> {
        let min = self.sections.iter().map(|s| s.base_address).min();
        let max = self
            .sections
            .iter()
            .map(|s| s.base_address + s.data.len())
            .max();
        min.zip(max).map(|(min, max)| min..max)
    }

    pub fn sections(&self) -> &[ImageSection] {
        &self.sections
    }

    pub fn contains_address(&self, addr: u64) -> bool {
        self.sections
            .iter()
            .any(|s| s.base_address <= addr as usize && (s.base_address + s.data.len()) >= addr as usize)
    }

    pub fn contains_range(&self, mut range: Range<u64>) -> bool {
        range.all(|i| self.contains_address(i))
    }
}

impl From<&[u8]> for Image {
    fn from(value: &[u8]) -> Self {
        Self {
            sections: vec![ImageSection {
                data: value.to_vec(),
                perms: Perms {
                    read: true,
                    write: true,
                    exec: true,
                },
                base_address: 0,
            }],
        }
    }
}

impl From<Vec<u8>> for Image {
    fn from(value: Vec<u8>) -> Self {
        Self {
            sections: vec![ImageSection {
                data: value,
                perms: Perms {
                    read: true,
                    write: true,
                    exec: true,
                },
                base_address: 0,
            }],
        }
    }
}
