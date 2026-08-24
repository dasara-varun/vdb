# ADR-0002: Opt-in loopback read-only GUI

- **Status:** Accepted
- **Date:** 2026-08-24
- **Owners:** VDB maintainers
- **Supersedes:** None

## Context

VDB is local-first and currently exposes database operations through its CLI. Developers need an easier way to inspect health, collections, and representative documents, but a GUI can accidentally become a network service, a second database implementation, or an unreviewed mutation API. VDB also does not yet have authenticated encryption, an authentic dependency lockfile, cross-platform crash evidence, memory-bounded storage, or a mature authenticated local API.

The first usability milestone therefore needs to improve inspection without expanding the trust boundary or weakening the existing storage contract. The preserved research note in [`docs/gui-research.md`](../gui-research.md) evaluates native wrappers and browser delivery and recommends a dependency-minimal first step.

## Decision

VDB will provide an **opt-in, CLI-launched, read-only browser GUI** in the CLI process. The server will bind to the IPv4 loopback address `127.0.0.1` and an operator-selected port, defaulting to an ephemeral port. It will print the exact URL and will not launch a browser automatically.

The first implementation will use the Rust standard library and embedded HTML/CSS. It will not add Tauri, Electron, a frontend package manager, remote assets, JavaScript execution, or a separate server process. The GUI will reuse `VdbStore` for every read and will expose only:

| Route | Purpose | Limit |
|---|---|---|
| `GET /` | Local dashboard with health and collection links | Embedded HTML only |
| `GET /collection?collection=...` | Bounded, read-only document sample | At most 100 documents |
| `GET /api/health` | Machine-readable health summary | Core-generated response |
| `GET /api/collections` | Machine-readable collection names | Core-generated response |
| `GET /api/documents?collection=...&limit=...` | Bounded document listing, optionally with a JSON equality filter | `limit` 1–100; filter and request sizes capped |

All other routes return an error. Only `GET` is allowed. The GUI will not create, replace, patch, delete, compact, import, export, back up, restore, run arbitrary queries, browse filesystem paths, or invoke Steward/model actions. Writes and recovery operations remain explicit CLI commands.

## Security boundary

The GUI is intended for a trusted local operator on the same device. Loopback binding is a reachability restriction, not authentication: another local process may still be able to connect. The GUI therefore remains read-only and must not be exposed through a proxy, port forward, container bridge, LAN address, or public hostname. It serves no user-controlled markup or remote resources.

Request parsing is bounded and rejects malformed request lines, unsupported methods, oversized headers, invalid paths, duplicate or malformed query parameters, invalid percent encoding, oversized collection names, and out-of-range limits. Responses include a restrictive Content Security Policy, `X-Content-Type-Options: nosniff`, `Cache-Control: no-store`, `Referrer-Policy: no-referrer`, and `frame-ancestors 'none'`. Document and collection values are HTML-escaped before rendering.

These controls do not provide authentication, CSRF protection for future writes, authenticated encryption, process isolation, power-loss durability, or multi-user authorization. They must not be presented as production certification.

## Alternatives considered

| Alternative | Decision | Reason |
|---|---|---|
| Tauri desktop shell now | Deferred | Adds packaging, WebView, capability, and dependency-supply-chain surfaces before the local API and encryption designs are ready |
| Electron desktop shell now | Deferred | Adds a larger runtime and native-authority surface that is disproportionate to an inspection-only milestone |
| LAN-capable HTTP server | Rejected | Expands reachability and requires authentication, authorization, TLS, audit, rate limits, and a complete API threat model |
| GUI mutation endpoints | Rejected for this milestone | Mutation requires explicit authorization, version checks, auditability, confirmation, and failure-injection coverage |
| Embedded remote frontend assets | Rejected | Remote content creates an unnecessary supply-chain and data-exfiltration boundary |

## Consequences

The GUI is immediately useful for local inspection while preserving the CLI as the only mutation and recovery interface. It adds no runtime dependency beyond the existing Rust workspace and is testable through deterministic route and parser tests plus a CLI smoke test.

The intentionally small interface is not a general database administration API. It does not include authentication or a browser compatibility promise beyond ordinary standards-based HTML. A future desktop shell or authenticated local API may be considered only through a new ADR after the production-gap audit’s higher-priority gates have evidence.

## Validation requirements

Changes to this decision require tests for loopback binding, method and request-size rejection, route behavior, unavailable collections, bounded results, security headers, output escaping, and server shutdown. Documentation must update [`docs/api.md`](../api.md), [`docs/security.md`](../security.md), [`docs/operations.md`](../operations.md), [`docs/testing.md`](../testing.md), and [`docs/production-gap-audit.md`](../production-gap-audit.md) when the contract changes.

## References

- [`docs/gui-research.md`](../gui-research.md)
- [`docs/security.md`](../security.md)
- [`docs/production-gap-audit.md`](../production-gap-audit.md)

*Author: Manus AI*
