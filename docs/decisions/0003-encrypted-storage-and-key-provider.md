# ADR-0003: Explicit encrypted storage with an external file-key provider

- **Status:** Proposed
- **Date:** 2026-08-27
- **Authors:** Manus AI
- **Supersedes:** None
- **Superseded by:** None

## Context

The VDB MVP stores its WAL, backup files, and manifests as plaintext. The existing SHA-256 checksum detects accidental corruption but does not provide confidentiality or authenticate an attacker who can rewrite both a payload and its checksum. The production-gap audit therefore keeps authenticated encryption and key management as a P0 gate.

VDB must remain local-first, explicit, and simple. The GUI and Steward remain read-only. Existing plaintext VDB1 files must not be silently reinterpreted as encrypted files, and existing plaintext backups must not be relabeled as encrypted. A future encryption implementation must use an established AEAD library, bind storage context as associated data, fail closed on authentication failure, keep keys outside the database file, support recovery and rotation, and preserve bounded record sizes.

This decision is informed by the RustCrypto `aes-gcm` documentation and OWASP cryptographic-storage and key-management guidance. RustCrypto documents AES-GCM as an AEAD implementation and requires a unique nonce for every message. OWASP recommends AES with at least a 128-bit key, preferably 256-bit, an authenticated mode such as GCM, cryptographically secure randomness, established algorithms, separation of keys from data, and a documented lifecycle for generation, storage, rotation, backup, compromise recovery, and destruction.

## Decision

VDB will add an **opt-in encrypted storage format version 3** using the maintained RustCrypto `aes-gcm` crate with `Aes256Gcm`. Encryption will never be enabled implicitly for an existing plaintext database.

The first provider boundary will be an explicit **external raw-key file provider**. The key file contains exactly 32 random bytes, is created with restrictive permissions where supported, is never copied into the database, is never printed or logged, and must be supplied explicitly to commands that open encrypted data. This deliberately avoids inventing a passphrase KDF or pretending that a plain environment variable is a secure key store. OS keychain, KMS/HSM, and passphrase-derived providers are deferred until they have separate platform-specific designs and tests.

The key provider API will expose only key material and a stable provider error. The storage layer will not know how a future keychain or KMS retrieves a key. The CLI will provide explicit key-file provisioning and rotation commands while keeping key contents out of command output. A key-file lifecycle is the operator’s responsibility: generate it with the VDB keygen command, protect it separately from the database and backups, back it up through an approved secret-management process, rotate it with the rekey workflow, revoke compromised keys, and destroy retired copies only after retention and recovery requirements are satisfied.

The VDB3 header will contain the format version, encryption suite identifier, random database key identifier, and the metadata required to identify the encrypted file. It will contain no raw key. The data-encryption key is supplied by the provider. Each encrypted WAL record will contain a monotonically increasing sequence number, a fresh 96-bit nonce, authenticated ciphertext, and bounded lengths. The associated data will bind the VDB3 format identifier, key identifier, sequence number, record kind, collection, document identifier, and document version. The record body remains encrypted. Record authentication failure, wrong-key failure, nonce or sequence errors, malformed ciphertext, and semantic replay errors fail closed; only an incomplete final physical record may use the existing bounded trailing-tail repair rule when no complete record follows it.

Plaintext metadata leakage is intentionally minimized. The encrypted record envelope may expose only the fields required to construct and verify associated data; document contents, timestamps, user payloads, and control-record bodies remain encrypted. A future design may encrypt more envelope metadata if it can retain bounded recovery and replay validation without weakening the authenticated context.

Encrypted backups will be byte-for-byte copies of the encrypted source plus a manifest that records the encrypted format and database key identifier. The manifest will not contain key material. Restore will preserve ciphertext and require the provider key when reopening and verifying the destination. Plaintext backups remain explicitly plaintext and are not upgraded by this decision.

Rotation will be implemented as an explicit, serialized rewrite into a same-directory temporary VDB3 file. It will decrypt and validate the complete source, re-encrypt records with a newly generated database key identifier and supplied replacement key, synchronize the temporary file, atomically replace the original only after successful validation, and retain the original on pre-replacement failure. A rotation operation will not claim that previously exported backups have been re-encrypted; old backups remain dependent on their original key and must follow the operator’s retention and revocation policy.

## Alternatives considered

| Alternative | Benefits | Costs or risks | Reason not chosen |
|---|---|---|---|
| Continue plaintext plus SHA-256 | No migration or dependency cost | No confidentiality or attacker authentication | Fails the P0 security objective |
| Custom encryption or custom passphrase KDF | Superficially simple API | High cryptographic and key-management risk; difficult to review | Explicitly prohibited by the security policy |
| AES-GCM with a key embedded beside the database | Easy deployment | A copied backup would contain both data and key; key/data separation would be false | Rejected; the provider key remains external |
| Passphrase mode in the first release | User-friendly provisioning | Requires careful Argon2id parameter storage, memory-hard limits, secure input handling, and migration tests | Deferred until a separate reviewed KDF design is complete |
| OS keychain/KMS/HSM only | Stronger operational isolation where available | Platform/provider-specific APIs and recovery paths would make the first implementation broader and harder to qualify | Deferred behind the provider interface |

## Consequences

Encrypted databases become unreadable without the external key file. Loss, corruption, or revocation of that key can make all dependent data and backups unrecoverable; operators must test key backup and restore procedures before relying on encryption. A user who opens a VDB3 file without a key must receive a typed error without a plaintext fallback.

The format version increases from the current VDB1 versions 1–2 to VDB3 for new encrypted files. Existing VDB1 files remain readable only as plaintext legacy files. Migration is explicit: export or compact into a newly initialized encrypted destination, verify it, and preserve the original until the operator confirms replacement. No in-place reinterpretation is allowed.

Authenticated encryption replaces the accidental-corruption-only checksum for encrypted records. A keyed authentication failure must not be converted into a truncated-tail repair or silently ignored. File lengths, record counts, payload limits, WAL limits, and replay state limits remain enforced before allocation or state publication. Encryption does not solve filesystem power-loss durability, OS permissions, memory-bounded indexing, or application authentication.

Backups remain collision-safe and manifest-verified, but verification now requires the correct external key for encrypted sources. Rotation changes the active database key identifier and creates an operational dependency on both old and new keys until old backups are retired. Diagnostics and health output may expose only an encrypted flag and non-secret key identifier; they must never expose key bytes, plaintext payloads, or passphrases.

## Validation and rollout

Implementation must not be considered complete until it includes unit and integration tests for new encrypted initialization, wrong keys, tampered header and ciphertext, nonce/sequence validation, bounded oversized encrypted records, replay recovery, legacy plaintext compatibility, encrypted backup verification, restore-to-new-path, key rotation, failed rotation preservation, and absence of secrets in errors and health output. Tests must use generated ephemeral keys and must not commit key material.

The rollout sequence is: first implement and test the provider and VDB3 format behind an explicit opt-in CLI; then add encrypted backup/restore and rotation; then execute isolated restore drills and document the operator lifecycle; finally obtain an independent security review of the design and implementation. The ADR remains **Proposed** until a maintainer review records acceptance. Even after acceptance and green CI, production readiness still requires the separate durability, locking/filesystem, byte-accounting, release-provenance, and operational evidence gates.

## References

[1]: https://docs.rs/aes-gcm "RustCrypto AES-GCM documentation"
[2]: https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html "OWASP Cryptographic Storage Cheat Sheet"
[3]: https://cheatsheetseries.owasp.org/cheatsheets/Key_Management_Cheat_Sheet.html "OWASP Key Management Cheat Sheet"
[4]: ../crypto-research.md "VDB cryptography and key-management research note"
[5]: ../security.md "VDB security model and requirements"
[6]: ../data-format.md "VDB storage-format contract"
