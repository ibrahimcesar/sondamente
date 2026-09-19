//! Content hashing for preregistration.
//!
//! The hash covers what a spec *says*, not how its YAML is written:
//! comments, whitespace, key order and quoting do not change it, while any
//! change to a claim, hypothesis, prediction, condition, metric, generator
//! or sample size does. The `preregistration` block itself is excluded, so a
//! spec can carry its own hash.

use std::fmt::Write;

use sha2::{Digest, Sha256};

use crate::spec::ProbeSpec;

pub const HASH_PREFIX: &str = "sha256:";

/// Returns `sha256:<64 hex digits>` for the spec's canonical content.
pub fn spec_hash(spec: &ProbeSpec) -> String {
    let mut content = spec.clone();
    content.preregistration = None;

    // Serializing through `serde_json::Value` fixes the key order (object keys
    // are sorted) and the number formatting, independent of the source YAML.
    let canonical = serde_json::to_value(&content)
        .and_then(|value| serde_json::to_string(&value))
        .expect("a parsed spec always serializes to JSON");

    let digest = Sha256::digest(canonical.as_bytes());
    let mut out = String::with_capacity(HASH_PREFIX.len() + 64);
    out.push_str(HASH_PREFIX);
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// True when `s` has the shape of a value produced by [`spec_hash`].
pub fn is_well_formed(s: &str) -> bool {
    s.strip_prefix(HASH_PREFIX)
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
}
