use crate::context::builder::image::Perms;
use crate::error::JingleSleighError;
use crate::error::JingleSleighError::ImageLoadError;
use crate::ffi::image::bridge::{Image, ImageSection};
use elf::abi::{PF_R, PF_W, PF_X, PT_LOAD};
use elf::endian::EndianParse;
use elf::ElfBytes;
use std::cmp::min;

impl<'data, E: EndianParse> TryFrom<ElfBytes<'data, E>> for Image {
    type Error = JingleSleighError;

    fn try_from(value: ElfBytes<E>) -> Result<Self, Self::Error> {
        let mut img: Image = Image { sections: vec![] };
        let segments = value.segments().ok_or(ImageLoadError)?;
        for hdr in segments.iter().filter(|seg| seg.p_type == PT_LOAD) {
            let addr = hdr.p_vaddr;
            let mem_size = hdr.p_memsz;
            let flags = hdr.p_flags;

            let file_data = value.segment_data(&hdr)?;

            let perms = Perms {
                exec: (flags & PF_X) == PF_X,
                write: (flags & PF_W) == PF_W,
                read: (flags & PF_R) == PF_R,
            };
            let mut data = vec![0; mem_size as usize];
            let len = min(mem_size as usize, file_data.len());
            data[0..len].copy_from_slice(&file_data[0..len]);
            img.sections.push(ImageSection {
                perms,
                base_address: addr as usize,
                data,
            })
        }
        Ok(img)
    }
}

#[cfg(test)]
mod tests {
    use crate::ffi::image::bridge::Image;
    use elf::endian::AnyEndian;
    use elf::ElfBytes;

    // #[test]
    fn test_elf() {
        let path = std::path::PathBuf::from("../bin/vuln");
        let file_data = std::fs::read(path).unwrap();
        let slice = file_data.as_slice();
        let file = ElfBytes::<AnyEndian>::minimal_parse(slice).unwrap();
        let _img = Image::try_from(file).unwrap();
    }
}
