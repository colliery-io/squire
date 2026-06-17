use super::*;

// ─── IDENTITY (mutable table, per tenant) ───────────────────────────────────

/// A household member as the domain sees them — no credentials (those live in the
/// identity/auth layer, never in the shared domain). Deactivated, not deleted, so
/// historical events keep referring to a valid user.
#[derive(Clone, Debug)]
pub struct User {
    pub id: UserId,
    pub role: Role,
    pub display_name: String,
    pub active: bool,
}
