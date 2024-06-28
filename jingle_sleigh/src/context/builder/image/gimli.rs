use crate::context::builder::image::Perms;
use crate::context::{Image, ImageSection};
use crate::JingleSleighError;
use crate::JingleSleighError::ImageLoadError;
use object::elf::{PF_R, PF_W, PF_X, SHF_EXECINSTR, SHF_WRITE};
use object::macho::{VM_PROT_EXECUTE, VM_PROT_READ, VM_PROT_WRITE};
use object::{
    Architecture, Endianness, File, Object, ObjectSection, ObjectSegment, SectionFlags,
    SegmentFlags,
};

impl<'d> TryFrom<File<'d>> for Image {
    type Error = JingleSleighError;
    fn try_from(value: File) -> Result<Self, Self::Error> {
        let mut img: Image = Image { sections: vec![] };
        for x in value.sections() {
            let base_address = x.address();
            let data = x.data().map_err(|_| ImageLoadError)?.to_vec();
            let perms = map_flags(&x.flags());
            if perms.exec {
                img.sections.push(ImageSection {
                    perms,
                    data,
                    base_address: base_address as usize,
                })
            }
        }
        Ok(img)
    }
}

fn map_flags(flags: &SectionFlags) -> Perms {
    match flags {
        SectionFlags::Elf { sh_flags } => Perms {
            exec: (*sh_flags as u32 & SHF_EXECINSTR) == SHF_EXECINSTR,
            write: (*sh_flags as u32 & SHF_WRITE) == SHF_WRITE,
            read: true,
        },
        SectionFlags::MachO { flags, .. } => Perms {
            exec: (flags & VM_PROT_EXECUTE) == VM_PROT_EXECUTE,
            write: (flags & VM_PROT_WRITE) == VM_PROT_WRITE,
            read: (flags & VM_PROT_READ) == VM_PROT_READ,
        },
        _ => Perms {
            read: false,
            write: false,
            exec: false,
        },
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

        Architecture::Xtensa => match file.endianness() {
            Endianness::Little => Some("Xtensa:LE:32:default"),
            Endianness::Big => Some("Xtensa:BE:32:default"),
        },
        _ => None,
    }
}
