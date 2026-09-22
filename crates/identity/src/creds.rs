//! Credential hashing — the production secret-storage primitive (NFR-2.3).
//!
//! **Scheme: Argon2id (argon2's default params), per-credential random salt, PHC-string storage.**
//!
//! A member secret arrives as an opaque `&str` (we never interpret or constrain it). We hash it
//! with [Argon2id](https://en.wikipedia.org/wiki/Argon2) using a fresh random salt per credential
//! and store **only** the resulting [PHC string](https://github.com/P-H-C/phc-string-format)
//! (`$argon2id$v=19$m=...,t=...,p=...$<salt>$<hash>`). The plaintext is never persisted — the
//! store keeps the hash and nothing else.
//!
//! Parameters are the argon2 crate defaults (Argon2id, v19). Because the chosen parameters are
//! embedded in each PHC string, the **upgrade path** is to re-hash a secret on the next successful
//! login (verify against the old hash, then `hash_secret` again with current defaults and replace
//! the stored string) — no migration of at-rest data is required.

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

/// Hash a secret with Argon2id and a fresh random salt, returning the PHC string to store.
///
/// The returned string is the **only** thing that should be persisted for a credential
/// (NFR-2.3): it carries the algorithm, parameters, salt, and digest. Two calls with the same
/// secret return different strings (independent random salts).
/// Shortest secret a member may *choose* (SQUIRE-T-0131) — `change_secret` and the `reset_secret`
/// recovery tool. Deliberately modest: the login throttle (SQUIRE-T-0130), not length, is what bounds
/// guessing, and a parent has to be able to type this on a phone.
pub const MIN_SECRET_LEN: usize = 8;

pub fn hash_secret(secret: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(secret.as_bytes(), &salt)
        .expect("argon2 hashing of an in-memory secret cannot fail")
        .to_string()
}

/// Verify a secret against a stored PHC hash in constant time.
///
/// Returns `true` only when `stored_phc` is a well-formed hash that `secret` satisfies. Malformed
/// input (not a PHC string, unknown algorithm, etc.) returns `false` and never panics — the
/// comparison itself is constant-time (delegated to argon2's `verify_password`).
pub fn verify_secret(secret: &str, stored_phc: &str) -> bool {
    let parsed = match PasswordHash::new(stored_phc) {
        Ok(p) => p,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(secret.as_bytes(), &parsed)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_accepts_matching_secret() {
        let h = hash_secret("excalibur");
        assert!(verify_secret("excalibur", &h));
    }

    #[test]
    fn verify_rejects_wrong_secret() {
        let h = hash_secret("excalibur");
        assert!(!verify_secret("wrong", &h));
    }

    #[test]
    fn two_hashes_of_same_secret_differ() {
        // Independent random salts → distinct PHC strings.
        assert_ne!(hash_secret("same"), hash_secret("same"));
    }

    #[test]
    fn verify_of_malformed_hash_is_false_not_panic() {
        assert!(!verify_secret("x", "not-a-phc-string"));
    }
}
