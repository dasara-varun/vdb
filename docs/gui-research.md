# VDB GUI Research and Design Decision

**Review date:** 2026-08-24

## Decision summary

VDB should gain a **local, read-only browser GUI first**, launched explicitly by the existing CLI and bound only to `127.0.0.1`. This provides immediate usability without adding a remote server, authentication system, JavaScript runtime, or desktop packaging dependency to the database core. The first GUI milestone should expose health, collections, bounded document browsing, and safe recovery links; all mutations remain in the CLI until a separately reviewed local application API exists.

A future native desktop shell may use Tauri because it provides Rust-backed application logic, native system WebViews, and a capability model for restricting frontend access. Tauri’s own security guidance emphasizes that frontend and Rust code are different trust groups and that data crossing the IPC boundary must be strongly defined.[1] Its capability system allows permissions to be granted to specific windows or WebViews, while remote sources are not enabled by default.[2] However, adopting Tauri immediately would add a second build and packaging ecosystem before VDB has a stable local API, authenticated encryption, or reproducible dependency supply chain. The browser GUI is therefore an intentionally smaller first step, not a claim that a browser listener is production-safe for remote exposure.

## Security requirements

The GUI must bind to the IPv4 loopback address only, never `0.0.0.0`, a configurable LAN address, or an externally reachable hostname. It must use read-only GET endpoints in the first milestone, reject other HTTP methods, cap request-line and response sizes, bound collection/document/query parameters, and return structured JSON errors without exposing filesystem paths or internal stack traces. It must serve only embedded local assets and must not fetch remote scripts, load user-controlled HTML, or provide arbitrary file browsing. Every database operation must reuse the core’s existing validation and limits.

The GUI must be opt-in rather than automatically started when opening a database. It should print the exact loopback URL and the selected database path should remain controlled by the CLI. A future mutation API requires a written threat model, explicit user confirmation, CSRF or equivalent origin controls, optimistic-version enforcement, audit logging, and tests for path traversal, request smuggling, denial-of-service budgets, and unauthorized state changes.

Electron’s official security checklist reinforces the same principle: desktop web content should not receive unnecessary native authority, navigation and new-window creation should be limited, a restrictive Content Security Policy should be used, and IPC senders should be validated.[3] These requirements support keeping VDB’s first GUI local, read-only, embedded, and free of remote content.

## Usability goals

The first screen should answer the questions a new operator has immediately: whether the database is healthy, how much WAL and payload space it uses, which collections exist, and how to inspect a bounded sample of documents. The interface should offer copyable CLI commands for mutations and recovery rather than silently performing them. It should clearly label the mode as **read-only** and link to the operations and API guides.

The most valuable follow-on improvements are a visible restore drill workflow, schema and index summaries, bounded query forms with JSON validation, export and backup status, and a desktop wrapper only after the local API contract is stable. The GUI should not become a second database implementation; the Rust core remains the only source of truth.

## Release boundary

A local read-only GUI is a developer-experience feature and does not remove VDB’s production gates. It does not provide network service, authentication, encryption, power-loss guarantees, or multi-user concurrency. The release gate remains blocked until the durability, encryption/key-management, dependency-lockfile, memory-boundedness, and API security work documented in `docs/production-gap-audit.md` is complete.

## References

[1]: https://v2.tauri.app/security/ "Tauri Security"

[2]: https://v2.tauri.app/security/capabilities/ "Tauri Capabilities"

[3]: https://www.electronjs.org/docs/latest/tutorial/security "Electron Security Recommendations"

## Maintenance

Update this note whenever the GUI transport, trust boundary, exposed endpoint set, native wrapper, or security assumptions change. Do not describe the GUI as remote-capable or production-certified without updating the threat model, tests, installation guidance, and production-gap audit.

*Author: Manus AI*

*Last updated: 2026-08-24*

*Status: design decision; implementation must remain bounded and read-only for the first milestone.*

> This document is a design input. It does not override `AGENTS.md`, `docs/security.md`, or the release gates.
