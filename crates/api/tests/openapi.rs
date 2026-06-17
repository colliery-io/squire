//! OpenAPI conformance test (SQUIRE-T-0031 / ADR SQUIRE-A-0009).
//!
//! Guards the frozen `crates/api/openapi.json` against drift: the live `ApiDoc::openapi()` is
//! re-derived from the Rust types, serialized, and asserted byte-equal to the committed file. Any
//! intentional contract change must be re-frozen by re-running the generator (or this test with
//! `UPDATE_OPENAPI=1`). A smoke check asserts the document carries the key schemas, paths, and the
//! `int64` id invariant.

const FROZEN_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/openapi.json");

/// Pretty-printed JSON of the live doc, with a trailing newline (matching the generator).
fn live_json() -> String {
    let doc = api::openapi_doc();
    let json = serde_json::to_string_pretty(&doc).expect("OpenAPI doc serializes to JSON");
    format!("{json}\n")
}

/// The committed doc equals the freshly-generated one. On an intentional contract change, re-run
/// `cargo run -p api --example gen_openapi` (or this test with `UPDATE_OPENAPI=1`) to re-freeze.
#[test]
fn frozen_openapi_matches_generated() {
    let live = live_json();

    if std::env::var_os("UPDATE_OPENAPI").is_some() {
        std::fs::write(FROZEN_PATH, &live).expect("write frozen openapi.json");
        eprintln!("UPDATE_OPENAPI=1 — re-froze {FROZEN_PATH}");
        return;
    }

    let frozen = std::fs::read_to_string(FROZEN_PATH).unwrap_or_else(|e| {
        panic!(
            "could not read frozen {FROZEN_PATH}: {e}. \
             Generate it with `cargo run -p api --example gen_openapi`."
        )
    });

    assert_eq!(
        live, frozen,
        "\n\nThe committed crates/api/openapi.json is out of date with the Rust types.\n\
         If this change is INTENTIONAL, re-freeze it with:\n\
         \n    cargo run -p api --example gen_openapi\n\
         (or re-run this test with UPDATE_OPENAPI=1)\n\n\
         If it is NOT intentional, you have changed the wire contract — revert it.\n"
    );
}

/// Smoke check: the frozen doc round-trips as valid OpenAPI and carries the key schemas and paths.
#[test]
fn frozen_openapi_has_key_schemas_and_paths() {
    let frozen = std::fs::read_to_string(FROZEN_PATH).expect("read frozen openapi.json");
    let doc: serde_json::Value = serde_json::from_str(&frozen).expect("frozen file is valid JSON");

    let schemas = &doc["components"]["schemas"];
    for schema in [
        "StateView",
        "HouseholdReview",
        "QuestStatus",
        "ClaimState",
        "LockReason",
        "RegisterHouseholdReq",
        "UserId",
    ] {
        assert!(
            schemas.get(schema).is_some(),
            "frozen openapi.json is missing schema `{schema}`"
        );
    }

    let paths = &doc["paths"];
    for path in [
        "/state",
        "/claims",
        "/redemption-requests",
        "/admin/review-claim",
        "/household-review",
        "/register",
        "/login",
        "/members",
    ] {
        assert!(
            paths.get(path).is_some(),
            "frozen openapi.json is missing path `{path}`"
        );
    }

    // A-0009 invariant: id newtypes are schema-typed int64 (numeric on the wire).
    assert_eq!(schemas["UserId"]["type"], "integer");
    assert_eq!(schemas["UserId"]["format"], "int64");

    // The bearer security scheme is documented.
    assert!(
        doc["components"]["securitySchemes"]["bearer_auth"].is_object(),
        "frozen openapi.json is missing the bearer_auth security scheme"
    );

    // Externally-tagged data-carrying enum: `ClaimState::Approved { points }` must serialize as
    // `{ "Approved": { points } }` (serde external tagging), reflected as a oneOf object variant.
    let claim_state = &schemas["ClaimState"]["oneOf"];
    let has_approved_object = claim_state
        .as_array()
        .expect("ClaimState is a oneOf")
        .iter()
        .any(|v| v["properties"].get("Approved").is_some());
    assert!(
        has_approved_object,
        "ClaimState schema does not reflect serde's externally-tagged `Approved` variant"
    );
}
