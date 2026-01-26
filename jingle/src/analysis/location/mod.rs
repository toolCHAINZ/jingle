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
