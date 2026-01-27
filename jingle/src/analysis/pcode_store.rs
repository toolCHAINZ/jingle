use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use jingle_sleigh::context::loaded::LoadedSleighContext;
use std::borrow::{Borrow, Cow};

/// PcodeOpRef — a small, ergonomic wrapper for p-code operations
///
/// `PcodeOpRef` encapsulates either a borrowed reference to a `PcodeOperation`
/// or an owned `PcodeOperation` (internally using `Cow`). This hides the `Cow`
/// type from the rest of the codebase and provides simple helper methods so
/// callers don't need to care about ownership when they only need a `&PcodeOperation`.
///
/// Why this exists
/// - Some stores (e.g., an in-memory CFG) can return a reference to an operation
///   stored inside the structure (no clone required).
/// - Other stores (e.g., `LoadedSleighContext::instruction_at`) construct an
///   `Instruction` on each call and therefore must return an owned `PcodeOperation`.
/// - `PcodeOpRef` lets the store return either without exposing `Cow` to callers.
///
/// Basic usage
/// ```ignore
/// // Get an op from a pcode store (may be borrowed or owned internally)
/// if let Some(op_ref) = store.get_pcode_op_at(addr) {
///     // Use the borrowed reference for transfer/inspection:
///     let op: &PcodeOperation = op_ref.as_ref();
///     // When you need an owned op, clone the reference:
///     let owned_op: PcodeOperation = op_ref.as_ref().clone();
/// }
/// ```
///
/// Note: there is no
/// `into_owned` method on `PcodeOpRef` in order to keep the
/// abstraction minimal; callers that need an owned value can call `.as_ref().clone()`.
pub struct PcodeOpRef<'a>(std::borrow::Cow<'a, PcodeOperation>);

impl<'a> AsRef<PcodeOperation> for PcodeOpRef<'a> {
    fn as_ref(&self) -> &PcodeOperation {
        self.0.as_ref()
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

/// A store of p-code operations.
///
/// Implementations return `Option<PcodeOpRef<'a>>`. Callers that only need to
/// observe/borrow the operation should use `op_ref.as_ref()` to get a `&PcodeOperation`.
/// If an owned operation is required, clone via `.as_ref().clone()`.
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
        // `instruction_at` produces an owned `Instruction` per call. Its `ops`
        // are owned inside that `Instruction`, so we cannot return references
        // pointing into a temporary. Convert to an owned `PcodeOperation`.
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
