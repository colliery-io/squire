//! Bearer tokens — the API's per-request credential (ADR SQUIRE-A-0004).
//!
//! **Scheme: a self-contained HMAC-SHA256-signed bearer token** carrying the claims
//! `(household, user, role, exp)`. The token is a compact
//! `base64url(claims_json) "." base64url(HMAC-SHA256(key, claims_json))` — *not* a full JWT, to
//! keep the dependency surface small, but with the same three properties the auth seam needs:
//!
//! - **No DB lookup to verify.** [`TokenSigner::verify`] recomputes the MAC and decodes the claims;
//!   it never touches the store. (`DevIdentity`'s in-memory token map is a separate, dev-only path.)
//! - **Tamper-evident.** The MAC is over the exact claims bytes; any edit fails the constant-time
//!   [`verify_slice`](hmac::Mac::verify_slice) check → [`AuthError::BadToken`].
//! - **Expiry-enforced.** `exp` (unix millis) is a claim; a token whose `exp <= now_ms` is rejected.
//!
//! The token carries its own `household`, so the same format works for the LAN-local single-tenant
//! MVP and a future multi-tenant hosted deployment (the tenant travels in the token, not the URL).
//!
//! The current time is **injected** (`now_ms`) on both issue and verify so callers stay
//! deterministic — pass it from `store::SystemClock`; this module never reads the system clock.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use domain_core::contract::{AuthToken, HouseholdHandle, Role, UserId};

use crate::{AuthError, Principal};

type HmacSha256 = Hmac<Sha256>;

/// The signed payload. Serialized to JSON, base64url'd, and MAC'd. `user` is carried as the inner
/// `u128` and `household` as its inner `String`, so the claims round-trip back to a [`Principal`]
/// exactly. `exp` is unix millis (`now_ms + ttl_ms` at issue time).
#[derive(Serialize, Deserialize)]
struct Claims {
    household: String,
    user: u128,
    role: Role,
    exp: i64,
}

/// Holds the server signing key and issues / verifies [`AuthToken`]s.
///
/// The key is shared-secret HMAC material (any byte string); keep it server-side only. Two signers
/// with different keys cannot verify each other's tokens.
pub struct TokenSigner {
    key: Vec<u8>,
}

impl TokenSigner {
    /// A signer over the given HMAC key.
    pub fn new(key: &[u8]) -> Self {
        Self { key: key.to_vec() }
    }

    /// Compute the HMAC-SHA256 of `payload` under the signing key.
    fn mac(&self, payload: &[u8]) -> Vec<u8> {
        let mut mac =
            HmacSha256::new_from_slice(&self.key).expect("HMAC accepts a key of any length");
        mac.update(payload);
        mac.finalize().into_bytes().to_vec()
    }

    /// Issue a token for `principal`, expiring at `now_ms + ttl_ms`.
    ///
    /// Encodes the claims as `base64url(json)` and appends `"." + base64url(HMAC(key, json))`.
    pub fn issue(&self, principal: &Principal, now_ms: i64, ttl_ms: i64) -> AuthToken {
        let claims = Claims {
            household: principal.household.0.clone(),
            user: principal.user.0,
            role: principal.role,
            exp: now_ms + ttl_ms,
        };
        let json = serde_json::to_vec(&claims).expect("Claims serialize to JSON");
        let payload = URL_SAFE_NO_PAD.encode(&json);
        let sig = URL_SAFE_NO_PAD.encode(self.mac(payload.as_bytes()));
        AuthToken(format!("{payload}.{sig}"))
    }

    /// Verify a token's signature and expiry, returning the carried [`Principal`].
    ///
    /// Any failure — malformed structure, bad base64, signature mismatch, expired (`exp <= now_ms`),
    /// or undecodable claims — maps to [`AuthError::BadToken`]. Never panics on attacker input.
    pub fn verify(&self, token: &AuthToken, now_ms: i64) -> Result<Principal, AuthError> {
        let (payload, sig_b64) = token.0.split_once('.').ok_or(AuthError::BadToken)?;

        // Recompute the MAC over the payload and compare in constant time.
        let presented = URL_SAFE_NO_PAD
            .decode(sig_b64)
            .map_err(|_| AuthError::BadToken)?;
        let mut mac =
            HmacSha256::new_from_slice(&self.key).expect("HMAC accepts a key of any length");
        mac.update(payload.as_bytes());
        mac.verify_slice(&presented).map_err(|_| AuthError::BadToken)?;

        // Signature is good: decode the claims.
        let json = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| AuthError::BadToken)?;
        let claims: Claims = serde_json::from_slice(&json).map_err(|_| AuthError::BadToken)?;

        if claims.exp <= now_ms {
            return Err(AuthError::BadToken);
        }

        Ok(Principal {
            household: HouseholdHandle(claims.household),
            user: UserId(claims.user),
            role: claims.role,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal() -> Principal {
        Principal {
            household: HouseholdHandle("house1".into()),
            user: UserId(42),
            role: Role::Knight,
        }
    }

    #[test]
    fn issue_then_verify_round_trips() {
        let signer = TokenSigner::new(b"server-key");
        let tok = signer.issue(&principal(), 1_000, 60_000);
        let p = signer.verify(&tok, 1_000).expect("fresh token verifies");
        assert_eq!(p, principal());
    }

    #[test]
    fn flipped_byte_is_bad_token() {
        let signer = TokenSigner::new(b"server-key");
        let tok = signer.issue(&principal(), 1_000, 60_000);
        // Flip the first char of the payload.
        let mut bytes: Vec<u8> = tok.0.into_bytes();
        bytes[0] ^= 0x01;
        let tampered = AuthToken(String::from_utf8(bytes).expect("ascii"));
        assert_eq!(signer.verify(&tampered, 1_000), Err(AuthError::BadToken));
    }

    #[test]
    fn different_key_is_bad_token() {
        let signer = TokenSigner::new(b"server-key");
        let tok = signer.issue(&principal(), 1_000, 60_000);
        let other = TokenSigner::new(b"different-key");
        assert_eq!(other.verify(&tok, 1_000), Err(AuthError::BadToken));
    }

    #[test]
    fn expired_token_is_bad_token() {
        let signer = TokenSigner::new(b"server-key");
        let tok = signer.issue(&principal(), 1_000, 1_000); // exp = 2_000
        assert_eq!(signer.verify(&tok, 2_000), Err(AuthError::BadToken)); // now == exp
        assert_eq!(signer.verify(&tok, 5_000), Err(AuthError::BadToken)); // now past exp
        assert!(signer.verify(&tok, 1_999).is_ok()); // still valid
    }

    #[test]
    fn garbage_string_is_bad_token() {
        let signer = TokenSigner::new(b"server-key");
        assert_eq!(
            signer.verify(&AuthToken("garbage".into()), 1_000),
            Err(AuthError::BadToken)
        );
        assert_eq!(
            signer.verify(&AuthToken("a.b.c".into()), 1_000),
            Err(AuthError::BadToken)
        );
    }
}
