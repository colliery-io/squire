//! The OpenAPI 3 document for the Local API — emitted **from the Rust types** (SQUIRE-T-0031 /
//! ADR SQUIRE-A-0009).
//!
//! [`ApiDoc`] aggregates the `#[utoipa::path]`-annotated handlers (Squire, Knight, control-plane)
//! and the `ToSchema`-deriving wire DTOs from `domain-core::contract` (plus the api's own request
//! envelopes). The document is the language-neutral contract that the phone-client Kotlin SDK is
//! generated from; a committed `openapi.json` is the frozen source of truth, guarded by the
//! conformance test in `tests/openapi.rs`.
//!
//! ## Auth
//! Every authenticated route carries a `bearer_auth` HTTP-bearer security requirement plus an
//! `X-Household` header param (documented per-path). The scheme is registered below via a
//! [`Modify`] addon, since `#[derive(OpenApi)]` does not declare security schemes inline.

use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

/// Registers the `bearer_auth` HTTP-bearer security scheme referenced by the annotated paths.
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi
            .components
            .as_mut()
            .expect("OpenApi derive always produces a components object");
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .description(Some(
                        "Tenant-scoped bearer token (proves household, user, role).",
                    ))
                    .build(),
            ),
        );
    }
}

/// The aggregate OpenAPI document for the Local API.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Squire Local API",
        description = "The LAN HTTP wire contract for the Squire household reward system — emitted from the Rust types (ADR SQUIRE-A-0009). IDs are int64 on the wire (a 'fits in i64' invariant).",
        version = "0.1.0",
        // A non-empty license keeps the emitted spec valid for OpenAPI 3.1 tooling
        // (openapi-generator rejects the empty license utoipa otherwise infers from Cargo metadata).
        license(name = "Proprietary", identifier = "LicenseRef-Proprietary"),
    ),
    paths(
        // Control plane (SQUIRE-T-0017)
        crate::control::register,
        crate::control::login,
        crate::control::add_member,
        // Squire-role surface (SQUIRE-T-0015)
        crate::squire::get_state,
        crate::squire::submit_claim,
        crate::squire::request_redemption,
        // Knight-role surface (SQUIRE-T-0016)
        crate::knight::review_claim,
        crate::knight::review_redemption,
        crate::knight::redeem,
        crate::knight::adjust,
        crate::knight::mark_done,
        crate::knight::household_review,
    ),
    components(schemas(
        // ── Squire state view + nested ──
        domain_core::contract::StateView,
        domain_core::contract::QuestCard,
        domain_core::contract::QuestStatus,
        domain_core::contract::StreakView,
        domain_core::contract::RewardCard,
        domain_core::contract::LockReason,
        domain_core::contract::ClaimStatus,
        domain_core::contract::ClaimState,
        domain_core::contract::RedemptionStatus,
        domain_core::contract::RedemptionState,
        // ── Squire request/response envelopes ──
        domain_core::contract::SubmitClaimReq,
        domain_core::contract::SubmitClaimResp,
        domain_core::contract::RequestRedemptionReq,
        domain_core::contract::RequestRedemptionResp,
        // ── Knight review read ──
        domain_core::contract::HouseholdReview,
        domain_core::contract::SquireSummary,
        domain_core::contract::PendingClaim,
        domain_core::contract::PendingRequest,
        domain_core::contract::ItemOption,
        domain_core::contract::QuestOption,
        // ── Identity / control-plane DTOs ──
        domain_core::contract::RegisterHouseholdReq,
        domain_core::contract::RegisterHouseholdResp,
        domain_core::contract::LoginReq,
        domain_core::contract::LoginResp,
        domain_core::contract::AddMemberReq,
        domain_core::contract::AddMemberResp,
        domain_core::contract::HouseholdHandle,
        domain_core::contract::AuthToken,
        // ── id / enum primitives ──
        domain_core::contract::UserId,
        domain_core::contract::QuestId,
        domain_core::contract::ItemId,
        domain_core::contract::AchievementId,
        domain_core::contract::ClaimId,
        domain_core::contract::RequestId,
        domain_core::contract::CommandId,
        domain_core::contract::Role,
        domain_core::contract::Date,
        domain_core::contract::Timestamp,
        domain_core::contract::Category,
        // ── api's own Knight request envelopes ──
        crate::knight::ReviewClaimReq,
        crate::knight::ReviewRedemptionReq,
        crate::knight::RedeemReq,
        crate::knight::AdjustReq,
        crate::knight::MarkDoneReq,
        crate::knight::Ack,
        crate::knight::DecisionDto,
        crate::knight::DecisionKind,
    )),
    modifiers(&SecurityAddon),
    tags(
        (name = "squire", description = "Squire-role endpoints (the child phone)."),
        (name = "knight", description = "Knight-role privileged endpoints (the parent phone)."),
        (name = "control", description = "Control plane: registration, login, member management."),
    ),
)]
pub struct ApiDoc;

/// The aggregate OpenAPI 3 document for the Local API — the spec emitted from the Rust types.
pub fn openapi_doc() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}
