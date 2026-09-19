//! Spec validation.
//!
//! Besides ordinary structural checks, the validator enforces the rule the
//! project exists for: a probe must contain at least one *crucial condition*,
//! a condition and metric on which two rival hypotheses predict outcomes that
//! cannot both occur. Without one, no possible result favors any hypothesis
//! over another, and running the probe cannot move the debate.

use std::collections::{BTreeMap, HashSet};
use std::fmt;

use crate::hash::{self, spec_hash};
use crate::spec::{Interval, Prediction, ProbeSpec, SPEC_VERSION};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Warning,
    Error,
}

/// One finding. `code` is stable and documented in the README.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{level}[{}]: {}", self.code, self.message)
    }
}

/// All findings for one spec, errors first.
#[derive(Debug, Clone, Default)]
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    /// True when the spec has no errors. Warnings do not fail a spec.
    pub fn is_ok(&self) -> bool {
        self.errors().next().is_none()
    }

    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
    }

    fn error(&mut self, code: &'static str, message: String) {
        self.push(Severity::Error, code, message);
    }

    fn warning(&mut self, code: &'static str, message: String) {
        self.push(Severity::Warning, code, message);
    }

    fn push(&mut self, severity: Severity, code: &'static str, message: String) {
        self.diagnostics.push(Diagnostic {
            severity,
            code,
            message,
        });
    }
}

/// Checks a parsed spec. Parsing already rejected unknown fields and
/// missing required ones; this checks everything the types cannot express.
pub fn validate(spec: &ProbeSpec) -> Report {
    let mut report = Report::default();
    check_header(spec, &mut report);
    check_identifiers(spec, &mut report);
    check_baseline(spec, &mut report);
    check_predictions(spec, &mut report);
    check_discrimination(spec, &mut report);
    check_coverage(spec, &mut report);
    check_execution(spec, &mut report);
    check_preregistration(spec, &mut report);
    // Stable sort: errors first, otherwise in the order they were found.
    report
        .diagnostics
        .sort_by(|a, b| b.severity.cmp(&a.severity));
    report
}

fn check_header(spec: &ProbeSpec, report: &mut Report) {
    if spec.spec_version != SPEC_VERSION {
        report.error(
            "unsupported-version",
            format!(
                "spec_version is \"{}\", but this release understands \"{SPEC_VERSION}\"",
                spec.spec_version
            ),
        );
    }
    if spec.title.trim().is_empty() {
        report.error("empty-title", "title is empty".into());
    }
    if spec.claim.statement.trim().is_empty() {
        report.error("empty-claim", "claim.statement is empty".into());
    }
    if spec.claim.sources.is_empty() {
        report.warning(
            "claim-without-source",
            "claim.sources is empty; cite where the claim is argued so readers can check \
             that the probe tests the claim as its defenders state it"
                .into(),
        );
    }
}

fn check_identifiers(spec: &ProbeSpec, report: &mut Report) {
    if !is_valid_id(&spec.id) {
        report.error("invalid-id", invalid_id_message("probe", &spec.id));
    }
    check_namespace(
        "hypothesis",
        spec.hypotheses.iter().map(|h| h.id.as_str()),
        report,
    );
    check_namespace(
        "condition",
        spec.conditions.iter().map(|c| c.id.as_str()),
        report,
    );
    check_namespace("metric", spec.metrics.iter().map(|m| m.id.as_str()), report);
}

fn check_namespace<'a>(kind: &str, ids: impl Iterator<Item = &'a str>, report: &mut Report) {
    let mut seen = HashSet::new();
    for id in ids {
        if !is_valid_id(id) {
            report.error("invalid-id", invalid_id_message(kind, id));
        }
        if !seen.insert(id) {
            report.error(
                "duplicate-id",
                format!("{kind} id `{id}` is declared more than once"),
            );
        }
    }
}

fn check_baseline(spec: &ProbeSpec, report: &mut Report) {
    let baselines: Vec<&str> = spec
        .conditions
        .iter()
        .filter(|c| c.baseline)
        .map(|c| c.id.as_str())
        .collect();
    match baselines.len() {
        1 => {}
        0 => report.error(
            "no-baseline",
            "no condition is marked `baseline: true`; perturbations need a reference".into(),
        ),
        _ => report.error(
            "multiple-baselines",
            format!(
                "exactly one condition may be the baseline, found: {}",
                baselines.join(", ")
            ),
        ),
    }
}

fn check_predictions(spec: &ProbeSpec, report: &mut Report) {
    let conditions: HashSet<&str> = spec.conditions.iter().map(|c| c.id.as_str()).collect();
    let metrics: HashSet<&str> = spec.metrics.iter().map(|m| m.id.as_str()).collect();

    for h in &spec.hypotheses {
        if h.predictions.is_empty() {
            report.error(
                "hypothesis-without-predictions",
                format!(
                    "hypothesis `{}` predicts nothing, so no result can bear on it",
                    h.id
                ),
            );
        }
        let mut seen = HashSet::new();
        for p in &h.predictions {
            if !conditions.contains(p.condition.as_str()) {
                report.error(
                    "unknown-condition",
                    format!(
                        "hypothesis `{}` predicts under condition `{}`, which is not declared",
                        h.id, p.condition
                    ),
                );
            }
            if !metrics.contains(p.metric.as_str()) {
                report.error(
                    "unknown-metric",
                    format!(
                        "hypothesis `{}` predicts metric `{}`, which is not declared",
                        h.id, p.metric
                    ),
                );
            }
            if p.expect.bounds().iter().any(|x| !x.is_finite()) {
                report.error(
                    "non-finite-bound",
                    format!(
                        "hypothesis `{}` uses a non-finite bound for `{}` under `{}`",
                        h.id, p.metric, p.condition
                    ),
                );
            } else {
                let i = p.expect.interval();
                if i.lo > i.hi {
                    report.error(
                        "empty-interval",
                        format!(
                            "hypothesis `{}` expects `{}` under `{}` within [{}, {}], \
                             which contains no values",
                            h.id, p.metric, p.condition, i.lo, i.hi
                        ),
                    );
                }
            }
            if !seen.insert((p.condition.as_str(), p.metric.as_str())) {
                report.error(
                    "duplicate-prediction",
                    format!(
                        "hypothesis `{}` predicts `{}` under `{}` more than once",
                        h.id, p.metric, p.condition
                    ),
                );
            }
        }
    }
}

/// A hypothesis's well-formed predictions, keyed by (condition, metric).
type PredictionTable<'a> = BTreeMap<(&'a str, &'a str), Interval>;

fn prediction_table(spec: &ProbeSpec) -> Vec<(&str, PredictionTable<'_>)> {
    spec.hypotheses
        .iter()
        .map(|h| {
            let table = h
                .predictions
                .iter()
                .filter(|p| p.expect.bounds().iter().all(|x| x.is_finite()))
                .map(|p| {
                    (
                        (p.condition.as_str(), p.metric.as_str()),
                        p.expect.interval(),
                    )
                })
                .filter(|(_, i)| i.lo <= i.hi)
                .collect();
            (h.id.as_str(), table)
        })
        .collect()
}

fn check_discrimination(spec: &ProbeSpec, report: &mut Report) {
    let n = spec.hypotheses.len();
    if n < 2 {
        report.error(
            "no-rival-hypotheses",
            format!(
                "a probe needs at least two rival hypotheses, found {n}; a claim tested \
                 against no alternative cannot be discriminated by any result"
            ),
        );
        return;
    }

    let table = prediction_table(spec);
    let mut discriminated = 0;
    let mut undiscriminated = Vec::new();
    for (i, (a_id, a)) in table.iter().enumerate() {
        for (b_id, b) in &table[i + 1..] {
            let crucial = a
                .iter()
                .any(|(key, ia)| b.get(key).is_some_and(|ib| ia.is_disjoint_from(ib)));
            if crucial {
                discriminated += 1;
            } else {
                undiscriminated.push((*a_id, *b_id));
            }
        }
    }

    if discriminated == 0 {
        report.error(
            "no-crucial-condition",
            "no condition and metric on which two hypotheses predict non-overlapping \
             outcomes; whatever the result, it favors no hypothesis over another"
                .into(),
        );
        return;
    }
    for (a, b) in undiscriminated {
        report.warning(
            "indistinguishable-hypotheses",
            format!(
                "`{a}` and `{b}` never predict non-overlapping outcomes on a shared \
                 condition and metric, so this probe cannot tell them apart"
            ),
        );
    }
}

fn check_coverage(spec: &ProbeSpec, report: &mut Report) {
    let predicted = |f: &dyn Fn(&Prediction) -> bool| {
        spec.hypotheses.iter().flat_map(|h| &h.predictions).any(f)
    };
    for c in spec.conditions.iter().filter(|c| !c.baseline) {
        if !predicted(&|p| p.condition == c.id) {
            report.warning(
                "unused-condition",
                format!(
                    "condition `{}` is declared but no hypothesis predicts anything under it",
                    c.id
                ),
            );
        }
    }
    for m in &spec.metrics {
        if !predicted(&|p| p.metric == m.id) {
            report.warning(
                "unused-metric",
                format!(
                    "metric `{}` is declared but no hypothesis predicts it",
                    m.id
                ),
            );
        }
    }
}

fn check_execution(spec: &ProbeSpec, report: &mut Report) {
    if spec.sampling.n_per_condition == 0 {
        report.error(
            "empty-sample",
            "sampling.n_per_condition must be greater than zero".into(),
        );
    }
    if spec.generator.path.trim().is_empty() {
        report.error("missing-generator", "generator.path is empty".into());
    }
}

fn check_preregistration(spec: &ProbeSpec, report: &mut Report) {
    let Some(prereg) = &spec.preregistration else {
        return;
    };
    if !is_iso_date(&prereg.registered) {
        report.error(
            "invalid-date",
            format!(
                "preregistration.registered is \"{}\", expected YYYY-MM-DD",
                prereg.registered
            ),
        );
    }
    if !hash::is_well_formed(&prereg.spec_hash) {
        report.error(
            "malformed-hash",
            "preregistration.spec_hash must look like `sha256:<64 hex digits>`; \
             generate it with `sondamente hash <spec>`"
                .into(),
        );
        return;
    }
    let current = spec_hash(spec);
    if !prereg.spec_hash.eq_ignore_ascii_case(&current) {
        report.error(
            "spec-changed-after-preregistration",
            format!(
                "the spec no longer matches the hash recorded on {} (recorded {}, now {}); \
                 revert the change, or register the new version under a new id",
                prereg.registered, prereg.spec_hash, current
            ),
        );
    }
}

/// Lowercase ASCII letters and digits, in groups separated by `-` or `_`.
fn is_valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.split(['-', '_']).all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

fn invalid_id_message(kind: &str, id: &str) -> String {
    format!(
        "{kind} id `{id}` must be lowercase letters and digits, in groups separated by `-` or `_`"
    )
}

fn is_iso_date(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    let [y, m, d] = parts.as_slice() else {
        return false;
    };
    let numeric = |p: &str, len: usize| p.len() == len && p.bytes().all(|b| b.is_ascii_digit());
    if !(numeric(y, 4) && numeric(m, 2) && numeric(d, 2)) {
        return false;
    }
    let (m, d): (u32, u32) = (m.parse().unwrap_or(0), d.parse().unwrap_or(0));
    (1..=12).contains(&m) && (1..=31).contains(&d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Expectation;

    fn disjoint(a: Expectation, b: Expectation) -> bool {
        a.interval().is_disjoint_from(&b.interval())
    }

    #[test]
    fn strict_bounds_meeting_a_closed_bound_are_disjoint() {
        assert!(disjoint(
            Expectation::Below(-0.1),
            Expectation::Within([-0.1, 0.0])
        ));
        assert!(disjoint(
            Expectation::Above(1.0),
            Expectation::Within([0.0, 1.0])
        ));
    }

    #[test]
    fn closed_intervals_sharing_an_endpoint_overlap() {
        assert!(!disjoint(
            Expectation::Within([-0.1, 0.0]),
            Expectation::Within([0.0, 0.1])
        ));
    }

    #[test]
    fn below_and_above_the_same_point_are_disjoint() {
        assert!(disjoint(Expectation::Below(0.5), Expectation::Above(0.5)));
        assert!(!disjoint(Expectation::Below(0.6), Expectation::Above(0.5)));
    }

    #[test]
    fn identifiers() {
        for ok in ["baseline", "pt_br", "novel-rules-001", "h2"] {
            assert!(is_valid_id(ok), "{ok}");
        }
        for bad in ["", "Baseline", "pt-BR", "a--b", "-a", "a_", "a b"] {
            assert!(!is_valid_id(bad), "{bad}");
        }
    }

    #[test]
    fn dates() {
        assert!(is_iso_date("2026-10-01"));
        for bad in [
            "2026-13-01",
            "2026-1-01",
            "26-10-01",
            "2026/10/01",
            "2026-10-00",
        ] {
            assert!(!is_iso_date(bad), "{bad}");
        }
    }
}
