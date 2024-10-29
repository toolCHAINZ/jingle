use jingle_sleigh::{JingleSleighError, PcodeOperation};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum JingleError {
    #[error("Error talking to Sleigh")]
    Sleigh(#[from] JingleSleighError),
    #[error("Given address disassembles cleanly but does not terminate within the given bound")]
    DisassemblyLengthBound,
    #[error("Attempted to construct an illegally-sized branch target")]
    InvalidBranchTargetSize,
    #[error("A z3 array selection operation returned something other than a bitvector")]
    UnexpectedArraySort,
    #[error("Something referenced a space that isn't declared")]
    UnmodeledSpace,
    #[error("Tried to create a block containing zero instructions")]
    EmptyBlock,
    #[error("Something tried to access a 0-sized varnode")]
    ZeroSizedVarnode,
    #[error("Cannot write values into constant space.")]
    ConstantWrite,
    #[error("Attempt to read an indirect value from the constant space. While this can be modeled, it's almost definitely unintended.")]
    IndirectConstantRead,
    #[error("Attempted to perform a write of a bitvector to a VarNode with leftover space. This is a sleigh bug.")]
    MismatchedWordSize,
    #[error("Attempted to perform a write to a space using the wrong size of address. This is a sleigh bug.")]
    MismatchedAddressSize,
    #[error("Jingle does not yet model this instruction")]
    UnmodeledInstruction(Box<PcodeOperation>),
}
