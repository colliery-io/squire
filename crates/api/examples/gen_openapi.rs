//! Regenerate the frozen `crates/api/openapi.json` from the Rust types (SQUIRE-T-0031).
//!
//! Run from the workspace root:
//!
//! ```sh
//! cargo run -p api --example gen_openapi
//! ```
//!
//! This is the source-of-truth generator: it pretty-prints `ApiDoc::openapi()` to
//! `crates/api/openapi.json`. The conformance test (`tests/openapi.rs`) asserts the committed
//! file equals this output (and can re-freeze it via `UPDATE_OPENAPI=1`).

fn main() -> std::io::Result<()> {
    let doc = api::openapi_doc();
    let json = serde_json::to_string_pretty(&doc).expect("OpenAPI doc serializes to JSON");
    // Resolve relative to this crate's manifest dir so the path is stable regardless of cwd.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/openapi.json");
    std::fs::write(path, format!("{json}\n"))?;
    eprintln!("wrote {path}");
    Ok(())
}
