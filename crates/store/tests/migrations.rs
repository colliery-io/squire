//! Migration smoke tests (SQLite, default features).
//!
//! Opens a temp-file SQLite DB, runs the embedded migrations, and asserts that each
//! expected table exists and is empty. A temp file (not `:memory:`) is used so the same
//! database is visible across statements on the one connection — and to mirror real use.

use diesel::connection::SimpleConnection;
use diesel::sql_types::BigInt;
use diesel::sqlite::SqliteConnection;
use diesel::{Connection, QueryableByName, RunQueryDsl};

use store::{run_migrations, AnyConnection, SqliteStore};

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = BigInt)]
    n: i64,
}

fn count_any(conn: &mut AnyConnection, table: &str) -> i64 {
    let row: Count = diesel::sql_query(format!("SELECT COUNT(*) AS n FROM {table}"))
        .get_result(conn)
        .unwrap_or_else(|e| panic!("count {table}: {e}"));
    row.n
}

fn temp_db_url() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("squire-test.sqlite");
    let url = path.to_str().expect("utf-8 path").to_string();
    (dir, url)
}

fn count(conn: &mut SqliteConnection, table: &str) -> i64 {
    let row: Count = diesel::sql_query(format!("SELECT COUNT(*) AS n FROM {table}"))
        .get_result(conn)
        .unwrap_or_else(|e| panic!("count {table}: {e}"));
    row.n
}

#[test]
fn migrations_apply_clean_and_tables_exist() {
    let (_dir, url) = temp_db_url();
    let mut conn = SqliteConnection::establish(&url).expect("establish");

    // Apply via the public runner.
    run_migrations(&mut conn).expect("run_migrations");

    for table in ["users", "quests", "items", "achievements", "events"] {
        assert_eq!(
            count(&mut conn, table),
            0,
            "table {table} should exist and be empty"
        );
    }
}

#[test]
fn migrations_are_idempotent() {
    let (_dir, url) = temp_db_url();
    let mut conn = SqliteConnection::establish(&url).expect("establish");

    run_migrations(&mut conn).expect("first run");
    // Running again should be a no-op (no pending migrations), not an error.
    run_migrations(&mut conn).expect("second run is idempotent");

    assert_eq!(count(&mut conn, "users"), 0);
}

#[test]
fn sqlite_store_open_runs_migrations() {
    let (_dir, url) = temp_db_url();
    let mut store = SqliteStore::open(&url).expect("open store");

    assert_eq!(count_any(&mut store.connection(), "events"), 0);
}

#[test]
fn events_table_is_append_only_ordered() {
    // Sanity-check the events log shape. `seq` is application-assigned (portable across
    // SQLite/Postgres — no AUTOINCREMENT), so the writer supplies monotone values exactly
    // as the real `apply` will (max(seq)+1). Uses raw SQL (the typed insert path lands in a
    // later task).
    let (_dir, url) = temp_db_url();
    let mut conn = SqliteConnection::establish(&url).expect("establish");
    run_migrations(&mut conn).expect("run_migrations");

    conn.batch_execute(
        "INSERT INTO events (seq, kind, squire, at) VALUES (1, 'CompletionClaimed', '1', 100);
         INSERT INTO events (seq, kind, squire, at) VALUES (2, 'CompletionApproved', '1', 200);",
    )
    .expect("insert events");

    let row: Count =
        diesel::sql_query("SELECT MIN(seq) AS n FROM events WHERE kind = 'CompletionApproved'")
            .get_result(&mut conn)
            .expect("query seq");
    let approved_seq = row.n;

    let row: Count =
        diesel::sql_query("SELECT MIN(seq) AS n FROM events WHERE kind = 'CompletionClaimed'")
            .get_result(&mut conn)
            .expect("query seq");
    let claimed_seq = row.n;

    assert!(claimed_seq < approved_seq, "seq must order by insertion");
    assert_eq!(count(&mut conn, "events"), 2);
}
