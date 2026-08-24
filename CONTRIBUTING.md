# Contributing to VDB

Thank you for helping build VDB. VDB values **small, inspectable, safe changes** over feature count. Contributors should preserve the project’s local-first design, keep the Steward read-only by default, and make the repository understandable to a beginner, an expert, and a future maintainer.

## Before coding

Read [`AGENTS.md`](AGENTS.md), [`README.md`](README.md), [`docs/README.md`](docs/README.md), and [`docs/project-status.md`](docs/project-status.md). Then read the task-specific documents identified by the navigation guide. Inspect the current branch and working tree before editing.

Classify the change before implementation:

| Change type | Required design and documentation |
|---|---|
| Bug fix | Reproduction or explanation, regression test, affected guide, and changelog entry when user-visible |
| Storage or recovery | Format contract, migration/recovery procedure, failure-injection test, security review, and compatibility note |
| Security | Threat-model update, abuse cases, least-privilege analysis, negative tests, and explicit limitation changes |
| CLI or API | Contract examples, validation/error behavior, compatibility note, and smoke or contract tests |
| Performance | Baseline workload, measurement method, correctness test, regression threshold, and performance-document update |
| Steward or AI | Context boundary, typed output, prompt-injection analysis, no-side-effect test, approval behavior, and security-document update |
| Documentation only | Navigation link, source references where relevant, and status/changelog update if project understanding changes |

## During implementation

Keep changes focused and preserve existing behavior unless the change explicitly requires a contract update. Avoid unchecked assumptions in storage and recovery code. Validate user input before durable writes. Do not add a network listener, arbitrary command execution, automatic destructive repair, or model-controlled mutation without an accepted decision record and owner approval.

If an external source, benchmark, incident, or user decision changes the rationale, record it in the appropriate guide or a dated research note. If the change introduces a durable architectural choice, create a decision record from [`docs/templates/decision-record.md`](docs/templates/decision-record.md).

## Documentation obligations

Documentation is part of the implementation. Update the smallest set of source-of-truth documents that fully explains the change, then update the navigation index and status snapshot when needed. Do not merely append contradictory notes; reconcile the existing guide and mark superseded decisions clearly.

At minimum, a completed change should answer the following questions for a future reader: What changed? Why was it changed? What assumptions and limits exist? How is it tested? How can an operator recover from failure? Which files are the source of truth? What work remains?

## Validation

Run these commands before opening a pull request:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

Run the CLI smoke workflow for lifecycle or CLI changes. Run the relevant recovery, corruption, concurrency, or benchmark checks for storage changes. Use `git diff --check` and inspect the final diff for stale claims, broken links, accidental secrets, generated artifacts, and untracked documentation.

## Pull requests

A pull request description should state the user problem, scope, design decision, tests run, documentation updated, security implications, performance impact, migration/recovery implications, and known limitations. Use a focused title and keep unrelated cleanup separate.

Reviewers should reject changes that make unsupported durability claims, silently expand the Steward’s authority, omit a required regression test, leave the status snapshot stale, or create a second conflicting source of truth. Green CI is necessary but is not by itself a production-readiness claim.

## Release and post-merge maintenance

After merge, update `docs/project-status.md` with the new commit or milestone when the change affects status, and update the changelog for user-visible behavior. Periodically review the roadmap, production-gap audit, dependency posture, examples, and diagram sources for drift. Record significant decisions rather than relying on chat history or issue context that future maintainers may not see.
