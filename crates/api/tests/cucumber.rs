//! Gherkin/Cucumber acceptance suite for the Local API (SQUIRE-I-0004 / SQUIRE-T-0110).
//!
//! Drives the axum `Router` via `tower::ServiceExt::oneshot` — the exact trust-boundary path as
//! `tests/integration.rs`, but expressed as Given/When/Then over `.feature` files. This is the
//! foundation suite: the shared scenario vocabulary the Keep (Playwright), Android, and full-stack
//! integration suites reuse. Runs as its own binary (`harness = false`); `angreal test gherkin`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use api::{router, AppState};
use cucumber::{given, then, when, World};

use axum::body::Body;
use axum::http::Request;
use domain_core::contract::{
    Assignment, Cadence, Change, Clock, Completion, Date, Quest, QuestId, Repository, Schedule,
};
use http_body_util::BodyExt;
use identity::DevIdentity;
use serde_json::{json, Value};
use store::tenant::{Backend, Provisioner};
use store::SystemClock;
use tower::ServiceExt;

/// A fresh app over an isolated temp-dir SQLite tenant (mirrors `integration::app`). Manual `Debug`
/// because `AppState`/`TempDir` aren't `Debug` and the cucumber `World` requires it.
struct TestApp {
    state: Arc<AppState>,
    /// Kept alive so the tenant SQLite dir survives; reused on `restart` to reopen the same data.
    dir: tempfile::TempDir,
    today: Date,
    /// Reused across a `restart` so issued tokens stay valid (`DevIdentity` holds credentials in
    /// memory). The durability being proven is the **store** — a fresh connection reading on-disk events.
    identity: Arc<dyn identity::Identity>,
}

impl std::fmt::Debug for TestApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TestApp(today={})", self.today.0)
    }
}

fn new_app() -> TestApp {
    let dir = tempfile::tempdir().expect("tempdir");
    let provisioner = Provisioner::new(Backend::Sqlite {
        dir: dir.path().to_path_buf(),
    });
    let store = provisioner
        .open("seed", SystemClock)
        .expect("open tenant store");
    let today = store.clock().today();
    let store = Arc::new(Mutex::new(store));
    let identity: Arc<dyn identity::Identity> = Arc::new(DevIdentity::new(store.clone()));
    let state = AppState::new(store.clone(), identity.clone());
    TestApp {
        state,
        dir,
        today,
        identity,
    }
}

/// The Cucumber world: a live app plus the tokens/results threaded across steps.
#[derive(Debug, Default, World)]
struct ApiWorld {
    app: Option<TestApp>,
    handle: String,
    knight: String,
    /// name → (user id, token)
    squires: HashMap<String, (u128, String)>,
    /// quest title → id (authored straight into the store, off the wire)
    quests: HashMap<String, u128>,
    last_claim_id: u128,
    last_request_id: u128,
    next_id: u128,
    last_status: u16,
    last_body: Value,
}

impl ApiWorld {
    fn app(&self) -> &Arc<AppState> {
        &self
            .app
            .as_ref()
            .expect("household not set up (missing a Given)")
            .state
    }

    #[allow(dead_code)] // used once claim scenarios land
    fn today(&self) -> i32 {
        self.app.as_ref().expect("household not set up").today.0
    }

    fn mint_id(&mut self) -> u128 {
        self.next_id += 1;
        9_000_000 + self.next_id
    }

    fn squire(&self, name: &str) -> (u128, String) {
        self.squires
            .get(name)
            .cloned()
            .unwrap_or_else(|| panic!("unknown squire {name}"))
    }

    /// Issue a request to the router and capture `(status, body)` into the world.
    async fn send(&mut self, method: &str, path: &str, token: Option<&str>, body: Option<Value>) {
        let mut b = Request::builder().method(method).uri(path);
        if let Some(t) = token {
            b = b
                .header("authorization", format!("Bearer {t}"))
                .header("x-household", &self.handle);
        }
        let req = match body {
            Some(j) => {
                b = b.header("content-type", "application/json");
                b.body(Body::from(j.to_string())).unwrap()
            }
            None => b.body(Body::empty()).unwrap(),
        };
        let resp = router(self.app().clone()).oneshot(req).await.unwrap();
        self.last_status = resp.status().as_u16();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        self.last_body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        };
    }

    /// `POST /admin/adjust` as `token`, capturing the result; `currency` included only when `Some`.
    async fn adjust(
        &mut self,
        token: String,
        squire: u128,
        amount: i64,
        reason: &str,
        currency: Option<&str>,
    ) {
        let cmd = self.mint_id();
        let mut body =
            json!({ "command_id": cmd, "squire": squire, "amount": amount, "reason": reason });
        if let Some(c) = currency {
            // `null` → JSON null (the SDK's encodeDefaults behaviour); a name → the string tag.
            body["currency"] = if c == "null" { Value::Null } else { json!(c) };
        }
        self.send("POST", "/admin/adjust", Some(&token), Some(body))
            .await;
    }

    /// Author a daily quest (assigned to all squires, not auto-approved → the review flow) straight
    /// into the store. Authoring is intentionally off the wire, so this is a direct `apply`.
    fn seed_quest(&mut self, title: &str, reward: u32) {
        let id = self.mint_id();
        let quest = Quest {
            id: QuestId(id),
            title: title.to_string(),
            description: None,
            category: None,
            reward,
            cash: 0,
            cadence: Cadence::Recurring(Schedule::Daily),
            assignment: Assignment::AllSquires,
            completion: Completion::EachAssignee,
            auto_approve: false,
            repeatable_within_day: false,
            active: true,
            icon: None,
        };
        self.app()
            .store
            .lock()
            .expect("store mutex poisoned")
            .apply(None, &[Change::PutQuest(quest)])
            .expect("seed quest");
        self.quests.insert(title.to_string(), id);
    }

    /// Reopen the app over the SAME on-disk SQLite tenant (a "server restart"): a **fresh store
    /// connection** over the same file, reusing the identity (so issued tokens stay valid). A new
    /// connection only sees balances that were actually written to disk — so a later `/state` read
    /// returning the credited balance proves the event log is durable, not merely in memory.
    fn restart(&mut self) {
        let app = self.app.as_ref().expect("no app");
        let dir_path = app.dir.path().to_path_buf();
        let identity = app.identity.clone();
        let provisioner = Provisioner::new(Backend::Sqlite { dir: dir_path });
        let store = provisioner
            .open("seed", SystemClock)
            .expect("reopen tenant store");
        let store = Arc::new(Mutex::new(store));
        self.app.as_mut().unwrap().state = AppState::new(store, identity);
    }
}

// ─── Given ───────────────────────────────────────────────────────────────────────────────────

#[given(regex = r#"^a fresh household with a Knight "([^"]+)"$"#)]
async fn fresh_household(world: &mut ApiWorld, knight_name: String) {
    world.app = Some(new_app());
    let body =
        json!({ "household_name": "home", "admin_name": knight_name, "admin_secret": "secret" });
    world.send("POST", "/register", None, Some(body)).await;
    assert_eq!(world.last_status, 200, "register should be 200");
    world.handle = world.last_body["household"]
        .as_str()
        .expect("household")
        .to_string();
    world.knight = world.last_body["token"]
        .as_str()
        .expect("token")
        .to_string();
}

#[given(regex = r#"^a Squire "([^"]+)"$"#)]
async fn a_squire(world: &mut ApiWorld, name: String) {
    let knight = world.knight.clone();
    let add = json!({ "role": "Squire", "display_name": name, "initial_secret": "lake" });
    world
        .send("POST", "/members", Some(&knight), Some(add))
        .await;
    assert_eq!(world.last_status, 200, "add member should be 200");
    let user = world.last_body["user"].as_u64().expect("user id") as u128;

    let login = json!({ "household": world.handle, "user": user, "secret": "lake" });
    world.send("POST", "/login", None, Some(login)).await;
    assert_eq!(world.last_status, 200, "login should be 200");
    let token = world.last_body["token"]
        .as_str()
        .expect("token")
        .to_string();
    world.squires.insert(name, (user, token));
}

// ─── When ────────────────────────────────────────────────────────────────────────────────────

#[when(regex = r#"^the Knight adjusts (\w+) by (-?\d+) coins with reason "([^"]*)"$"#)]
async fn knight_grants(world: &mut ApiWorld, name: String, amount: i64, reason: String) {
    let (id, _) = world.squire(&name);
    let knight = world.knight.clone();
    world.adjust(knight, id, amount, &reason, None).await;
}

#[when(
    regex = r#"^the Knight posts an adjust for (\w+) with amount (-?\d+), reason "([^"]*)", and currency null$"#
)]
async fn knight_adjust_null(world: &mut ApiWorld, name: String, amount: i64, reason: String) {
    let (id, _) = world.squire(&name);
    let knight = world.knight.clone();
    world
        .adjust(knight, id, amount, &reason, Some("null"))
        .await;
}

#[when(
    regex = r#"^(\w+) posts an adjust for (\w+) with amount (-?\d+), reason "([^"]*)", and currency null$"#
)]
async fn squire_adjust_null(
    world: &mut ApiWorld,
    actor: String,
    target: String,
    amount: i64,
    reason: String,
) {
    let (_, token) = world.squire(&actor);
    let (id, _) = world.squire(&target);
    world.adjust(token, id, amount, &reason, Some("null")).await;
}

// ─── Then ────────────────────────────────────────────────────────────────────────────────────

#[then("the request succeeds")]
async fn request_succeeds(world: &mut ApiWorld) {
    assert_eq!(
        world.last_status, 200,
        "expected 200, got {} ({})",
        world.last_status, world.last_body
    );
}

#[then(regex = r#"^the request is rejected with status (\d+)$"#)]
async fn request_rejected(world: &mut ApiWorld, status: u16) {
    assert_eq!(
        world.last_status, status,
        "expected {status}, got {} ({})",
        world.last_status, world.last_body
    );
}

#[then(regex = r#"^(\w+)'s coin balance is (-?\d+)$"#)]
async fn coin_balance_is(world: &mut ApiWorld, name: String, expected: i64) {
    let (_, token) = world.squire(&name);
    world.send("GET", "/state", Some(&token), None).await;
    assert_eq!(world.last_status, 200, "GET /state should be 200");
    let balance = world.last_body["balance"].as_i64().expect("balance");
    assert_eq!(balance, expected, "balance for {name}");
}

// ─── claims & review (the central credit flow) ─────────────────────────────────────────────────

#[given(regex = r#"^a daily quest "([^"]+)" worth (\d+) coins$"#)]
async fn a_quest(world: &mut ApiWorld, title: String, reward: u32) {
    world.seed_quest(&title, reward);
}

#[when(regex = r#"^(\w+) submits a claim for "([^"]+)"$"#)]
async fn submits_claim(world: &mut ApiWorld, name: String, quest: String) {
    let (_, token) = world.squire(&name);
    let quest_id = *world
        .quests
        .get(&quest)
        .unwrap_or_else(|| panic!("unknown quest {quest}"));
    let claim_id = world.mint_id();
    world.last_claim_id = claim_id;
    let on = world.today();
    let body = json!({ "claim_id": claim_id, "quest_id": quest_id, "on": on });
    world
        .send("POST", "/claims", Some(&token), Some(body))
        .await;
}

#[then("the claim is pending")]
async fn claim_is_pending(world: &mut ApiWorld) {
    assert_eq!(
        world.last_status, 200,
        "submit should be 200 ({})",
        world.last_body
    );
    assert_eq!(
        world.last_body["state"]["state"], "Pending",
        "fresh claim is Pending"
    );
}

#[when("the Knight approves the claim")]
async fn knight_approves(world: &mut ApiWorld) {
    let knight = world.knight.clone();
    let body = json!({ "claim_id": world.last_claim_id, "decision": { "verdict": "approve" } });
    world
        .send("POST", "/admin/review-claim", Some(&knight), Some(body))
        .await;
}

// ─── cash-out (SQUIRE-T-0118): a Squire draws down owed Cash, parent-approved ──────────────────

#[given(regex = r#"^the Knight grants (\w+) (\d+) dollars$"#)]
async fn knight_grants_cash(world: &mut ApiWorld, name: String, amount: i64) {
    let (id, _) = world.squire(&name);
    let knight = world.knight.clone();
    world
        .adjust(knight, id, amount, "owed for chores", Some("Cash"))
        .await;
}

#[when(regex = r#"^(\w+) requests to cash out (\d+) dollars$"#)]
async fn requests_cashout(world: &mut ApiWorld, name: String, amount: i64) {
    let (_, token) = world.squire(&name);
    let request_id = world.mint_id();
    world.last_request_id = request_id;
    let body = json!({ "request_id": request_id, "amount": amount });
    world
        .send("POST", "/cash-out-requests", Some(&token), Some(body))
        .await;
}

#[when("the Knight approves the cash-out")]
async fn knight_approves_cashout(world: &mut ApiWorld) {
    let knight = world.knight.clone();
    let body = json!({ "request_id": world.last_request_id, "decision": { "verdict": "approve" } });
    world
        .send("POST", "/admin/review-cashout", Some(&knight), Some(body))
        .await;
}

#[when("the Knight rejects the cash-out")]
async fn knight_rejects_cashout(world: &mut ApiWorld) {
    let knight = world.knight.clone();
    let body = json!({ "request_id": world.last_request_id, "decision": { "verdict": "reject" } });
    world
        .send("POST", "/admin/review-cashout", Some(&knight), Some(body))
        .await;
}

#[then(regex = r#"^(\w+)'s owed cash is (-?\d+)$"#)]
async fn owed_cash_is(world: &mut ApiWorld, name: String, expected: i64) {
    let (_, token) = world.squire(&name);
    world.send("GET", "/state", Some(&token), None).await;
    assert_eq!(world.last_status, 200, "GET /state should be 200");
    let cash = world.last_body["balances"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|b| b["currency"] == "Cash")
        .map(|b| b["balance"].as_i64().unwrap_or(0))
        .unwrap_or(0);
    assert_eq!(cash, expected, "owed cash for {name} ({})", world.last_body);
}

#[when(regex = r#"^(\w+) requests the household review$"#)]
async fn squire_requests_review(world: &mut ApiWorld, name: String) {
    let (_, token) = world.squire(&name);
    world
        .send("GET", "/household-review", Some(&token), None)
        .await;
}

// ─── full-stack durability (phone → API → store, SQUIRE-T-0116) ──────────────────────────────────

#[when("the server restarts")]
async fn the_server_restarts(world: &mut ApiWorld) {
    world.restart();
}

#[then(regex = r#"^(\w+)'s active quests (include|do not include) "([^"]+)"$"#)]
async fn active_quests(world: &mut ApiWorld, name: String, verb: String, title: String) {
    let (_, token) = world.squire(&name);
    world.send("GET", "/state", Some(&token), None).await;
    let present = world.last_body["quests_today"]
        .as_array()
        .map(|a| {
            a.iter()
                .any(|q| q["title"].as_str() == Some(title.as_str()))
        })
        .unwrap_or(false);
    if verb == "include" {
        assert!(
            present,
            "expected '{title}' in active quests; got {}",
            world.last_body["quests_today"]
        );
    } else {
        assert!(
            !present,
            "expected '{title}' NOT in active quests; got {}",
            world.last_body["quests_today"]
        );
    }
}

#[tokio::main]
async fn main() {
    ApiWorld::cucumber().run_and_exit("tests/features").await;
}
