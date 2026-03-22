//! Caller-supplied capability input types.

pub mod cable;
pub mod candidate;
pub mod sink;
pub mod source;

pub use cable::CableCapabilities;
pub use candidate::CandidateConfig;
pub use sink::SinkCapabilities;
pub use source::SourceCapabilities;
