# sondamente

**Preregistered probes for philosophy-of-mind claims about LLMs.**

Claims that language models "understand", "reason", or "introspect" usually
rest on one of two weak supports: a striking transcript, or a benchmark score
disconnected from any particular philosophical position. sondamente makes the
inference explicit instead. Every probe states a claim, at least two rival
hypotheses about it, and what each hypothesis predicts under named conditions,
all written down before any data exists.

The validator enforces one rule above the rest: **a probe must contain a
crucial condition**, a condition and metric on which two rival hypotheses
predict outcomes that cannot both occur. If no such condition exists, no
possible result favors one hypothesis over another, and running the probe
cannot move the debate. The name comes from the Portuguese *sonda* ("probe");
*sondamente* reads as "probingly".

> **Status: v0.0.1.** This release contains the spec format, the validator and
> the preregistration hash. Runners, generators and analysis come next (see
> [Roadmap](#roadmap)).

## A probe

```yaml
spec_version: "0.1"
id: systematicity-novel-rules-001
title: Systematic generalization over novel rule systems
claim:
  statement: >
    A system that grasps a rule system should apply it equally well when the
    symbols expressing it are replaced or its premises are reordered.
  sources: ["Fodor & Pylyshyn (1988). Cognition, 28(1-2), 3-71."]
tier: behavioral
hypotheses:
  - id: systematic
    description: Accuracy is invariant under structure-preserving changes.
    predictions:
      - { condition: renamed_symbols, metric: accuracy_delta, expect: { within: [-0.05, 0.05] } }
  - id: surface_statistics
    description: Structure-preserving surface changes degrade accuracy.
    predictions:
      - { condition: renamed_symbols, metric: accuracy_delta, expect: { below: -0.10 } }
conditions:
  - { id: baseline, description: Conventional symbols., baseline: true }
  - { id: renamed_symbols, description: Symbols replaced by nonsense tokens. }
metrics:
  - { id: accuracy_delta, description: Accuracy minus baseline accuracy. }
generator: { path: generators/novel_rule_systems.py }
sampling: { n_per_condition: 200 }
```

The full version, which adds a shuffled-premise condition and a baseline
accuracy check, is in [`probes/`](probes/systematicity-novel-rules-001.yaml).

Expectations take one of three forms: `{ below: x }` and `{ above: x }` are
strict, while `{ within: [a, b] }` includes its endpoints. That distinction
matters: `below: -0.1` and `within: [-0.1, 0]` cannot both hold, so they
discriminate, but `within: [-0.1, 0]` and `within: [0, 0.1]` overlap at 0,
so they do not.

## Usage

```console
$ cargo install sondamente

$ sondamente validate probes/*.yaml
probes/systematicity-novel-rules-001.yaml: ok (0 warning(s))

$ sondamente hash probes/systematicity-novel-rules-001.yaml
sha256:68d8c27d…
```

`validate` exits with 1 if any spec has errors and 2 if a file cannot be read
or parsed, so it works as a CI gate.

## Preregistration

To lock a probe, record its hash and the date:

```yaml
preregistration:
  registered: "2026-10-01"
  spec_hash: "sha256:68d8c27deec992146f10a795c3c90d8d3f7d2bd5c8b5bb2cfffeebcb7ef226c8"
```

From then on, `validate` fails with `spec-changed-after-preregistration` if
anything the spec *says* changes. The hash is computed over the parsed
content, so comments, whitespace, key order and quoting can be edited freely.
The `preregistration` block is excluded from its own hash. A probe that needs
to change after registration gets a new id; the old one stays as it was.

## Validation rules

| Code | Severity | Meaning |
|---|---|---|
| `no-rival-hypotheses` | error | Fewer than two hypotheses. |
| `no-crucial-condition` | error | No condition and metric on which two hypotheses predict non-overlapping outcomes. |
| `indistinguishable-hypotheses` | warning | Some pair of hypotheses is never discriminated, although others are. |
| `hypothesis-without-predictions` | error | A hypothesis predicts nothing. |
| `unknown-condition`, `unknown-metric` | error | A prediction refers to something undeclared. |
| `duplicate-prediction` | error | A hypothesis predicts the same condition and metric twice. |
| `empty-interval`, `non-finite-bound` | error | An expectation that no finite value can satisfy. |
| `no-baseline`, `multiple-baselines` | error | Exactly one condition must be `baseline: true`. |
| `invalid-id`, `duplicate-id` | error | Ids are lowercase letters and digits joined by `-` or `_`, unique per kind. |
| `unsupported-version` | error | `spec_version` is not `"0.1"`. |
| `empty-title`, `empty-claim` | error | Required text is blank. |
| `empty-sample`, `missing-generator` | error | Nothing would be run. |
| `invalid-date`, `malformed-hash` | error | The preregistration block is malformed. |
| `spec-changed-after-preregistration` | error | The spec's content no longer matches its recorded hash. |
| `claim-without-source` | warning | The claim cites no source. |
| `unused-condition`, `unused-metric` | warning | Declared but never predicted. |

Unknown fields are rejected when parsing, so a typo such as `samplng:` cannot
silently drop part of a probe.

## What sondamente does not claim

Behavioral evidence underdetermines claims about minds. A probe can show that
a model's behavior fits one hypothesis's predictions better than a rival's; it
cannot show that the model understands, or is conscious, in any sense beyond
what the hypothesis operationalizes. Making that gap visible, probe by probe,
is the point of stating the hypotheses explicitly.

## Roadmap

- **Generators** for the example probe, producing fresh instances per run so
  results cannot come from memorized data.
- **Behavioral runner** for API models: pinned model identifiers, sampling
  parameters and seeds, with immutable raw outputs stored per run.
- **Scoring and reports** as pure functions over stored outputs, with
  confidence intervals, so anyone can re-score without re-running.
- **Mechanistic tier**: a Python adapter for probes that need activation
  access on open-weight models, reading and writing the same formats.
- **Replications** of published experiments on LLM introspection and
  self-report, restated with explicit rival predictions.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE), at your option.
