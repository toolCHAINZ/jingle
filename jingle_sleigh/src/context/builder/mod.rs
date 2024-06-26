use crate::context::builder::image::Image;
use crate::context::builder::language_def::{parse_ldef, LanguageDefinition};
use crate::context::builder::processor_spec::parse_pspec;
use crate::context::SleighContext;
use crate::error::JingleSleighError;
use crate::error::JingleSleighError::{InvalidLanguageId, LanguageSpecRead, NoImageProvided};
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{event, instrument, Level};

pub mod image;
pub(crate) mod language_def;
pub(crate) mod processor_spec;

#[derive(Debug, Default, Clone)]
pub struct SleighContextBuilder {
    defs: Vec<(LanguageDefinition, PathBuf)>,
    image: Option<Image>,
}

impl SleighContextBuilder {
    pub fn get_language_ids(&self) -> Vec<&str> {
        self.defs.iter().map(|(l, _)| l.id.as_str()).collect()
    }

    fn get_language(&self, id: &str) -> Option<&(LanguageDefinition, PathBuf)> {
        self.defs.iter().find(|(p, _)| p.id.eq(id))
    }
    #[instrument(skip_all, fields(%id))]
    pub fn build(mut self, id: &str) -> Result<SleighContext, JingleSleighError> {
        let image = self.image.take().ok_or(NoImageProvided)?;
        let (lang, path) = self.get_language(id).ok_or(InvalidLanguageId)?;
        let sla_path = path.join(&lang.sla_file);
        let mut context = SleighContext::new(&sla_path, image)?;
        event!(Level::INFO, "Created sleigh context");
        let pspec_path = path.join(&lang.processor_spec);
        let pspec = parse_pspec(&pspec_path)?;
        for set in pspec.context_data.context_set.sets {
            context.set_initial_context(&set.name, set.value as u32)
        }

        Ok(context)
    }
    pub fn load_folder<T: AsRef<Path>>(path: T) -> Result<Self, JingleSleighError> {
        let ldef = SleighContextBuilder::_load_folder(path.as_ref())?;
        Ok(SleighContextBuilder {
            defs: ldef,
            image: None,
        })
    }

    fn _load_folder(path: &Path) -> Result<Vec<(LanguageDefinition, PathBuf)>, JingleSleighError> {
        let path = path.canonicalize();
        let path = path.map_err(|_| LanguageSpecRead)?;
        if !path.is_dir() {
            return Err(LanguageSpecRead);
        }
        let ldef_path = find_ldef(&path)?;
        let defs = parse_ldef(ldef_path.as_path())?;
        let defs = defs
            .iter()
            .map(|f| (f.clone(), path.to_path_buf()))
            .collect();
        Ok(defs)
    }

    #[instrument]
    pub fn load_ghidra_installation<T: AsRef<Path> + Debug>(
        path: T,
    ) -> Result<Self, JingleSleighError> {
        let path = path.as_ref().join("Ghidra").join("Processors");
        let mut defs = vec![];
        for entry in (path.read_dir().map_err(|_| LanguageSpecRead)?).flatten() {
            let lang_path = entry.path().join("data").join("languages");
            if lang_path.exists() {
                let d = Self::_load_folder(&lang_path)?;
                defs.extend(d);
            }
        }
        Ok(SleighContextBuilder { defs, image: None })
    }

    pub fn set_image(mut self, img: Image) -> Self {
        self.image = Some(img);
        self
    }
}

fn find_ldef(path: &Path) -> Result<PathBuf, JingleSleighError> {
    for entry in (fs::read_dir(path).map_err(|_| LanguageSpecRead)?).flatten() {
        if let Some(e) = entry.path().extension() {
            if e == "ldefs" {
                return Ok(entry.path().clone());
            }
        }
    }
    Err(LanguageSpecRead)
}

#[cfg(test)]
mod tests {
    use crate::context::builder::processor_spec::parse_pspec;
    use crate::context::builder::{parse_ldef, SleighContextBuilder};

    use crate::tests::SLEIGH_ARCH;
    use std::path::Path;

    #[test]
    fn test_parse_ldef() {
        parse_ldef(Path::new(
            "ghidra/Ghidra/Processors/x86/data/languages/x86.ldefs",
        ))
        .unwrap();
    }

    #[test]
    fn test_parse_pspec() {
        parse_pspec(Path::new(
            "ghidra/Ghidra/Processors/x86/data/languages/x86.pspec",
        ))
        .unwrap();
    }

    #[test]
    fn test_parse_language_folder() {
        SleighContextBuilder::load_folder(Path::new(
            "ghidra/Ghidra/Processors/x86/data/languages/",
        ))
        .unwrap();
        SleighContextBuilder::load_folder(Path::new("ghidra/Ghidra/Processors/x86/data/languages"))
            .unwrap();
    }

    #[test]
    fn test_parse_language_ghidra() {
        let _builder = SleighContextBuilder::load_ghidra_installation(Path::new("ghidra")).unwrap();
    }

    #[test]
    fn test_get_language() {
        let langs = SleighContextBuilder::load_folder(Path::new(
            "ghidra/Ghidra/Processors/x86/data/languages/",
        ))
        .unwrap();
        assert!(langs.get_language("sdf").is_none());
        assert!(langs.get_language(SLEIGH_ARCH).is_some());
    }
}
