# Cryptography and Key-Management Research

**Review date:** 2026-08-24
**Status:** Design input only; no encryption implementation is authorized by this note.

## Findings

The RustCrypto `aes-gcm` crate documents AES-GCM as an authenticated-encryption-with-associated-data implementation and explicitly states that a nonce must be unique for every message. Its usage example uses an operating-system-backed random nonce generator and `Aes256Gcm`; the crate also documents a prior NCC Group review with no significant findings at the time of that review. VDB must not copy an example without separately defining nonce lifecycle, associated-data binding, key rotation, failure handling, and backup compatibility. [1]

OWASP recommends AES with at least a 128-bit key, preferably 256-bit, and an authenticated mode such as GCM or CCM. It also requires cryptographically secure randomness for security-critical values and warns against custom algorithms. [2]

OWASP’s key-management guidance requires a documented lifecycle covering generation, distribution, storage, rotation, destruction, compromise recovery, backup/escrow, and accountability. It recommends separate keys for separate purposes, protecting keys in dedicated cryptographic or operating-system facilities where available, and using envelope-encryption separation between data-encryption keys and key-encryption keys when applicable. [3]

## VDB design constraints

The first encrypted format must be a versioned format migration, not a silent reinterpretation of existing plaintext VDB1 files. It must authenticate the file header, record type, collection, document identifier, version, and key identifier as associated data; generate a unique nonce per encrypted record; fail closed on authentication errors; and preserve bounded record sizes. Key material must never be committed, logged, printed in health output, or stored beside plaintext without an explicit threat-model decision.

The CLI must make encryption opt-in and explicit, with a documented provisioning path. A passphrase-based mode requires a reviewed memory-hard KDF and parameter record; an OS-keychain or external provider mode requires platform-specific implementations and recovery procedures. A key identifier must support rotation and encrypted restore drills. Existing plaintext backups must remain clearly labeled and must not be presented as encrypted merely because their manifest has a SHA-256 checksum.

No production-readiness claim should be made until the selected AEAD design, provider boundary, key lifecycle, format migration, rotation, corruption behavior, and backup/restore drills are implemented and independently reviewed.

## References

[1]: https://docs.rs/aes-gcm "RustCrypto aes-gcm crate documentation"
[2]: https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html "OWASP Cryptographic Storage Cheat Sheet"
[3]: https://cheatsheetseries.owasp.org/cheatsheets/Key_Management_Cheat_Sheet.html "OWASP Key Management Cheat Sheet"
