//! # sondamente
//!
//! Preregistered probes for philosophy-of-mind claims about LLMs.
//!
//! Every probe states a claim, at least two rival hypotheses about it, and
//! what each hypothesis predicts under named conditions. The validator
//! refuses probes on which no possible result could favor one hypothesis
//! over another, and the preregistration hash detects any change to a
//! probe after it was locked.
//!
//! ```no_run
//! let spec = sondamente::load("probes/systematicity-novel-rules-001.yaml")?;
//! let report = sondamente::validate(&spec);
//! for diagnostic in &report.diagnostics {
//!     println!("{diagnostic}");
//! }
//! println!("{}", sondamente::spec_hash(&spec));
//! # Ok::<(), sondamente::LoadError>(())
//! ```

pub mod hash;
pub mod spec;
pub mod validate;

pub use hash::spec_hash;
pub use spec::{ProbeSpec, SPEC_VERSION};
pub use validate::{validate, Diagnostic, Report, Severity};

use std::{fmt, fs, io, path::Path};

/// Failure to read or parse a spec file.
#[derive(Debug)]
pub enum LoadError {
    Io(io::Error),
    Parse(serde_yaml::Error),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "cannot read spec: {e}"),
            LoadError::Parse(e) => write!(f, "invalid spec: {e}"),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LoadError::Io(e) => Some(e),
            LoadError::Parse(e) => Some(e),
        }
    }
}

/// Parses a spec from YAML text. Unknown fields are rejected, so a typo
/// cannot silently drop part of a probe.
pub fn from_yaml_str(text: &str) -> Result<ProbeSpec, serde_yaml::Error> {
    serde_yaml::from_str(text)
}

/// Reads and parses a spec file.
pub fn load(path: impl AsRef<Path>) -> Result<ProbeSpec, LoadError> {
    let text = fs::read_to_string(path).map_err(LoadError::Io)?;
    from_yaml_str(&text).map_err(LoadError::Parse)
}
