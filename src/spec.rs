//! The probe specification format, `spec_version: "0.1"`.
//!
//! A probe ties one philosophical claim to at least two rival hypotheses.
//! Each hypothesis commits, before any data exists, to the outcome it
//! predicts for a metric under a named condition. A probe is worth running
//! only if some possible result would favor one hypothesis over another;
//! [`crate::validate`] checks exactly that.

use serde::{Deserialize, Serialize};

/// The only spec version this release understands.
pub const SPEC_VERSION: &str = "0.1";

/// A complete probe specification, usually loaded from a YAML file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeSpec {
    pub spec_version: String,
    /// Stable identifier. A changed probe gets a new id, never an edit.
    pub id: String,
    pub title: String,
    pub claim: Claim,
    pub tier: Tier,
    pub hypotheses: Vec<Hypothesis>,
    pub conditions: Vec<Condition>,
    pub metrics: Vec<Metric>,
    pub generator: Generator,
    pub sampling: Sampling,
    /// Languages the generator produces instances in, e.g. `en`, `pt-BR`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
    /// Present once the probe is locked. Excluded from its own hash.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preregistration: Option<Preregistration>,
}

/// The philosophical claim under test, with where it comes from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub statement: String,
    #[serde(default)]
    pub sources: Vec<String>,
}

/// Behavioral probes need only model outputs; mechanistic probes need
/// access to internal activations, so they run on open-weight models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Behavioral,
    Mechanistic,
}

/// One position on the claim, stated as testable predictions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hypothesis {
    pub id: String,
    pub description: String,
    pub predictions: Vec<Prediction>,
}

/// What a hypothesis expects a metric to be under a condition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prediction {
    pub condition: String,
    pub metric: String,
    pub expect: Expectation,
}

/// An expected region for a metric's value.
///
/// In YAML: `{ below: -0.1 }`, `{ above: 0.6 }`, or `{ within: [-0.05, 0.05] }`.
/// `below` and `above` are strict; `within` includes its endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ExpectationRepr", into = "ExpectationRepr")]
pub enum Expectation {
    Below(f64),
    Above(f64),
    Within([f64; 2]),
}

/// On-disk form of [`Expectation`]: a map with exactly one key. Spelled out
/// because serde_yaml would otherwise encode the enum as a YAML tag.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectationRepr {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    below: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    above: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    within: Option<[f64; 2]>,
}

impl TryFrom<ExpectationRepr> for Expectation {
    type Error = String;

    fn try_from(repr: ExpectationRepr) -> Result<Self, Self::Error> {
        match (repr.below, repr.above, repr.within) {
            (Some(x), None, None) => Ok(Expectation::Below(x)),
            (None, Some(x), None) => Ok(Expectation::Above(x)),
            (None, None, Some(range)) => Ok(Expectation::Within(range)),
            _ => Err("`expect` must have exactly one of `below`, `above`, `within`".into()),
        }
    }
}

impl From<Expectation> for ExpectationRepr {
    fn from(e: Expectation) -> Self {
        let mut repr = ExpectationRepr {
            below: None,
            above: None,
            within: None,
        };
        match e {
            Expectation::Below(x) => repr.below = Some(x),
            Expectation::Above(x) => repr.above = Some(x),
            Expectation::Within(range) => repr.within = Some(range),
        }
        repr
    }
}

impl Expectation {
    /// The set of metric values this expectation accepts.
    pub fn interval(&self) -> Interval {
        match *self {
            Expectation::Below(x) => Interval {
                lo: f64::NEG_INFINITY,
                lo_open: true,
                hi: x,
                hi_open: true,
            },
            Expectation::Above(x) => Interval {
                lo: x,
                lo_open: true,
                hi: f64::INFINITY,
                hi_open: true,
            },
            Expectation::Within([lo, hi]) => Interval {
                lo,
                lo_open: false,
                hi,
                hi_open: false,
            },
        }
    }

    /// The numbers written in the spec, for well-formedness checks.
    pub fn bounds(&self) -> Vec<f64> {
        match *self {
            Expectation::Below(x) | Expectation::Above(x) => vec![x],
            Expectation::Within([lo, hi]) => vec![lo, hi],
        }
    }
}

/// An interval on the real line with independently open or closed ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub lo: f64,
    pub lo_open: bool,
    pub hi: f64,
    pub hi_open: bool,
}

impl Interval {
    /// True when no single value satisfies both intervals, meaning any
    /// observed result is inconsistent with at least one of them.
    pub fn is_disjoint_from(&self, other: &Interval) -> bool {
        self.lies_left_of(other) || other.lies_left_of(self)
    }

    fn lies_left_of(&self, other: &Interval) -> bool {
        self.hi < other.lo || (self.hi == other.lo && (self.hi_open || other.lo_open))
    }
}

/// A named variation of the task, such as renamed symbols or a translation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub id: String,
    pub description: String,
    /// Exactly one condition is the unperturbed reference.
    #[serde(default)]
    pub baseline: bool,
}

/// A quantity computed from model outputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metric {
    pub id: String,
    pub description: String,
}

/// Where fresh task instances come from. Generated rather than fixed
/// instances keep a probe resistant to training-data contamination.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Generator {
    /// Path relative to the repository root.
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sampling {
    pub n_per_condition: u32,
}

/// The lock on a probe: when it was registered and what it said then.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preregistration {
    /// ISO date, `YYYY-MM-DD`.
    pub registered: String,
    /// Output of [`crate::spec_hash`] at registration time.
    pub spec_hash: String,
}
