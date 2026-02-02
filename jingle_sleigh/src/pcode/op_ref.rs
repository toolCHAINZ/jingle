use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter, LowerHex};

use super::PcodeOperation;

/// `PcodeOpRef` — ergonomic wrapper for p-code operations stored or borrowed.
///
/// `PcodeOpRef` encapsulates either a borrowed reference to a `PcodeOperation` or an
/// owned `PcodeOperation` (internally using `Cow`). This type is intended to be
/// the canonical p-code operation reference type used by the sleigh crate.
#[derive(Clone)]
pub struct PcodeOpRef<'a>(Cow<'a, PcodeOperation>);

impl<'a> AsRef<PcodeOperation> for PcodeOpRef<'a> {
    fn as_ref(&self) -> &PcodeOperation {
        self.0.as_ref()
    }
}

impl<'a> std::ops::Deref for PcodeOpRef<'a> {
    type Target = PcodeOperation;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl<'a> From<PcodeOperation> for PcodeOpRef<'a> {
    fn from(op: PcodeOperation) -> Self {
        PcodeOpRef(Cow::Owned(op))
    }
}

impl<'a> From<&'a PcodeOperation> for PcodeOpRef<'a> {
    fn from(op: &'a PcodeOperation) -> Self {
        PcodeOpRef(Cow::Borrowed(op))
    }
}

impl<'a> Debug for PcodeOpRef<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Delegate to the inner `PcodeOperation` Debug implementation.
        std::fmt::Debug::fmt(self.as_ref(), f)
    }
}

impl<'a> Display for PcodeOpRef<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Delegate to the inner `PcodeOperation` Display implementation.
        Display::fmt(self.as_ref(), f)
    }
}

impl<'a> LowerHex for PcodeOpRef<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Delegate to the inner `PcodeOperation` LowerHex implementation.
        LowerHex::fmt(self.as_ref(), f)
    }
}

/// Implement the JingleDisplay trait for `PcodeOpRef` by delegating to the
/// inner `PcodeOperation`'s `fmt_jingle` implementation.
impl<'a> crate::display::JingleDisplay for PcodeOpRef<'a> {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &crate::SleighArchInfo) -> std::fmt::Result {
        self.as_ref().fmt_jingle(f, info)
    }
}
