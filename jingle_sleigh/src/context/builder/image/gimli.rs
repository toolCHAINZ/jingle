use crate::context::builder::image::Perms;
use crate::context::{Image, ImageSection};
use crate::{JingleSleighError, VarNode};
use crate::JingleSleighError::ImageLoadError;
use object::{Architecture, Endianness, File, Object, ObjectSection, SectionKind};
use tracing::{event, instrument, Level};
use crate::context::image::ImageProvider;

impl<'a> ImageProvider for File<'a>{
    fn load(&self, vn: &VarNode, output: &mut [u8]) -> usize {
        todo!()
    }

    fn has_full_range(&self, vn: &VarNode) -> bool {
        todo!()
    }
}

fn map_kind(kind: &SectionKind) -> Perms {
    Perms {
        exec: matches!(kind, SectionKind::Text),
        write: matches!(kind, SectionKind::Data)
            && !matches!(
                kind,
                SectionKind::ReadOnlyData
                    | SectionKind::ReadOnlyString
                    | SectionKind::ReadOnlyDataWithRel
            ),
        read: matches!(
            kind,
            SectionKind::Data
                | SectionKind::ReadOnlyData
                | SectionKind::ReadOnlyString
                | SectionKind::ReadOnlyDataWithRel
        ),
    }
}

pub fn map_gimli_architecture(file: &File) -> Option<&'static str> {
    match &file.architecture() {
        Architecture::Unknown => None,
        Architecture::Aarch64 => match file.endianness() {
            Endianness::Little => Some("AARCH64:LE:64:v8A"),
            Endianness::Big => Some("AARCH64:BE:64:v8A"),
        },
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
