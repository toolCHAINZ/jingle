use crate::context::builder::image::Perms;
use crate::context::{Image, ImageSection};
use crate::JingleSleighError;
use crate::JingleSleighError::ImageLoadError;
use object::{Architecture, Endianness, File, Object, ObjectSection, SectionKind};
use tracing::{event, instrument, Level};

impl<'d> TryFrom<File<'d>> for Image {
    type Error = JingleSleighError;
    #[instrument(skip_all)]
    fn try_from(value: File) -> Result<Self, Self::Error> {
        let mut img: Image = Image { sections: vec![] };
        for x in value
            .sections()
            .filter(|s| matches!(s.kind(), SectionKind::Text))
        {
            let base_address = x.address();
            let data = x.data().map_err(|_| ImageLoadError)?.to_vec();
            let perms = map_kind(&x.kind());
            let name = x.name().unwrap_or("<unknown>");
            let start = base_address;
            let end = base_address + data.len() as u64;
            event!(
                Level::TRACE,
                "Selecting section {} ({:x}-{:x})",
                name,
                start,
                end
            );
            img.sections.push(ImageSection {
                perms,
                data,
                base_address: base_address as usize,
            })
        }
        if img.sections.is_empty() {
            event!(Level::WARN, "No executable sections loaded from file")
        }
        Ok(img)
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
