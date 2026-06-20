//! The three **Squire-role** endpoints (SQUIRE-T-0015): `GET /state`, `POST /claims`,
//! `POST /redemption-requests`, plus the per-Squire [`StateView`] assembly.
//!
//! A Squire only ever sees and acts on its OWN data: every handler reads `principal.user`
//! from the verified token and never trusts a query parameter for identity (the wire request
//! DTOs intentionally omit `squire`; the API fills it from the token when building the
//! `Command`). All reads are pure over a single `store.snapshot()` taken under the store
//! mutex, after which the lock is dropped and the (lock-free) projections run.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use domain_core::contract::{
    Achievement, Change, Command, Criterion, Date, DomainError, Event, Points, Quest, Scope, Snapshot,
    Timestamp, UserId,
};
use domain_core::contract::{
    AdjustmentView, ClaimState, ClaimStatus, GoalView, QuestCard, RedemptionState,
    RedemptionStatus, RequestRedemptionReq, RequestRedemptionResp, RewardCard, StateView,
    StreakView, SubmitClaimReq, SubmitClaimResp,
};
use domain_core::contract::{Clock, Engine, Projections, Repository};
use domain_core::{quest_status, reward_view, streak_view, Proj};

use crate::auth::RequireSquire;
use crate::state::AppState;

/// `GET /state` (RequireSquire) — assemble the authenticated Squire's full [`StateView`] from a
/// single snapshot. Returns 200 JSON; the Squire is always `principal.user` (token-derived).
#[utoipa::path(
    get,
    path = "/state",
    tag = "squire",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
    ),
    responses(
        (status = 200, description = "The authenticated Squire's full state view", body = StateView),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Squire"),
    ),
)]
pub async fn get_state(
    State(state): State<Arc<AppState>>,
    RequireSquire(principal): RequireSquire,
) -> Json<StateView> {
    let snap = {
        let store = state.store.lock().expect("store mutex poisoned");
        store.snapshot()
    };
    let squire = principal.user;
    let today = state.clock.today();
    let now = state.clock.now();
    Json(assemble_state(&snap, squire, today, now))
}

/// `POST /claims` (RequireSquire) — submit a completion claim for the authenticated Squire.
/// Idempotent on the phone-minted `claim_id` (a replay returns the current state). Returns the
/// resulting [`ClaimState`] (`Pending`, or `Approved { points }` if the quest auto-approves).
#[utoipa::path(
    post,
    path = "/claims",
    tag = "squire",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
    ),
    request_body = SubmitClaimReq,
    responses(
        (status = 200, description = "The claim's resulting state", body = SubmitClaimResp),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Squire, or not allowed to act on this occurrence"),
        (status = 404, description = "Quest not found"),
        (status = 409, description = "Already claimed today"),
    ),
)]
pub async fn submit_claim(
    State(state): State<Arc<AppState>>,
    RequireSquire(principal): RequireSquire,
    Json(req): Json<SubmitClaimReq>,
) -> Result<Json<SubmitClaimResp>, StatusCode> {
    let squire = principal.user;
    let cmd = Command::SubmitClaim {
        claim_id: req.claim_id,
        squire,
        quest_id: req.quest_id,
        on: req.on,
    };
    // Child submission: `by` is None — the audit `by` is for authoring; the appended events
    // carry their own `squire`/`actor`. A replay yields an empty change set (no-op).
    handle_command(&state, None, cmd).map_err(domain_status)?;

    // Re-read after applying and report the claim's current state (covers both the fresh
    // submit and the idempotent replay).
    let snap = {
        let store = state.store.lock().expect("store mutex poisoned");
        store.snapshot()
    };
    let state_ = claim_state(&snap, req.claim_id);
    Ok(Json(SubmitClaimResp { claim_id: req.claim_id, state: state_ }))
}

/// `POST /redemption-requests` (RequireSquire) — record a redemption request for the
/// authenticated Squire. No affordability check (it is re-checked at approval time). Idempotent
/// on the phone-minted `request_id`; always reports `Pending` (a fresh request is never resolved
/// in the same call, and an idempotent replay returns the still-pending request).
#[utoipa::path(
    post,
    path = "/redemption-requests",
    tag = "squire",
    security(("bearer_auth" = [])),
    params(
        ("X-Household" = String, Header, description = "Opaque household handle routing the request to its tenant"),
    ),
    request_body = RequestRedemptionReq,
    responses(
        (status = 200, description = "The redemption request's state (always Pending)", body = RequestRedemptionResp),
        (status = 401, description = "Missing or invalid credentials"),
        (status = 403, description = "Authenticated but not a Squire"),
        (status = 404, description = "Item not found"),
    ),
)]
pub async fn request_redemption(
    State(state): State<Arc<AppState>>,
    RequireSquire(principal): RequireSquire,
    Json(req): Json<RequestRedemptionReq>,
) -> Result<Json<RequestRedemptionResp>, StatusCode> {
    let squire = principal.user;
    let cmd = Command::RequestRedemption {
        request_id: req.request_id,
        squire,
        item_id: req.item_id,
    };
    handle_command(&state, None, cmd).map_err(domain_status)?;
    Ok(Json(RequestRedemptionResp {
        request_id: req.request_id,
        state: RedemptionState::pending(),
    }))
}

// ─── command helper (reused by T-0016) ──────────────────────────────────────

/// Lock the store, snapshot, run the command through the engine, and on success apply the
/// resulting changes under the same writer. `by` stamps definition-edit audit columns (`None`
/// for child submissions / system). Returns the changes the engine emitted (empty on an
/// idempotent no-op) or the [`DomainError`] the engine raised.
///
/// Shared by all the privileged endpoints too (T-0016) — the single place command handling
/// touches the store.
pub fn handle_command(
    state: &AppState,
    by: Option<UserId>,
    cmd: Command,
) -> Result<Vec<Change>, DomainError> {
    let mut store = state.store.lock().expect("store mutex poisoned");
    let snap = store.snapshot();
    let changes = state.engine.handle(&snap, cmd, &state.clock)?;
    store
        .apply(by, &changes)
        .expect("apply: single-writer store write failed");
    Ok(changes)
}

/// Map a [`DomainError`] onto an HTTP status, shared by the Squire and Knight (T-0016) surfaces.
/// Caller-correctable conditions are 4xx; a redemption blocked at commit time (insufficient
/// points / achievement-locked / out of stock) is a 409 conflict against current state.
///
/// Note on `OccurrenceTaken`: over the Squire surface a Race occurrence already won by a sibling
/// is "not yours to take" → 403; the Knight's direct mark-done can also hit it (a genuine
/// conflict). We keep the single mapping (403) — the Knight rarely races a sibling, and 403 still
/// signals "this occurrence is closed to you".
pub(crate) fn domain_status(err: DomainError) -> StatusCode {
    match err {
        // The Squire isn't allowed to act on this subject / occurrence.
        DomainError::NotAssigned
        | DomainError::OccurrenceTaken
        | DomainError::BadCommandForActor => StatusCode::FORBIDDEN,
        // The referenced definition / subject doesn't exist.
        DomainError::QuestNotFound
        | DomainError::ItemNotFound
        | DomainError::AchievementNotFound
        | DomainError::ClaimNotFound
        | DomainError::RequestNotFound
        | DomainError::UserNotFound => StatusCode::NOT_FOUND,
        // A duplicate live claim / a second review of an already-resolved claim — the prior
        // resolution stands (a genuine double-action, not an idempotent replay).
        DomainError::AlreadyClaimedToday | DomainError::AlreadyReviewed => StatusCode::CONFLICT,
        // A redeem blocked by current state (can't afford / locked / out of stock): a conflict.
        DomainError::Redeem(_) => StatusCode::CONFLICT,
        // A malformed subject role / malformed authoring input → 400.
        DomainError::NotASquire | DomainError::InvalidDefinition => StatusCode::BAD_REQUEST,
        // Archived / unavailable subject.
        DomainError::Inactive => StatusCode::BAD_REQUEST,
    }
}

// ─── StateView assembly (pure over one snapshot) ─────────────────────────────

/// Build the per-Squire [`StateView`] from one snapshot. All reads are pure projections over
/// `snap`; `today`/`now` come from the clock.
pub(crate) fn assemble_state(
    snap: &Snapshot,
    squire: UserId,
    today: Date,
    now: Timestamp,
) -> StateView {
    let quests_today = quests_today(snap, squire, today);
    let balance = clamp_balance(Proj::balance(snap, squire));
    let streaks = streaks(snap, squire, today);
    let badges = badges(snap, squire);
    let goals = goals(snap, squire);
    let rewards = rewards(snap, squire);
    let my_claims = my_claims(snap, squire);
    let my_requests = my_requests(snap, squire);
    let adjustments = adjustments(snap, squire);

    StateView {
        squire,
        generated_at: now,
        balance,
        quests_today,
        streaks,
        badges,
        goals,
        rewards,
        my_claims,
        my_requests,
        adjustments,
    }
}

/// The Squire's recent point adjustments (newest first, capped) — grants and hazard penalties a
/// Knight applied via `AdjustPoints`, surfaced so the child sees coins gained/lost and *why*
/// (SQUIRE-T-0094 / SQUIRE-T-0096).
fn adjustments(snap: &Snapshot, squire: UserId) -> Vec<AdjustmentView> {
    snap.events
        .iter()
        .rev()
        .filter_map(|e| match e {
            Event::PointsAdjusted { squire: s, amount, reason, at, .. } if *s == squire => {
                Some(AdjustmentView { amount: *amount, reason: reason.clone(), at: *at })
            }
            _ => None,
        })
        .take(20)
        .collect()
}

/// Active achievements this Squire has **not yet earned** — "goals to unlock" on the child home
/// (SQUIRE-T-0094 #3). Streak achievements are shown live in the Streaks section, so they're excluded
/// here to avoid duplication; this surfaces the rest (TotalCompletions / PointsEarned) that would
/// otherwise be invisible until earned.
fn goals(snap: &Snapshot, squire: UserId) -> Vec<GoalView> {
    let earned = |aid| {
        snap.events.iter().any(|e| {
            matches!(e, Event::AchievementUnlocked { squire: s, id, .. } if *s == squire && *id == aid)
        })
    };
    snap.achievements
        .iter()
        .filter(|a| a.active)
        .filter(|a| !matches!(a.criterion, Criterion::Streak { .. }))
        .filter(|a| !earned(a.id))
        .map(|a| GoalView {
            id: a.id,
            name: a.name.clone(),
            description: goal_description(snap, &a.criterion),
            bonus: a.bonus_points,
        })
        .collect()
}

/// Kid-friendly "how to earn it" for a non-streak [`Criterion`], with a scope suffix.
fn goal_description(snap: &Snapshot, criterion: &Criterion) -> String {
    fn scope_suffix(snap: &Snapshot, scope: &Scope) -> String {
        match scope {
            Scope::Any => String::new(),
            Scope::Category(cat) => format!(" in {}", cat.0),
            Scope::Quest(qid) => match snap.quests.iter().find(|q| q.id == *qid) {
                Some(q) => format!(" on “{}”", q.title),
                None => " on one quest".to_string(),
            },
        }
    }
    match criterion {
        Criterion::TotalCompletions { scope, count } => format!(
            "Complete {count} chore{}{}",
            if *count == 1 { "" } else { "s" },
            scope_suffix(snap, scope),
        ),
        Criterion::PointsEarned { total } => format!("Earn {total} coins"),
        // Streaks are surfaced separately; describe defensively just in case.
        Criterion::Streak { length, scope, .. } => {
            format!("{length}-day streak{}", scope_suffix(snap, scope))
        }
    }
}

/// The achievements this Squire has **earned** — one [`BadgeView`] per unlocked achievement, newest
/// first (SQUIRE-T-0079). Built from `AchievementUnlocked` events (deduped by achievement id, keeping
/// the most recent), with the name resolved from the current definitions.
fn badges(snap: &Snapshot, squire: UserId) -> Vec<domain_core::contract::BadgeView> {
    use domain_core::contract::{BadgeView, Event};
    let mut seen: std::collections::BTreeSet<u128> = std::collections::BTreeSet::new();
    let mut out: Vec<BadgeView> = Vec::new();
    // Walk newest → oldest so the first sighting of an id is the latest unlock.
    for e in snap.events.iter().rev() {
        if let Event::AchievementUnlocked { squire: s, id, bonus, at } = e {
            if *s == squire && seen.insert(id.0) {
                let name = snap
                    .achievements
                    .iter()
                    .find(|a| a.id == *id)
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| format!("#{}", id.0));
                out.push(BadgeView { id: *id, name, bonus: *bonus, at: *at });
            }
        }
    }
    out
}

/// `Proj::balance` returns an `i64` (only `PointsAdjusted` can take it negative); the display
/// balance clamps to `>= 0` and into [`Points`].
fn clamp_balance(raw: i64) -> Points {
    raw.max(0) as Points
}

/// The cards for every active quest the Squire is scheduled + assigned for "today" — including
/// pending/completed/taken ones (so the card can show status), not just the currently-claimable
/// ones. `quest_status` returns a relevant status iff the quest is on this Squire's "today" list
/// (it assumes the quest is scheduled on `on` and `squire` is an assignee); we gate inclusion on
/// the same scheduled-and-assigned predicate that `quests_due` uses, then label each card.
fn quests_today(snap: &Snapshot, squire: UserId, today: Date) -> Vec<QuestCard> {
    // `quests_due` gives the currently-claimable set (active + scheduled + assignee + not
    // already satisfied). A quest can also be on the list yet *not* due (pending / completed /
    // taken). To capture both, start from a scheduled+assigned predicate.
    snap.quests
        .iter()
        .filter(|q| q.active && quest_relevant_today(snap, squire, q, today))
        .map(|q| QuestCard {
            quest_id: q.id,
            title: q.title.clone(),
            reward: q.reward,
            category: q.category.clone(),
            icon: q.icon.clone(),
            on: today,
            status: quest_status(snap, squire, q, today),
        })
        .collect()
}

/// True when `quest` belongs on `squire`'s "today" list: scheduled on `today` AND the Squire is
/// an assignee. Either it is currently claimable (`quests_due`) OR it is scheduled+assigned but
/// already pending/completed/taken (so `quest_status` reports a non-`Available` status that we
/// still want to render).
fn quest_relevant_today(snap: &Snapshot, squire: UserId, quest: &Quest, today: Date) -> bool {
    if !is_assignee(snap, squire, quest) {
        return false;
    }
    let due = Proj::quests_due(snap, squire, today).contains(&quest.id);
    // Not due could mean "wrong day" OR "scheduled today but already acted on". Distinguish by
    // whether the Squire has a claim/completion for this quest today (i.e. the status is
    // Pending / CompletedToday / TakenByOther rather than simply off-schedule).
    due || squire_acted_today(snap, squire, quest.id, today)
}

/// Whether `squire` is an assignee of `quest` (`AllSquires`, or an explicit member).
fn is_assignee(snap: &Snapshot, squire: UserId, quest: &Quest) -> bool {
    use domain_core::contract::Assignment;
    match &quest.assignment {
        Assignment::AllSquires => snap
            .users
            .iter()
            .any(|u| u.id == squire && matches!(u.role, domain_core::contract::Role::Squire)),
        Assignment::Squires(set) => set.contains(&squire),
    }
}

/// Whether `squire` has already claimed (or had completed/taken) `quest` on `on` — i.e. there is
/// a claim of theirs for this occurrence, or (for a Race) the occurrence is closed. Used to keep
/// a scheduled-today quest on the list even after it leaves the `quests_due` set.
fn squire_acted_today(snap: &Snapshot, squire: UserId, quest_id: domain_core::contract::QuestId, on: Date) -> bool {
    // The Squire's own claim for this occurrence …
    let own_claim = snap.events.iter().any(|e| {
        matches!(
            e,
            Event::CompletionClaimed { squire: s, quest_id: q, on: d, .. }
                if *s == squire && *q == quest_id && *d == on
        )
    });
    // … or a Race occurrence closed by a sibling (so we render TakenByOther).
    let race_taken = snap.quests.iter().any(|q| {
        q.id == quest_id
            && q.completion == domain_core::contract::Completion::Race
            && snap.events.iter().any(|e| matches!(
                e,
                Event::CompletionApproved { claim_id, .. }
                    if claim_targets(snap, *claim_id, quest_id, on)
            ))
    });
    own_claim || race_taken
}

/// Whether `claim_id` is a claim for `(quest_id, on)` (any Squire) — resolves an approval back
/// to its quest/occurrence via the originating `CompletionClaimed`.
fn claim_targets(snap: &Snapshot, claim_id: domain_core::contract::ClaimId, quest_id: domain_core::contract::QuestId, on: Date) -> bool {
    snap.events.iter().any(|e| {
        matches!(
            e,
            Event::CompletionClaimed { claim_id: c, quest_id: q, on: d, .. }
                if *c == claim_id && *q == quest_id && *d == on
        )
    })
}

/// One [`StreakView`] per active streak-achievement scope relevant to the Squire — built from the
/// achievement's scope/basis via `streak_view`, named from the achievement.
fn streaks(snap: &Snapshot, squire: UserId, today: Date) -> Vec<StreakView> {
    snap.achievements
        .iter()
        .filter(|a| a.active)
        .filter_map(|a| streak_view_for(snap, squire, a, today))
        .collect()
}

/// Build a [`StreakView`] for one achievement iff its criterion is a `Streak`.
fn streak_view_for(snap: &Snapshot, squire: UserId, ach: &Achievement, today: Date) -> Option<StreakView> {
    match &ach.criterion {
        Criterion::Streak { scope, basis, .. } => {
            let (current, best, alive, next_milestone) =
                streak_view(snap, squire, scope, *basis, today);
            Some(StreakView {
                name: ach.name.clone(),
                current,
                best,
                alive,
                next_milestone,
            })
        }
        _ => None,
    }
}

/// A [`RewardCard`] for every active item, with this Squire's affordability / lock / last-redeemed
/// derived via `reward_view`.
fn rewards(snap: &Snapshot, squire: UserId) -> Vec<RewardCard> {
    snap.items
        .iter()
        .filter(|i| i.active)
        .map(|item| {
            let (affordable, lock, last_redeemed) = reward_view(snap, squire, item);
            RewardCard {
                item_id: item.id,
                name: item.name.clone(),
                cost: item.cost,
                icon: item.icon.clone(),
                affordable,
                lock,
                last_redeemed,
            }
        })
        .collect()
}

/// The Squire's own claims, each labelled with its quest title and current state. Derived from
/// `CompletionClaimed` events (theirs) joined to any approval/rejection.
fn my_claims(snap: &Snapshot, squire: UserId) -> Vec<ClaimStatus> {
    snap.events
        .iter()
        .filter_map(|e| match e {
            Event::CompletionClaimed { claim_id, squire: s, quest_id, on, .. } if *s == squire => {
                let quest_title = snap
                    .quests
                    .iter()
                    .find(|q| q.id == *quest_id)
                    .map(|q| q.title.clone())
                    .unwrap_or_default();
                Some(ClaimStatus {
                    claim_id: *claim_id,
                    quest_title,
                    on: *on,
                    state: claim_state(snap, *claim_id),
                })
            }
            _ => None,
        })
        .collect()
}

/// The resolution of one claim from the log: `Approved { points }` / `Rejected { reason }` /
/// `Pending` (the latter when no matching approval/rejection exists yet).
fn claim_state(snap: &Snapshot, claim_id: domain_core::contract::ClaimId) -> ClaimState {
    for e in &snap.events {
        match e {
            Event::CompletionApproved { claim_id: c, points, .. } if *c == claim_id => {
                return ClaimState::approved(*points);
            }
            Event::CompletionRejected { claim_id: c, reason, .. } if *c == claim_id => {
                return ClaimState::rejected(reason.clone());
            }
            _ => {}
        }
    }
    ClaimState::pending()
}

/// The Squire's own redemption requests, each labelled with item name / cost and current state.
fn my_requests(snap: &Snapshot, squire: UserId) -> Vec<RedemptionStatus> {
    snap.events
        .iter()
        .filter_map(|e| match e {
            Event::RedemptionRequested { request_id, squire: s, item_id, .. } if *s == squire => {
                let item = snap.items.iter().find(|i| i.id == *item_id);
                Some(RedemptionStatus {
                    request_id: *request_id,
                    item_name: item.map(|i| i.name.clone()).unwrap_or_default(),
                    cost: item.map(|i| i.cost).unwrap_or(0),
                    state: redemption_state(snap, *request_id),
                })
            }
            _ => None,
        })
        .collect()
}

/// The resolution of one redemption request: `Approved` (an `ItemRedeemed` carries the
/// `request_id`), `Rejected { reason }`, or `Pending`.
fn redemption_state(snap: &Snapshot, request_id: domain_core::contract::RequestId) -> RedemptionState {
    for e in &snap.events {
        match e {
            Event::ItemRedeemed { request_id: Some(r), .. } if *r == request_id => {
                return RedemptionState::approved();
            }
            Event::RedemptionRejected { request_id: r, reason, .. } if *r == request_id => {
                return RedemptionState::rejected(reason.clone());
            }
            _ => {}
        }
    }
    RedemptionState::pending()
}
