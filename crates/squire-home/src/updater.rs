//! Server self-update (SQUIRE-T-0089 / ADR SQUIRE-A-0012): on startup the running squire-serve checks
//! the public dist repo's latest GitHub Release for a newer build of itself for this platform,
//! downloads + replaces its own binary, and **re-execs** into it.
//!
//! Silent + best-effort: any failure logs and the current binary keeps running. `SQUIRE_SELF_UPDATE=off`
//! disables it; a dev build (`cargo run`, exe under `target/`) is skipped so a working tree isn't
//! clobbered. Must run BEFORE the async runtime starts — `self_update` uses blocking HTTP, and nesting
//! a runtime inside tokio panics.

/// Check for + apply a newer server build. If it updates, this **re-execs into the new binary and
/// does not return**. Otherwise it returns (up to date / disabled / dev / any error — all best-effort).
pub fn maybe_self_update() {
    if std::env::var("SQUIRE_SELF_UPDATE").is_ok_and(|v| v.eq_ignore_ascii_case("off")) {
        return;
    }
    if is_dev_build() {
        return; // never replace a `cargo run` build
    }

    let repo =
        std::env::var("SQUIRE_DIST_REPO").unwrap_or_else(|_| "colliery-io/squire".to_string());
    let (owner, name) = repo.split_once('/').unwrap_or(("colliery-io", "squire"));

    // Authenticate the GitHub API calls when a token is configured (SQUIRE-T-0117): unauthenticated
    // is 60 req/hr/IP and 403s under repeated restarts; a token lifts it to 5000/hr. Optional —
    // without one we keep working, just rate-limited (logged).
    let mut builder = self_update::backends::github::Update::configure();
    builder
        .repo_owner(owner)
        .repo_name(name)
        .bin_name("squire-serve")
        .current_version(env!("CARGO_PKG_VERSION"))
        .no_confirm(true)
        .show_download_progress(false)
        .show_output(false);
    match update_token() {
        Some(tok) => {
            builder.auth_token(&tok);
        }
        None => tracing::warn!(
            "self-update: no SQUIRE_UPDATE_TOKEN/GITHUB_TOKEN set — using the unauthenticated \
             GitHub API (60 req/hr; updates may be rate-limited)"
        ),
    }
    let outcome = builder.build().and_then(|u| u.update());

    match outcome {
        Ok(self_update::Status::Updated(v)) => {
            tracing::info!(version = %v, "self-update: updated squire-serve; restarting…");
            reexec();
        }
        Ok(self_update::Status::UpToDate(_)) => {}
        Err(e) => {
            tracing::warn!(error = %e, "self-update: failed (continuing on the current build)")
        }
    }
}

/// A GitHub token for the update API calls — raises the 60→5000 req/hr rate limit (SQUIRE-T-0117).
/// Reads `SQUIRE_UPDATE_TOKEN`, then `GITHUB_TOKEN`. For the **public** dist repo any valid token
/// works (no scopes needed). `None` ⇒ unauthenticated (rate-limited but functional).
pub(crate) fn update_token() -> Option<String> {
    ["SQUIRE_UPDATE_TOKEN", "GITHUB_TOKEN"]
        .iter()
        .find_map(|k| std::env::var(k).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// True when the running exe lives under a `target/` dir — i.e. a `cargo build`/`cargo run` artifact.
fn is_dev_build() -> bool {
    let sep = std::path::MAIN_SEPARATOR;
    std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.contains(&format!("{sep}target{sep}"))))
        .unwrap_or(false)
}

/// Re-exec the (now-replaced) binary with the same args + env, so the update takes effect immediately.
fn reexec() -> ! {
    let exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("squire-serve"));
    let args: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new(&exe).args(&args).exec(); // replaces this process
        tracing::error!(error = %err, "self-update: re-exec failed; exiting so a supervisor can restart");
        std::process::exit(0);
    }
    #[cfg(not(unix))]
    {
        let _ = std::process::Command::new(&exe).args(&args).spawn();
        std::process::exit(0);
    }
}
