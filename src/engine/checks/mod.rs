mod color;
mod dsc;
mod frl;
mod timing;

pub(super) use color::{BitDepthCheck, ColorEncodingCheck};
pub(super) use dsc::DscCheck;
pub(super) use frl::FrlCeilingCheck;
pub(super) use timing::{RefreshRateCheck, TmdsClockCheck};

use crate::engine::rule::ConstraintRule;
use crate::output::warning::Violation;

/// The ordered list of constraint rules applied by `DefaultConstraintEngine`.
///
/// Rules are evaluated in declaration order. In alloc mode all violations are
/// collected; in no-alloc mode the engine short-circuits on the first failure.
pub(super) static DEFAULT_CHECKS: &[&(dyn ConstraintRule<Violation> + Sync)] = &[
    &FrlCeilingCheck,
    &RefreshRateCheck,
    &TmdsClockCheck,
    &ColorEncodingCheck,
    &BitDepthCheck,
    &DscCheck,
];
