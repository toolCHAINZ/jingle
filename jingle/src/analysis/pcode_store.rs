use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::context::loaded::LoadedSleighContext;
use jingle_sleigh::{PcodeOperation, VarNode};
use std::borrow::{Borrow, Cow};

/// A lightweight wrapper that encapsulates either a borrowed or owned `PcodeOperation`.
/// This hides the use of `Cow` from the rest of the codebase while still allowing
/// stores to return borrowed references when possible and owned values when necessary.
pub struct PcodeOpRef<'a>(std::borrow::Cow<'a, PcodeOperation>);

impl<'a> PcodeOpRef<'a> {
    /// Get a shared reference to the underlying `PcodeOperation`.
    pub fn as_ref(&self) -> &PcodeOperation {
        &self.0
    }
}

impl<'a> std::ops::Deref for PcodeOpRef<'a> {
    type Target = PcodeOperation;
    fn deref(&self) -> &Self::Target {
        &self.0
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

/// A store of p-code operations. Implementations may return either a borrowed or an owned
/// `PcodeOperation` via `PcodeOpRef`. The `PcodeOpRef` type hides `Cow` so callers can
/// work with a clean abstraction (they can call `.as_ref()` or use `Deref`).
pub trait PcodeStore {
    fn get_pcode_op_at<'a, T: Borrow<ConcretePcodeAddress>>(
        &'a self,
        addr: T,
    ) -> Option<PcodeOpRef<'a>>;
}

pub trait EntryPoint {
    fn get_entry(&self) -> ConcretePcodeAddress;
}

impl<'a> PcodeStore for LoadedSleighContext<'a> {
    fn get_pcode_op_at<'b, T: Borrow<ConcretePcodeAddress>>(
        &'b self,
        addr: T,
    ) -> Option<PcodeOpRef<'b>> {
        let addr = addr.borrow();
        // `instruction_at` produces an owned `Instruction`. Its `ops` are owned
        // inside that `Instruction`, so we cannot return references pointing into
        // a temporary. Convert to an owned `PcodeOperation` when necessary.
        let instr = self.instruction_at(addr.machine())?;
        instr
            .ops
            .get(addr.pcode() as usize)
            .cloned()
            .map(PcodeOpRef::from)
    }
}

impl<T: PcodeStore + ?Sized> PcodeStore for &T {
    fn get_pcode_op_at<'a, B: Borrow<ConcretePcodeAddress>>(
        &'a self,
        addr: B,
    ) -> Option<PcodeOpRef<'a>> {
        (*self).get_pcode_op_at(addr)
    }
}
