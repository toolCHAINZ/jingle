//! Public-facing location analyses.
//!
//! This module keeps the internal component analyses private but exposes a small set of
//! compound analyses built from them:
//!
//! - `LocationAnalysis` — the plain location analysis (direct location).
//! - `BoundedLocationAnalysis` — location + bounded-branch counting.
//! - `UnwoundLocationAnalysis` — location + back-edge counting (unwinding).
//! - `UnwoundBoundedLocationAnalysis` — location + bounded-branch + back-edge counting.
//!
//! The tuple-based compound analyses rely on the generic tuple `ConfigurableProgramAnalysis`
//! impls in `analysis::compound`, so we only export convenient type aliases here.
//!
//! We also re-export the `CallBehavior` enum so callers can configure call handling.

mod basic;
mod bound;
mod unwind;

/// Re-export the call behavior enum so users can configure how direct calls are handled.
pub use basic::state::CallBehavior;
pub use basic::state::DirectLocationState;

/// A plain location analysis.
pub type LocationAnalysis = basic::DirectLocationAnalysis;

/// Location analysis combined with a bounded-branch counter.
/// This is equivalent to `(DirectLocationAnalysis, BoundedBranchAnalysis)`.
pub type BoundedLocationAnalysis = (basic::DirectLocationAnalysis, bound::BoundedBranchAnalysis);

/// Location analysis combined with back-edge counting (unwinding).
/// This is equivalent to `(DirectLocationAnalysis, BackEdgeCountCPA)`.
pub type UnwoundLocationAnalysis = (basic::DirectLocationAnalysis, unwind::BackEdgeCountCPA);

/// Location + bounded-branch + back-edge counting (all three components).
/// This is equivalent to `(DirectLocationAnalysis, BoundedBranchAnalysis, BackEdgeCountCPA)`.
pub type UnwoundBoundedLocationAnalysis = (
    basic::DirectLocationAnalysis,
    bound::BoundedBranchAnalysis,
    unwind::BackEdgeCountCPA,
);

/// Convenience constructors for the analyses.
///
/// These helpers create the corresponding analysis or tuple-of-analyses with sensible
/// parameters supplied by callers. They make it more ergonomic to instantiate the
/// common compound analyses without having to reference the internal modules.
impl LocationAnalysis {
    /// Construct a new plain `LocationAnalysis` with the specified call behavior.
    pub fn with_call_behavior(call_behavior: CallBehavior) -> Self {
        basic::DirectLocationAnalysis::new(call_behavior)
    }
}

pub fn location(call_behavior: CallBehavior) -> LocationAnalysis {
    LocationAnalysis::new(call_behavior)
}

/// Construct a `BoundedLocationAnalysis` (location + bounded branch counter).
pub fn bounded_location(
    call_behavior: CallBehavior,
    max_branches: usize,
) -> BoundedLocationAnalysis {
    (
        basic::DirectLocationAnalysis::new(call_behavior),
        bound::BoundedBranchAnalysis::new(max_branches),
    )
}

/// Construct an `UnwoundLocationAnalysis` (location + back-edge/unwind counter).
pub fn unwound_location(call_behavior: CallBehavior, max_unwind: usize) -> UnwoundLocationAnalysis {
    (
        basic::DirectLocationAnalysis::new(call_behavior),
        unwind::BackEdgeCountCPA::new(max_unwind),
    )
}

/// Construct an `UnwoundBoundedLocationAnalysis` (location + bounded branch + back-edge).
pub fn unwound_bounded_location(
    call_behavior: CallBehavior,
    max_branches: usize,
    max_unwind: usize,
) -> UnwoundBoundedLocationAnalysis {
    (
        basic::DirectLocationAnalysis::new(call_behavior),
        bound::BoundedBranchAnalysis::new(max_branches),
        unwind::BackEdgeCountCPA::new(max_unwind),
    )
}
