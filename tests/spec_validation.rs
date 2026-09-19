use sondamente::{from_yaml_str, load, spec_hash, validate, Report};

/// A minimal valid spec. `HYPOTHESES` is replaced per test.
const TEMPLATE: &str = r#"
spec_version: "0.1"
id: test-probe
title: Test probe
claim:
  statement: A claim.
  sources: ["Someone (2000)."]
tier: behavioral
hypotheses:
HYPOTHESES
conditions:
  - { id: baseline, description: unmodified, baseline: true }
  - { id: perturbed, description: modified }
metrics:
  - { id: delta, description: change against baseline }
generator: { path: generators/example.py }
sampling: { n_per_condition: 100 }
"#;

const RIVALS: &str = r#"
  - id: invariant
    description: unaffected by the perturbation
    predictions:
      - { condition: perturbed, metric: delta, expect: { within: [-0.05, 0.05] } }
  - id: fragile
    description: degraded by the perturbation
    predictions:
      - { condition: perturbed, metric: delta, expect: { below: -0.1 } }
"#;

fn spec_yaml(hypotheses: &str) -> String {
    TEMPLATE.replace("HYPOTHESES", hypotheses.trim_matches('\n'))
}

fn report(yaml: &str) -> Report {
    validate(&from_yaml_str(yaml).expect("test spec should parse"))
}

fn error_codes(yaml: &str) -> Vec<&'static str> {
    report(yaml).errors().map(|d| d.code).collect()
}

fn warning_codes(yaml: &str) -> Vec<&'static str> {
    report(yaml).warnings().map(|d| d.code).collect()
}

#[test]
fn example_probe_is_valid_and_clean() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/probes/systematicity-novel-rules-001.yaml"
    );
    let report = validate(&load(path).unwrap());
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
}

#[test]
fn rival_hypotheses_with_a_crucial_condition_pass() {
    let r = report(&spec_yaml(RIVALS));
    assert!(r.diagnostics.is_empty(), "{:#?}", r.diagnostics);
}

#[test]
fn a_single_hypothesis_is_rejected() {
    let single = r#"
  - id: invariant
    description: unaffected by the perturbation
    predictions:
      - { condition: perturbed, metric: delta, expect: { within: [-0.05, 0.05] } }
"#;
    assert_eq!(error_codes(&spec_yaml(single)), ["no-rival-hypotheses"]);
}

#[test]
fn overlapping_predictions_have_no_crucial_condition() {
    let overlapping = r#"
  - id: invariant
    description: unaffected
    predictions:
      - { condition: perturbed, metric: delta, expect: { within: [-0.05, 0.05] } }
  - id: slightly_worse
    description: somewhat degraded
    predictions:
      - { condition: perturbed, metric: delta, expect: { below: 0.0 } }
"#;
    assert_eq!(
        error_codes(&spec_yaml(overlapping)),
        ["no-crucial-condition"]
    );
}

#[test]
fn closed_intervals_touching_at_a_point_are_not_crucial() {
    let touching = r#"
  - id: a
    description: nonpositive change
    predictions:
      - { condition: perturbed, metric: delta, expect: { within: [-0.1, 0.0] } }
  - id: b
    description: nonnegative change
    predictions:
      - { condition: perturbed, metric: delta, expect: { within: [0.0, 0.1] } }
"#;
    assert_eq!(error_codes(&spec_yaml(touching)), ["no-crucial-condition"]);
}

#[test]
fn predictions_on_different_conditions_do_not_discriminate() {
    let disjoint_but_unshared = r#"
  - id: a
    description: predicts only under the perturbation
    predictions:
      - { condition: perturbed, metric: delta, expect: { below: -0.1 } }
  - id: b
    description: predicts only at baseline
    predictions:
      - { condition: baseline, metric: delta, expect: { above: 0.1 } }
"#;
    assert_eq!(
        error_codes(&spec_yaml(disjoint_but_unshared)),
        ["no-crucial-condition"]
    );
}

#[test]
fn a_redundant_third_hypothesis_is_a_warning() {
    let three = format!(
        "{}{}",
        RIVALS.trim_end(),
        r#"
  - id: also_invariant
    description: same prediction as `invariant`
    predictions:
      - { condition: perturbed, metric: delta, expect: { within: [-0.03, 0.03] } }
"#
    );
    let yaml = spec_yaml(&three);
    assert!(error_codes(&yaml).is_empty());
    assert_eq!(warning_codes(&yaml), ["indistinguishable-hypotheses"]);
}

#[test]
fn references_and_intervals_are_checked() {
    let broken = r#"
  - id: invariant
    description: typo in condition, inverted interval
    predictions:
      - { condition: perturbd, metric: delta, expect: { below: -0.1 } }
      - { condition: perturbed, metric: delta, expect: { within: [0.1, -0.1] } }
  - id: fragile
    description: degraded
    predictions:
      - { condition: perturbed, metric: delta, expect: { below: -0.1 } }
"#;
    let codes = error_codes(&spec_yaml(broken));
    assert!(codes.contains(&"unknown-condition"), "{codes:?}");
    assert!(codes.contains(&"empty-interval"), "{codes:?}");
}

#[test]
fn unknown_fields_are_rejected_at_parse_time() {
    let typo = spec_yaml(RIVALS).replace("sampling:", "samplng:");
    assert!(from_yaml_str(&typo).is_err());
}

#[test]
fn hash_ignores_formatting_but_not_content() {
    let original = spec_yaml(RIVALS);
    let hash = spec_hash(&from_yaml_str(&original).unwrap());

    let reformatted = format!(
        "# a comment\n{}\n\n",
        original.replace("title: Test probe", "title: \"Test probe\"")
    );
    assert_eq!(hash, spec_hash(&from_yaml_str(&reformatted).unwrap()));

    let changed = original.replace("n_per_condition: 100", "n_per_condition: 50");
    assert_ne!(hash, spec_hash(&from_yaml_str(&changed).unwrap()));
}

#[test]
fn preregistration_detects_later_edits() {
    let original = spec_yaml(RIVALS);
    let hash = spec_hash(&from_yaml_str(&original).unwrap());
    let locked = format!(
        "{original}preregistration:\n  registered: \"2026-10-01\"\n  spec_hash: \"{hash}\"\n"
    );
    assert!(report(&locked).diagnostics.is_empty());

    let edited = locked.replace("below: -0.1", "below: -0.2");
    assert_eq!(error_codes(&edited), ["spec-changed-after-preregistration"]);
}

#[test]
fn an_expectation_needs_exactly_one_bound() {
    let two_keys = spec_yaml(RIVALS).replace("{ below: -0.1 }", "{ below: -0.1, above: 0.2 }");
    let err = from_yaml_str(&two_keys).unwrap_err().to_string();
    assert!(err.contains("exactly one of"), "{err}");
}
