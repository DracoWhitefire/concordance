mod color;
mod dsc;
mod frl;
mod timing;

pub use color::{BitDepthCheck, ColorEncodingCheck};
pub use dsc::DscCheck;
pub use frl::FrlCeilingCheck;
pub use timing::{RefreshRateCheck, TmdsClockCheck};

use crate::engine::rule::ConstraintRule;
use crate::output::warning::Violation;

/// The ordered list of constraint rules applied by [`DefaultConstraintEngine`][super::DefaultConstraintEngine] by default.
///
/// Rules are evaluated in declaration order. In alloc mode all violations are
/// collected; in no-alloc mode the engine short-circuits on the first failure.
pub static DEFAULT_CHECKS: &[&(dyn ConstraintRule<Violation> + Sync)] = &[
    &FrlCeilingCheck,
    &RefreshRateCheck,
    &TmdsClockCheck,
    &ColorEncodingCheck,
    &BitDepthCheck,
    &DscCheck,
];
