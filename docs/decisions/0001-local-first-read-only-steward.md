# ADR-0001: Local-first storage with a read-only Steward by default

- **Status:** accepted
- **Date:** 2026-08-24
- **Authors:** VDB project owner and Manus AI
- **Supersedes:** none
- **Superseded by:** none

## Context

VDB is intended to be simple enough for beginners while remaining useful to expert developers and operators. A built-in AI component could reduce the effort needed to understand health, schema, indexes, and recovery, but model output is not inherently trustworthy. Document payloads may contain untrusted instructions, secrets, or adversarial text, and a model with mutation or shell authority could cause data loss or exfiltration.

The storage engine is therefore local-first and explicit. The initial database must be usable without a network listener, external service, or model. The Steward should provide useful deterministic observations without requiring an LLM.

## Decision

VDB will remain a local-first document database. The default Steward mode is read-only and deterministic. It may inspect bounded, minimized database context and emit findings or typed proposals, but it cannot directly mutate documents, execute shell commands, alter policy, or expose data remotely.

Any future model-backed Steward must be optional, preferably local or private, and isolated behind a typed interface. Proposed actions must pass schema validation, policy checks, explicit user approval, and post-action verification. A future change that expands Steward authority requires a superseding ADR, threat-model update, adversarial tests, and updated operational documentation.

## Alternatives considered

| Alternative | Benefits | Costs or risks | Reason not chosen |
|---|---|---|---|
| Model with direct database and shell access | Maximum automation | Prompt injection, destructive actions, data exfiltration, difficult auditability | Violates least privilege and safe defaults |
| Remote model as a required dependency | Stronger model capacity | Privacy, availability, cost, credentials, and network attack surface | Conflicts with local-first and beginner-friendly operation |
| No Steward at all | Smallest attack surface | Misses the usability and diagnostic opportunity | Deterministic read-only findings provide value safely |
| Deterministic read-only Steward first | Predictable, auditable, offline, and safe | Less natural-language explanation | Chosen baseline; model explanation can be added later behind controls |

## Consequences

VDB must keep the Steward’s context boundary and action contract documented. The deterministic implementation remains useful when no model is configured. The product will initially offer less automation than an unrestricted agent, but it will be easier to explain, test, recover, and trust.

The local-first choice also means that encryption, filesystem permissions, process locking, backup handling, and future API exposure require explicit operational guidance. It does not imply that the current MVP is production-ready; the production-gap audit remains authoritative.

## Validation and rollout

The decision is validated by the current read-only Steward output, unit tests, security documentation, and the absence of generic shell or unrestricted model tools in the MVP. Future Steward work must add no-side-effect tests, prompt-injection fixtures, typed-output validation, approval-gate tests, and documentation updates before release.

## References

- [`docs/steward.md`](../steward.md)
- [`docs/security.md`](../security.md)
- [`docs/product.md`](../product.md)
- [`docs/production-gap-audit.md`](../production-gap-audit.md)
