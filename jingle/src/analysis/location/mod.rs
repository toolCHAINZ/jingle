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

pub use basic::BasicLocationAnalysis;
pub use basic::state::BasicLocationState;

/// Re-export the call behavior enum so users can configure how direct calls are handled.
pub use basic::state::CallBehavior;

pub use bound::BoundedBranchAnalysis;
pub use bound::state::BoundedBranchState;

pub use unwind::UnwindingAnalysis;
pub use unwind::state::UnwindingState;
