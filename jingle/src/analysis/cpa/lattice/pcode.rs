use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::state::{AbstractState, LocationState, MergeOutcome, Successor};
use crate::display::JingleDisplayable;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{IndirectVarNode, PcodeOperation};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter, LowerHex};
use std::hash::Hash;
use std::iter::{empty, once};

/// The p-code address lattice used by analyses that track program location.
///
/// Variants:
/// - `Const(addr)` — a concrete pcode address (like `FlatLattice::Value`)
/// - `Computed(indirect)` — a location that may be computed from an indirect varnode
/// - `Top` — unknown / multiple locations
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum PcodeAddressLattice {
    Const(ConcretePcodeAddress),
    Computed(IndirectVarNode),
    Top,
}

impl JingleDisplayable for PcodeAddressLattice {
    fn fmt_jingle(
        &self,
        f: &mut Formatter<'_>,
        info: &jingle_sleigh::SleighArchInfo,
    ) -> std::fmt::Result {
        match self {
            PcodeAddressLattice::Const(addr) => write!(f, "{:x}", addr),
            PcodeAddressLattice::Computed(indirect_var_node) => {
                indirect_var_node.fmt_jingle(f, info)
            }
            PcodeAddressLattice::Top => write!(f, "Top"),
        }
    }
}

impl Debug for PcodeAddressLattice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PcodeAddressLattice::Const(a) => f
                .debug_tuple("PcodeAddressLattice::Const")
                .field(&format_args!("{:x}", a))
                .finish(),
            PcodeAddressLattice::Computed(c) => f
                .debug_tuple("PcodeAddressLattice::Computed")
                .field(&format_args!("{:?}", c))
                .finish(),
            PcodeAddressLattice::Top => write!(f, "PcodeAddressLattice::Top"),
        }
    }
}

impl LowerHex for PcodeAddressLattice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            // Delegate to the inner `ConcretePcodeAddress` LowerHex implementation
            // so `{:#x}` / `{:x}` on `PcodeAddressLattice::Const` prints the expected hex form.
            PcodeAddressLattice::Const(a) => write!(f, "PcodeAddressLattice::Const({:x})", a),
            // Computed values don't have a natural hex representation; fall back to debug.
            PcodeAddressLattice::Computed(c) => {
                write!(f, "PcodeAddressLattice::Computed({:?})", c)
            }
            PcodeAddressLattice::Top => write!(f, "PcodeAddressLattice::Top"),
        }
    }
}

impl From<ConcretePcodeAddress> for PcodeAddressLattice {
    fn from(value: ConcretePcodeAddress) -> Self {
        PcodeAddressLattice::Const(value)
    }
}

impl PcodeAddressLattice {
    pub fn is_top(&self) -> bool {
        matches!(self, PcodeAddressLattice::Top)
    }

    /// Returns the concrete address if this lattice element is `Const`.
    pub fn value(&self) -> Option<&ConcretePcodeAddress> {
        match self {
            PcodeAddressLattice::Const(c) => Some(c),
            _ => None,
        }
    }
}

impl PartialOrd for PcodeAddressLattice {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Match on references to avoid moving out of `self`/`other`
        match (&self, &other) {
            (Self::Top, Self::Top) => Some(Ordering::Equal),
            (Self::Top, Self::Const(_)) => Some(Ordering::Greater),
            (Self::Const(_), Self::Top) => Some(Ordering::Less),
            (Self::Const(a), Self::Const(b)) => {
                if a == b {
                    Some(Ordering::Equal)
                } else {
                    None
                }
            }
            (Self::Computed(x), Self::Computed(y)) => {
                if x == y {
                    Some(Ordering::Equal)
                } else {
                    None
                }
            }
            // Different kinds (Const vs Computed) are incomparable
            _ => None,
        }
    }
}

impl JoinSemiLattice for PcodeAddressLattice {
    fn join(&mut self, other: &Self) {
        // Match on references to avoid moving out of `self` while inspecting it.
        match (&*self, other) {
            (Self::Top, _) => *self = Self::Top,
            (_, Self::Top) => *self = Self::Top,
            (Self::Const(a), Self::Const(b)) => {
                if a == b {
                    // keep the same concrete value
                } else {
                    *self = Self::Top;
                }
            }
            (Self::Computed(x), Self::Computed(y)) => {
                if x == y {
                    // keep the same computed descriptor
                } else {
                    *self = Self::Top;
                }
            }
            // Mixing Const and Computed -> unknown
            _ => {
                *self = Self::Top;
            }
        };
    }
}

impl AbstractState for PcodeAddressLattice {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        // Preserve separate-merge default semantics (no merging by default).
        // Use `merge_sep` so analyses that rely on separate states continue to work.
        self.merge_sep(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, op: B) -> Successor<'a, Self> {
        let op = op.borrow();
        match op {
            PcodeOperation::BranchInd { input }
            | PcodeOperation::CallInd { input }
            | PcodeOperation::Return { input } => {
                return once(PcodeAddressLattice::Computed(input.clone())).into();
            }
            _ => {}
        }

        match self {
            PcodeAddressLattice::Const(a) => a.transfer(op).into_iter().map(Self::Const).into(),
            PcodeAddressLattice::Computed(_) => empty().into(),
            PcodeAddressLattice::Top => empty().into(),
        }
    }
}

impl LocationState for PcodeAddressLattice {
    fn get_operation<'a, T: crate::analysis::pcode_store::PcodeStore + ?Sized>(
        &'a self,
        t: &'a T,
    ) -> Option<crate::analysis::pcode_store::PcodeOpRef<'a>> {
        match self {
            PcodeAddressLattice::Const(a) => t.get_pcode_op_at(a),
            // If the location is computed or top, we cannot directly get a concrete op
            PcodeAddressLattice::Computed(_) => None,
            PcodeAddressLattice::Top => None,
        }
    }

    fn get_location(&self) -> Option<ConcretePcodeAddress> {
        self.value().cloned()
    }
}
