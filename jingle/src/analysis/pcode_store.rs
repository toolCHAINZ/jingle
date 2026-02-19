use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::context::image::SleighImage;
use jingle_sleigh::context::loaded::LoadedSleighContext;
use std::borrow::Borrow;

pub use jingle_sleigh::PcodeOpRef;

/// A store of p-code operations.
///
/// Implementations return `Option<PcodeOpRef<'op>>`. Callers that only need to
/// observe/borrow the operation should use `op_ref.as_ref()` to get a `&PcodeOperation`.
/// If an owned operation is required, clone via `.as_ref().clone()`.
///
/// The trait is lifetime-parameterized so stores can tie the returned operation
/// reference lifetime to the borrow of the store, avoiding unnecessary cloning.
pub trait PcodeStore<'op> {
    fn get_pcode_op_at<T: Borrow<ConcretePcodeAddress>>(
        &'op self,
        addr: T,
    ) -> Option<PcodeOpRef<'op>>;
}

pub trait EntryPoint {
    fn get_entry(&self) -> ConcretePcodeAddress;
}

impl<'a, I: SleighImage + 'a> PcodeStore<'a> for LoadedSleighContext<'a, I> {
    fn get_pcode_op_at<T: Borrow<ConcretePcodeAddress>>(
        &'a self,
        addr: T,
    ) -> Option<PcodeOpRef<'a>> {
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

impl<'op, T: PcodeStore<'op> + ?Sized> PcodeStore<'op> for &T {
    fn get_pcode_op_at<B: Borrow<ConcretePcodeAddress>>(
        &'op self,
        addr: B,
    ) -> Option<PcodeOpRef<'op>> {
        (*self).get_pcode_op_at(addr)
    }
}
