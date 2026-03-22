//! Configuration ranker trait and default implementation.

pub mod policy;

use alloc::vec::Vec;

use crate::diagnostic::Diagnostic;
use crate::output::config::NegotiatedConfig;
use crate::ranker::policy::NegotiationPolicy;

pub use policy::NegotiationPolicy as Policy;

/// Orders validated configurations according to a [`NegotiationPolicy`].
///
/// The default policy encodes a sensible preference (native resolution, max color
/// fidelity, then refresh rate, then fallback formats), but the caller can supply
/// an override via [`NegotiatorBuilder`][crate::NegotiatorBuilder].
pub trait ConfigRanker {
    /// Non-fatal diagnostic type attached to ranked configurations.
    type Warning: Diagnostic;

    /// Ranks and returns the validated configurations in priority order.
    fn rank(
        &self,
        configs: Vec<NegotiatedConfig<Self::Warning>>,
        policy: &NegotiationPolicy,
    ) -> Vec<NegotiatedConfig<Self::Warning>>;
}

/// Default configuration ranker.
///
/// Implements the built-in preference ordering: native resolution, maximum color
/// fidelity, highest refresh rate, then fallback formats. DSC configurations are
/// ranked lower by default.
#[derive(Debug, Default)]
pub struct DefaultRanker;

impl ConfigRanker for DefaultRanker {
    type Warning = crate::output::warning::Warning;

    fn rank(
        &self,
        configs: Vec<NegotiatedConfig<Self::Warning>>,
        _policy: &NegotiationPolicy,
    ) -> Vec<NegotiatedConfig<Self::Warning>> {
        // TODO: implement ranking according to policy
        configs
    }
}
