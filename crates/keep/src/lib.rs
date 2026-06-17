//! # Squire `keep` — the parent/admin app (SQUIRE-S-0004)
//!
//! The Keep is the household's **only writer** and the **full** privileged control surface. Per
//! ADR [SQUIRE-A-0008] it is an **embedded, loopback-only web application**: the UI (HTML/CSS/JS)
//! is baked into this binary, the admin HTTP server binds to **127.0.0.1 only**, and every handler
//! drives the Domain Core **in-process** — snapshot → [`Engine::handle`] → [`Repository::apply`]
//! over the single-writer store. It never routes operations through the network `api` crate (it
//! does not even depend on it), so authoring is never network-reachable (AR-8 / FR-ADM4).
//!
//! This task (SQUIRE-T-0025) ships the spine: the [`KeepState`] command seam ([`KeepState::commit`]),
//! the loopback web server, embedded assets, and operator (Knight) login. Authoring, the review
//! queue, member admin, and the log inspector land in T-0026..T-0030 on top of this.
//!
//! ## Operator identity
//!
//! The Keep is operated by a **Knight**. The operator signs in via the [`identity`] component
//! (`POST /login`), which sets an HttpOnly `keep_session` cookie carrying the identity token. The
//! [`Operator`] extractor verifies that token (cookie or `Authorization: Bearer`) on every
//! protected request and yields the acting Knight's [`Principal`], which handlers stamp as `by` /
//! `actor` for audit (A-0005, A-0007). Unauthenticated admin actions are refused.
//!
//! [SQUIRE-A-0008]: the Keep admin-UI ADR.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::{FromRequestParts, Path, State};
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE, SET_COOKIE};
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use serde::{Deserialize, Serialize};

use domain_core::contract::{
    AuthToken, Change, Command, DomainError, Engine, HouseholdHandle, LoginReq, RegisterHouseholdReq,
    Repository, Role, Snapshot, UserId,
};
use domain_core::DomainEngine;
use store::tenant::{Backend, ProvisionError, Provisioner};
use store::SystemClock;

use identity::{Identity, Principal, ProdIdentity, SharedStore, TokenSigner};

pub mod achievements;
pub mod inspector;
pub mod items;
pub mod members;
pub mod pair;
pub mod quests;
pub mod review;

/// Embedded web UI assets (`assets/`), baked into the binary so the Keep is one self-contained
/// artifact (ADR A-0008 — no separate asset host, no node build step).
#[derive(rust_embed::RustEmbed)]
#[folder = "assets/"]
struct Assets;

/// Everything a Keep handler needs: the single-writer store, the pure engine, the clock, the
/// identity port (operator auth), and the bound local-tenant household handle.
///
/// The Keep is the **single writer**: all writes — authoring, reviews, redemptions, adjustments,
/// and the identity's member writes — go through this one `Arc<Mutex<Store>>` (consistent with the
/// api's single-writer model). `commit` is the one place command handling touches the store.
pub struct KeepState {
    /// The tenant store, shared with the identity port (single writer behind a `Mutex`).
    pub store: SharedStore,
    /// The pure, stateless domain engine (the one validated entry point).
    pub engine: DomainEngine,
    /// Wall clock, passed to [`Engine::handle`] and used for audit timestamps.
    pub clock: SystemClock,
    /// The authentication / membership seam — used for operator login and token verification.
    pub identity: Arc<dyn Identity>,
    /// The local single-tenant household the operator signs into.
    pub household: HouseholdHandle,
}

impl KeepState {
    /// Wire a Keep over a single local tenant (the LAN-local MVP posture, mirroring
    /// `api::AppState::local_prod`): provision + migrate the tenant for `handle` over `backend`,
    /// open the one shared store, and build a `ProdIdentity` (single-writer) bound to the same
    /// handle. The store the handlers write and the store the identity seeds are the SAME
    /// `SharedStore`, so every writer serializes on one lock/connection (AR-1).
    pub fn local(
        backend: Backend,
        handle: HouseholdHandle,
        signer: TokenSigner,
        token_ttl_ms: i64,
    ) -> Result<Arc<Self>, ProvisionError> {
        let provisioner = Provisioner::new(backend);
        provisioner.provision(&handle.0)?;
        let store = provisioner.open(&handle.0, SystemClock)?;
        let store: SharedStore = Arc::new(Mutex::new(store));
        let identity = ProdIdentity::shared_local(store.clone(), signer, handle.clone(), token_ttl_ms);
        Ok(Arc::new(Self {
            store,
            engine: DomainEngine,
            clock: SystemClock,
            identity: Arc::new(identity),
            household: handle,
        }))
    }

    /// Assemble a Keep from already-wired parts (used by tests that build their own identity).
    pub fn from_parts(
        store: SharedStore,
        identity: Arc<dyn Identity>,
        household: HouseholdHandle,
    ) -> Arc<Self> {
        Arc::new(Self { store, engine: DomainEngine, clock: SystemClock, identity, household })
    }

    /// **The engine-direct command seam.** Lock the store, snapshot, run `cmd` through the engine,
    /// and on success apply the resulting changes under the same single writer. `by` stamps
    /// definition-edit audit columns (the acting Knight; `None` for system/seed). Returns the
    /// engine's `Vec<Change>` (empty on an idempotent no-op) or the [`DomainError`] it raised.
    ///
    /// This NEVER touches the network `api` — it is a direct in-process engine call (A-0008).
    pub fn commit(&self, by: Option<UserId>, cmd: Command) -> Result<Vec<Change>, DomainError> {
        let mut store = self.store.lock().expect("store mutex poisoned");
        let snap = store.snapshot();
        let changes = self.engine.handle(&snap, cmd, &self.clock)?;
        store
            .apply(by, &changes)
            .expect("apply: single-writer store write failed");
        Ok(changes)
    }

    /// A fresh snapshot under the store lock (reads are pure over it afterwards).
    pub fn snapshot(&self) -> Snapshot {
        self.store.lock().expect("store mutex poisoned").snapshot()
    }

    /// Apply `changes` directly through the single writer, stamping `by`. For member administration
    /// (`PutUser` / `SetUserActive`) which — like the identity component's member writes — is a
    /// direct store write, NOT an engine `Command`. Authoring/claim/redemption commands go through
    /// [`commit`](Self::commit) instead.
    pub fn apply_changes(
        &self,
        by: Option<UserId>,
        changes: &[Change],
    ) -> Result<(), domain_core::contract::RepoError> {
        self.store.lock().expect("store mutex poisoned").apply(by, changes)
    }
}

/// Map a [`DomainError`] onto an HTTP status — the Keep's engine-direct mirror of the api's
/// mapping, so the local UI surfaces the same actionable 4xx/409 instead of opaque 500s.
pub fn domain_status(err: DomainError) -> StatusCode {
    match err {
        DomainError::NotAssigned
        | DomainError::OccurrenceTaken
        | DomainError::BadCommandForActor => StatusCode::FORBIDDEN,
        DomainError::QuestNotFound
        | DomainError::ItemNotFound
        | DomainError::AchievementNotFound
        | DomainError::ClaimNotFound
        | DomainError::RequestNotFound
        | DomainError::UserNotFound => StatusCode::NOT_FOUND,
        DomainError::AlreadyClaimedToday | DomainError::AlreadyReviewed => StatusCode::CONFLICT,
        DomainError::Redeem(_) => StatusCode::CONFLICT,
        DomainError::NotASquire | DomainError::InvalidDefinition => StatusCode::BAD_REQUEST,
        DomainError::Inactive => StatusCode::BAD_REQUEST,
    }
}

// ─── operator auth ───────────────────────────────────────────────────────────────────────────

/// The name of the HttpOnly session cookie carrying the operator's identity token.
const SESSION_COOKIE: &str = "keep_session";

/// A verified operator: the acting **Knight** behind a protected request. Produced by reading the
/// session cookie (or `Authorization: Bearer`) and verifying it via the [`identity`] port against
/// the Keep's bound household; a non-Knight is refused (only Knights operate the Keep).
pub struct Operator(pub Principal);

impl FromRequestParts<Arc<KeepState>> for Operator {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<KeepState>,
    ) -> Result<Self, Self::Rejection> {
        let token = token_from_headers(&parts.headers).ok_or(StatusCode::UNAUTHORIZED)?;
        let principal = state
            .identity
            .verify(&state.household, &AuthToken(token))
            .map_err(|_| StatusCode::UNAUTHORIZED)?;
        if principal.role != Role::Knight {
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(Operator(principal))
    }
}

/// Pull the operator token from `Authorization: Bearer <tok>` or the `keep_session` cookie.
fn token_from_headers(headers: &HeaderMap) -> Option<String> {
    if let Some(auth) = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        if let Some(tok) = auth.strip_prefix("Bearer ").or_else(|| auth.strip_prefix("bearer ")) {
            let tok = tok.trim();
            if !tok.is_empty() {
                return Some(tok.to_string());
            }
        }
    }
    // Cookie header: `keep_session=<tok>` among `; `-separated pairs.
    let cookies = headers.get(axum::http::header::COOKIE).and_then(|v| v.to_str().ok())?;
    cookies.split(';').find_map(|pair| {
        let (k, v) = pair.trim().split_once('=')?;
        (k == SESSION_COOKIE && !v.is_empty()).then(|| v.to_string())
    })
}

// ─── handlers ──────────────────────────────────────────────────────────────────────────────────

/// `GET /health` — liveness probe (no auth, no state). 200 `"ok"`.
async fn health() -> &'static str {
    "ok"
}

/// `POST /login` — operator (Knight) sign-in. Exchanges `(user, secret)` for an identity token via
/// the [`identity`] port against the bound household, sets the `keep_session` cookie, and returns
/// the acting Knight. A bad secret is 401; a non-Knight is 403 (only Knights operate the Keep).
async fn login(
    State(state): State<Arc<KeepState>>,
    Form(form): Form<LoginForm>,
) -> Result<Response, StatusCode> {
    // `user` arrives as a string: `serde_urlencoded` does not support `u128`, and a `UserId` is a
    // u128, so we take it as text and parse here (a non-numeric id is a 400).
    let user = UserId(form.user.trim().parse().map_err(|_| StatusCode::BAD_REQUEST)?);
    let resp = state
        .identity
        .login(LoginReq { household: state.household.clone(), user, secret: form.secret })
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    if resp.role != Role::Knight {
        return Err(StatusCode::FORBIDDEN);
    }
    let display_name = state
        .snapshot()
        .users
        .iter()
        .find(|u| u.id == user)
        .map(|u| u.display_name.clone())
        .unwrap_or_default();

    let body = Json(Whoami { user: user.0, role: resp.role, display_name });
    let mut response = body.into_response();
    // HttpOnly + SameSite=Strict + Path=/; the Keep is loopback-only so this is a same-machine
    // session, but we still keep the cookie out of JS and off cross-site requests.
    let cookie = format!(
        "{SESSION_COOKIE}={}; HttpOnly; SameSite=Strict; Path=/",
        resp.token.0
    );
    response.headers_mut().insert(
        SET_COOKIE,
        cookie.parse().expect("session cookie is valid header value"),
    );
    Ok(response)
}

/// `POST /register` — **first-run bootstrap**: create the household's first Knight (the admin) when
/// none exists yet, then log the operator in (sets the `keep_session` cookie). Delegates to the
/// [`identity`] port, which refuses (`Forbidden` → 403) once a Knight already exists, so this is
/// safe to leave exposed on the loopback admin surface. A fresh Keep has no members and no way to
/// sign in otherwise; this is how the operator creates the first one.
async fn register(
    State(state): State<Arc<KeepState>>,
    Form(form): Form<RegisterForm>,
) -> Result<Response, StatusCode> {
    let resp = state
        .identity
        .register(RegisterHouseholdReq {
            household_name: state.household.0.clone(),
            admin_name: form.admin_name,
            admin_secret: form.admin_secret,
        })
        .map_err(|e| match e {
            identity::AuthError::Forbidden => StatusCode::CONFLICT, // already bootstrapped
            _ => StatusCode::BAD_REQUEST,
        })?;

    let body = Json(Whoami { user: resp.admin.0, role: Role::Knight, display_name: String::new() });
    let mut response = body.into_response();
    let cookie = format!("{SESSION_COOKIE}={}; HttpOnly; SameSite=Strict; Path=/", resp.token.0);
    response
        .headers_mut()
        .insert(SET_COOKIE, cookie.parse().expect("session cookie is valid header value"));
    Ok(response)
}

/// `GET /api/whoami` — echoes the verified acting Knight (proves the operator session works and is
/// the principal handlers will stamp). Refused (401/403) without a valid Knight session.
async fn whoami(State(state): State<Arc<KeepState>>, Operator(principal): Operator) -> Json<Whoami> {
    let display_name = state
        .snapshot()
        .users
        .iter()
        .find(|u| u.id == principal.user)
        .map(|u| u.display_name.clone())
        .unwrap_or_default();
    Json(Whoami { user: principal.user.0, role: principal.role, display_name })
}

/// `GET /` — the embedded app shell.
async fn index() -> Response {
    serve_asset("index.html")
}

/// `GET /static/{*path}` — an embedded static asset (css/js/…), or 404.
async fn static_asset(Path(path): Path<String>) -> Response {
    serve_asset(&path)
}

/// Serve an embedded asset by path, guessing a content type from its extension.
fn serve_asset(path: &str) -> Response {
    match Assets::get(path) {
        Some(content) => {
            let body = axum::body::Body::from(content.data.into_owned());
            ([(CONTENT_TYPE, content_type_for(path))], body).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Minimal extension → MIME mapping for the handful of asset kinds the Keep ships (avoids a
/// mime-guess dependency).
fn content_type_for(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, ext)| ext) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

// ─── DTOs ──────────────────────────────────────────────────────────────────────────────────────

/// `POST /login` form body (`application/x-www-form-urlencoded`).
#[derive(Debug, Deserialize)]
struct LoginForm {
    /// The Knight's `UserId` as a decimal string (parsed to `u128`; `serde_urlencoded` does not
    /// support `u128` directly).
    user: String,
    secret: String,
}

/// `POST /register` form body — the first-run admin Knight's name + secret.
#[derive(Debug, Deserialize)]
struct RegisterForm {
    admin_name: String,
    admin_secret: String,
}

/// The acting Knight echoed by `/login` and `/api/whoami`.
#[derive(Debug, Serialize)]
struct Whoami {
    user: u128,
    role: Role,
    display_name: String,
}

// ─── router + serve ──────────────────────────────────────────────────────────────────────────

/// Build the Keep's admin [`Router`]. Mounted on a loopback listener by [`serve`]; tests drive it
/// via `tower::ServiceExt::oneshot`. Later Keep tasks add authoring / review / member / log routes.
pub fn router(state: Arc<KeepState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/", get(index))
        .route("/static/{*path}", get(static_asset))
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/api/whoami", get(whoami))
        // ── Authoring: quests (SQUIRE-T-0026) ──────────────────────────────────
        .route("/api/quests", get(quests::list_quests).post(quests::create_quest))
        .route("/api/quests/{id}/archive", post(quests::archive_quest))
        // ── Authoring: items + achievements (SQUIRE-T-0027) ─────────────────────
        .route("/api/items", get(items::list_items).post(items::create_item))
        .route("/api/items/{id}/archive", post(items::archive_item))
        .route(
            "/api/achievements",
            get(achievements::list_achievements).post(achievements::create_achievement),
        )
        .route("/api/achievements/{id}/archive", post(achievements::archive_achievement))
        // ── Member administration (SQUIRE-T-0028) ──────────────────────────────
        .route("/api/members", get(members::list_members).post(members::add_member))
        .route("/api/members/{id}/active", post(members::set_active))
        // Device pairing (ADR A-0010): mint a one-time code + QR for a chosen member.
        .route("/api/pair/codes", post(pair::mint_pair_code))
        // ── Cross-Squire review queue + redeem + adjust (SQUIRE-T-0029) ─────────
        .route("/api/review", get(review::get_review))
        .route("/api/review/claim", post(review::review_claim))
        .route("/api/review/redemption", post(review::review_redemption))
        .route("/api/redeem", post(review::redeem))
        .route("/api/adjust", post(review::adjust))
        // ── Read-only event-log inspector (SQUIRE-T-0030) ──────────────────────
        .route("/api/log/quest/{id}", get(inspector::quest_log))
        .route("/api/log/item/{id}", get(inspector::item_log))
        .with_state(state)
}

/// The Keep's admin bind address: **loopback only** (`127.0.0.1:port`). Authoring is delivered
/// over HTTP, so binding to loopback (never `0.0.0.0` / the LAN) is the load-bearing control that
/// keeps it off the network (ADR A-0008 / AR-8).
pub fn admin_addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

/// Bind the **loopback** admin address and serve the Keep until the process exits.
pub async fn serve(
    state: Arc<KeepState>,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = admin_addr(port);
    debug_assert!(addr.ip().is_loopback(), "the Keep must bind loopback only");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router(state)).await?;
    Ok(())
}
