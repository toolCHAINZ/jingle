use crate::context::SleighContextBuilder;
use crate::context::image::{
    ImageProvider, ImageSection, ImageSectionIterator, Perms, SymbolInfo, SymbolLocation,
};
use crate::context::loaded::LoadedSleighContext;
use crate::{JingleSleighError, VarNode};
use object::{
    Architecture, BinaryFormat, Endianness, File, Object, ObjectSection, Section, SectionKind,
};
use std::cmp::{max, min};
use std::fmt::Debug;
use std::fs;
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub struct OwnedSection {
    data: Vec<u8>,
    perms: Perms,
    base_address: usize,
}

impl<'a> From<&'a OwnedSection> for ImageSection<'a> {
    fn from(value: &'a OwnedSection) -> Self {
        ImageSection {
            data: value.data.as_slice(),
            perms: value.perms.clone(),
            base_address: value.base_address,
        }
    }
}

impl TryFrom<Section<'_, '_>> for OwnedSection {
    type Error = JingleSleighError;

    fn try_from(value: Section) -> Result<Self, Self::Error> {
        let data = value
            .data()
            .map_err(|_| JingleSleighError::ImageLoadError)?
            .to_vec();
        Ok(OwnedSection {
            data,
            perms: map_sec_kind(&value.kind()),
            base_address: value.address() as usize,
        })
    }
}

#[derive(Debug)]
pub struct OwnedFile {
    sections: Vec<OwnedSection>,
}

impl OwnedFile {
    pub fn new(file: &File) -> Result<Self, JingleSleighError> {
        let mut sections = vec![];
        for x in file.sections().filter(|f| f.kind() == SectionKind::Text) {
            sections.push(x.try_into()?);
        }
        Ok(Self { sections })
    }
}

impl ImageProvider for OwnedFile {
    fn load(&self, vn: &VarNode, output: &mut [u8]) -> usize {
        let mut written = 0;
        output.fill(0);
        let output_start_addr = vn.offset as usize;
        let output_end_addr = output_start_addr + vn.size;
        if let Some(x) = self.get_section_info().find(|s| {
            output_start_addr >= s.base_address
                && output_start_addr < (s.base_address + s.data.len())
        }) {
            let input_start_addr = x.base_address;
            let input_end_addr = input_start_addr + x.data.len();
            let start_addr = max(input_start_addr, output_start_addr);
            let end_addr = max(min(input_end_addr, output_end_addr), start_addr);
            if end_addr > start_addr {
                let i_s = start_addr - x.base_address;
                let i_e = end_addr - x.base_address;
                let o_s = start_addr - vn.offset as usize;
                let o_e = end_addr - vn.offset as usize;
                let out_slice = &mut output[o_s..o_e];
                let in_slice = &x.data[i_s..i_e];
                out_slice.copy_from_slice(in_slice);
                written += end_addr - start_addr;
            }
        }
        written
    }

    fn has_full_range(&self, vn: &VarNode) -> bool {
        self.get_section_info().any(|s| {
            s.base_address <= vn.offset as usize
                && (s.base_address + s.data.len()) >= (vn.offset as usize + vn.size)
        })
    }

    fn get_section_info(&self) -> ImageSectionIterator<'_> {
        ImageSectionIterator::new(self.sections.iter().map(ImageSection::from))
    }
}

impl ImageProvider for File<'_> {
    fn load(&self, vn: &VarNode, output: &mut [u8]) -> usize {
        let mut written = 0;
        output.fill(0);
        let output_start_addr = vn.offset as usize;
        let output_end_addr = output_start_addr + vn.size;
        if let Some(x) = self.sections().find(|s| {
            output_start_addr >= s.address() as usize
                && output_start_addr < (s.address() + s.size()) as usize
        }) {
            if let Ok(data) = x.data() {
                let input_start_addr = x.address() as usize;
                let input_end_addr = input_start_addr + data.len();
                let start_addr = max(input_start_addr, output_start_addr);
                let end_addr = max(min(input_end_addr, output_end_addr), start_addr);
                if end_addr > start_addr {
                    let i_s = start_addr - x.address() as usize;
                    let i_e = end_addr - x.address() as usize;
                    let o_s = start_addr - vn.offset as usize;
                    let o_e = end_addr - vn.offset as usize;
                    let out_slice = &mut output[o_s..o_e];
                    let in_slice = &data[i_s..i_e];
                    out_slice.copy_from_slice(in_slice);
                    written += end_addr - start_addr;
                }
            }
        }
        written
    }

    fn has_full_range(&self, vn: &VarNode) -> bool {
        self.sections().any(|s| {
            s.address() <= vn.offset && (s.address() + s.size()) >= (vn.offset + vn.size as u64)
        })
    }

    fn get_section_info(&self) -> ImageSectionIterator<'_> {
        ImageSectionIterator::new(self.sections().filter_map(|s| {
            if let Ok(data) = s.data() {
                Some(ImageSection {
                    data,
                    base_address: s.address() as usize,
                    perms: map_sec_kind(&s.kind()),
                })
            } else {
                None
            }
        }))
    }

    fn resolve<T: AsRef<str>>(&self, t: T) -> Option<SymbolInfo> {
        let exp = self.exports().ok()?;
        let needle = t.as_ref().as_bytes();
        exp.iter().find(|e| e.name() == needle).map(|f| SymbolInfo {
            location: f.address(),
        })
    }
}

pub fn map_gimli_architecture(file: &File) -> Option<&'static str> {
    match &file.architecture() {
        Architecture::Unknown => None,
        Architecture::Aarch64 => {
            if file.format() == BinaryFormat::MachO {
                return Some("AARCH64:LE:64:AppleSilicon");
            }
            match file.endianness() {
                Endianness::Little => Some("AARCH64:LE:64:v8A"),
                Endianness::Big => Some("AARCH64:BE:64:v8A"),
            }
        }
        Architecture::Aarch64_Ilp32 => match file.endianness() {
            Endianness::Little => Some("AARCH64:LE:32:ilp32"),
            Endianness::Big => Some("AARCH64:BE:32:ilp32"),
        },
        Architecture::Arm => match file.endianness() {
            Endianness::Little => Some("ARM:LE:32:v8"),
            Endianness::Big => Some("ARM:BE:32:v8"),
        },
        Architecture::I386 => Some("x86:LE:32:default"),
        Architecture::X86_64 => Some("x86:LE:64:default"),
        Architecture::PowerPc64 => match file.endianness() {
            Endianness::Little => Some("PowerPC:LE:64:default"),
            Endianness::Big => Some("PowerPC:BE:64:default"),
        },
        Architecture::Xtensa => match file.endianness() {
            Endianness::Little => Some("Xtensa:LE:32:default"),
            Endianness::Big => Some("Xtensa:BE:32:default"),
        },
        _ => None,
    }
}

fn map_sec_kind(kind: &SectionKind) -> Perms {
    match kind {
        SectionKind::Unknown => Perms::RWX,
        SectionKind::Text => Perms::RX,
        SectionKind::Data => Perms::RW,
        SectionKind::ReadOnlyData => Perms::R,
        SectionKind::ReadOnlyDataWithRel => Perms::R,
        SectionKind::ReadOnlyString => Perms::R,
        SectionKind::UninitializedData => Perms::RW,
        _ => Perms::NONE,
    }
}

pub fn load_with_gimli<'a, P: AsRef<Path>, P2: AsRef<Path> + Debug>(
    p: P,
    ghidra_path: P2,
) -> Result<LoadedSleighContext<'a>, JingleSleighError> {
    let data = fs::read(p.as_ref()).map_err(|_| JingleSleighError::ImageLoadError)?;
    let f = object::File::parse(data.as_slice()).map_err(|_| JingleSleighError::ImageLoadError)?;
    let owned = OwnedFile::new(&f)?;
    let arch = map_gimli_architecture(&f).ok_or(JingleSleighError::InvalidLanguageId(format!(
        "{:?} (gimli)",
        f.architecture()
    )))?;
    let sleigh = SleighContextBuilder::load_ghidra_installation(ghidra_path)?.build(arch)?;
    let loaded = sleigh.initialize_with_image(owned)?;
    Ok(loaded)
}
