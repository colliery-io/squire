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

    let repo = std::env::var("SQUIRE_DIST_REPO").unwrap_or_else(|_| "colliery-io/squire".to_string());
    let (owner, name) = repo.split_once('/').unwrap_or(("colliery-io", "squire"));

    let outcome = self_update::backends::github::Update::configure()
        .repo_owner(owner)
        .repo_name(name)
        .bin_name("squire-serve")
        .current_version(env!("CARGO_PKG_VERSION"))
        .no_confirm(true)
        .show_download_progress(false)
        .show_output(false)
        .build()
        .and_then(|u| u.update());

    match outcome {
        Ok(self_update::Status::Updated(v)) => {
            println!("  Self-update:             updated squire-serve → v{v}; restarting…");
            reexec();
        }
        Ok(self_update::Status::UpToDate(_)) => {}
        Err(e) => eprintln!("self-update: {e} (continuing on the current build)"),
    }
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
    let exe =
        std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("squire-serve"));
    let args: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new(&exe).args(&args).exec(); // replaces this process
        eprintln!("self-update: re-exec failed ({err}); exiting so a supervisor can restart");
        std::process::exit(0);
    }
    #[cfg(not(unix))]
    {
        let _ = std::process::Command::new(&exe).args(&args).spawn();
        std::process::exit(0);
    }
}
