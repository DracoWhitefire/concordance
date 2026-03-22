//! Fatal error type for concordance API entry points.

/// Fatal errors returned from fallible API entry points.
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The capability inputs were internally inconsistent in a way that prevents negotiation.
    #[error("capability inputs were internally inconsistent")]
    InvalidCapabilities,
}
