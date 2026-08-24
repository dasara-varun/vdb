## Change summary

Describe the user problem and the implementation in complete sentences.

## Change classification

- [ ] Bug fix
- [ ] Storage or recovery
- [ ] Security or privacy
- [ ] CLI or API
- [ ] Performance
- [ ] Steward or AI
- [ ] Documentation only
- [ ] Other: ________

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cargo build --workspace --release`
- [ ] CLI/API smoke or recovery test run when relevant
- [ ] `git diff --check`

## Documentation synchronization

- [ ] Affected source-of-truth guides are updated.
- [ ] `docs/project-status.md` is updated if implementation, validation, priorities, or limitations changed.
- [ ] `CHANGELOG.md` is updated for user-visible behavior.
- [ ] `docs/README.md` is updated for new, renamed, or superseded documents.
- [ ] Relevant diagrams and rendered assets are updated.
- [ ] Research references or an ADR were added when the rationale changed.

## Safety and compatibility

- [ ] Security and privacy implications are documented.
- [ ] Recovery, migration, and failure behavior are documented where relevant.
- [ ] No unsupported production-readiness claim was added.
- [ ] No arbitrary shell access, silent mutation, remote exposure, or unrestricted model authority was introduced.
- [ ] Known limitations and follow-up work are stated below.

## Remaining limitations and follow-up

<!-- State what is not proven, what remains incomplete, and the next recommended action. -->
