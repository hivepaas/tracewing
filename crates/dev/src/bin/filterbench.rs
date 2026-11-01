//! `filterbench DB`: how long each way of finding flows takes on a seeded
//! database (`mitm-prototype seed --rich`). Builds every index on its own copy
//! of nothing: each goes in a database of its own, so sizes are exact and the
//! flows database is only read, once it has its bodies in a column of its own:
//! these indexes were chosen with text in SQL, before bodies left the database.

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tracewing_store::session::{BodyReader, StoredBody};

const BATCH: i64 = 5_000;
const PAGE: i64 = 200;

struct Index {
    name: &'static str,
    /// What goes in each row.
    source: &'static str,
    tokenize: &'static str,
    detail: &'static str,
}

const INDEXES: &[Index] = &[
    Index {
        name: "body_words_vn",
        source: "replace(replace(CAST(resp_body AS TEXT), 'đ', 'd'), 'Đ', 'd')",
        tokenize: "unicode61 remove_diacritics 2",
        detail: "full",
    },
    Index {
        name: "url_trigram",
        source: "host || path",
        tokenize: "trigram remove_diacritics 1",
        detail: "full",
    },
    Index {
        name: "body_words",
        source: "CAST(resp_body AS TEXT)",
        tokenize: "unicode61 remove_diacritics 2",
        detail: "full",
    },
    Index {
        name: "body_words_nophrase",
        source: "CAST(resp_body AS TEXT)",
        tokenize: "unicode61 remove_diacritics 2",
        detail: "none",
    },
    Index {
        name: "body_trigram",
        source: "CAST(resp_body AS TEXT)",
        tokenize: "trigram remove_diacritics 1",
        detail: "full",
    },
];

fn bodies_into_a_column(db: &Path, flows: &Connection) -> Result<()> {
    if flows.prepare("SELECT resp_body FROM flows LIMIT 1").is_ok() {
        return Ok(());
    }
    let session = db.parent().context("the database is in a session")?;
    flows.execute_batch("ALTER TABLE flows ADD COLUMN resp_body BLOB; BEGIN;")?;
    let mut reader = BodyReader::new(session);
    let mut select = flows.prepare(&format!(
        "SELECT id, {} FROM flows",
        StoredBody::columns("resp")
    ))?;
    let mut update = flows.prepare("UPDATE flows SET resp_body = ?2 WHERE id = ?1")?;
    let mut rows = select.query([])?;
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let body = StoredBody::from_row(row, 1)?;
        update.execute(params![id, reader.unpacked(&body, usize::MAX)])?;
    }
    drop(rows);
    flows.execute_batch("COMMIT;")?;
    Ok(())
}

fn main() -> Result<()> {
    let db = PathBuf::from(std::env::args().nth(1).context("usage: filterbench DB")?);
    let flows = Connection::open(&db)?;
    bodies_into_a_column(&db, &flows)?;
    let rows: i64 = flows.query_row("SELECT count(*) FROM flows", [], |r| r.get(0))?;
    let body: i64 =
        flows.query_row("SELECT sum(length(resp_body)) FROM flows", [], |r| r.get(0))?;
    println!(
        "{rows} flows, {:.0} MB of bodies, database {:.0} MB\n",
        body as f64 / 1e6,
        size_mb(&db)
    );

    println!("== filters on columns");
    add_column_indexes(&flows)?;
    let mut filters = Vec::new();
    for (name, filter) in [
        ("one host", "host = 'api.momo.vn'"),
        ("host suffix", "host LIKE '%.momo.vn'"),
        ("errors", "status >= 500"),
        ("no response", "status IS NULL"),
        ("POST", "method = 'POST'"),
        ("JSON", "content_type LIKE 'application/json%'"),
        (
            "POST + errors + host",
            "method = 'POST' AND status >= 400 AND host = 'api.momo.vn'",
        ),
        ("not one host", "host <> 'api.momo.vn'"),
        (
            "URL contains 'order' (LIKE)",
            "(host || path) LIKE '%order%'",
        ),
        (
            "URL contains a rare id (LIKE)",
            "(host || path) LIKE '%/users/1234567%'",
        ),
    ] {
        filters.push((
            name,
            format!("SELECT id FROM flows WHERE {filter} ORDER BY id DESC"),
        ));
    }
    for (name, sql) in &filters {
        report(&flows, name, sql, &[])?;
    }

    println!("\n== searching bodies with no index");
    for (name, sql) in [
        (
            "common word (LIKE)",
            "SELECT id FROM flows WHERE CAST(resp_body AS TEXT) LIKE '%đặt hàng%' ORDER BY id DESC",
        ),
        (
            "rare phrase (LIKE)",
            "SELECT id FROM flows WHERE CAST(resp_body AS TEXT) LIKE '%mã giảm giá%' ORDER BY id DESC",
        ),
        (
            "nothing (LIKE)",
            "SELECT id FROM flows WHERE CAST(resp_body AS TEXT) LIKE '%no_such_word_zz%' ORDER BY id DESC",
        ),
    ] {
        report(&flows, name, sql, &[])?;
    }

    let trace: String = flows.query_row(
        "SELECT substr(CAST(resp_body AS TEXT), instr(CAST(resp_body AS TEXT), 'trace_id') + 11, 32) FROM flows WHERE id = ?1",
        [rows / 2],
        |r| r.get(0),
    )?;

    let only = std::env::var("ONLY").ok();
    for index in INDEXES {
        if only
            .as_deref()
            .is_some_and(|o| !o.split(',').any(|n| n == index.name))
        {
            continue;
        }
        println!(
            "\n== {} ({}, detail={})",
            index.name, index.tokenize, index.detail
        );
        let path = db.with_extension(format!("{}.db", index.name));
        let _ = std::fs::remove_file(&path);
        let conn = build(&db, &path, index)?;
        let queries: Vec<(&str, String)> = match index.name {
            "url_trigram" => vec![
                ("common piece", "\"order\"".to_owned()),
                ("rare id", "\"/users/1234567\"".to_owned()),
                ("Vietnamese, unaccented", "\"ao khoac\"".to_owned()),
                ("nothing", "\"no_such_word_zz\"".to_owned()),
            ],
            "body_trigram" => vec![
                ("common word", "\"đặt hàng\"".to_owned()),
                ("unaccented", "\"dat hang\"".to_owned()),
                ("rare phrase", "\"mã giảm giá\"".to_owned()),
                ("one trace id", format!("\"{trace}\"")),
                ("piece of a trace id", format!("\"{}\"", &trace[8..20])),
                ("nothing", "\"no_such_word_zz\"".to_owned()),
            ],
            "body_words_vn" => vec![
                ("typed with marks", "\"dat hang\"".to_owned()),
                ("rare phrase", "\"ma giam gia\"".to_owned()),
                ("nothing", "\"zzznosuch\"".to_owned()),
            ],
            "body_words" => vec![
                ("common word", "\"đặt hàng\"".to_owned()),
                ("unaccented", "\"dat hang\"".to_owned()),
                ("rare phrase", "\"mã giảm giá\"".to_owned()),
                ("one trace id", format!("\"{trace}\"")),
                (
                    "piece of a trace id (words miss it)",
                    format!("\"{}\"", &trace[8..20]),
                ),
                ("nothing", "\"no_such_word_zz\"".to_owned()),
            ],
            _ => vec![
                ("common word", "hàng".to_owned()),
                ("one trace id", format!("\"{trace}\"")),
                ("nothing", "zzznosuch".to_owned()),
            ],
        };
        for (name, query) in &queries {
            report(
                &conn,
                name,
                "SELECT rowid FROM search WHERE search MATCH ?1 ORDER BY rowid DESC",
                &[query],
            )?;
        }
    }
    Ok(())
}

fn add_column_indexes(conn: &Connection) -> Result<()> {
    let before = indexes_size(conn)?;
    let started = Instant::now();
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS flows_status ON flows (status);
         CREATE INDEX IF NOT EXISTS flows_method ON flows (method);",
    )?;
    println!(
        "status and method indexes: {:.1} s, {:.0} MB",
        started.elapsed().as_secs_f64(),
        (indexes_size(conn)? - before) as f64 / 1e6
    );
    Ok(())
}

fn indexes_size(conn: &Connection) -> Result<i64> {
    let pages: i64 = conn.query_row("PRAGMA page_count", [], |r| r.get(0))?;
    let size: i64 = conn.query_row("PRAGMA page_size", [], |r| r.get(0))?;
    Ok(pages * size)
}

/// Fills a contentless index a batch at a time, as an indexer following the
/// flows table would, on one thread, and says how fast that went.
fn build(flows_path: &Path, path: &Path, index: &Index) -> Result<Connection> {
    let flows = Connection::open(flows_path)?;
    let mut conn = Connection::open(path)?;
    conn.execute_batch(&format!(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         CREATE VIRTUAL TABLE search USING fts5(
             text, tokenize = '{}', content = '', detail = {}
         );",
        index.tokenize, index.detail
    ))?;
    let started = Instant::now();
    let mut read = flows.prepare(&format!(
        "SELECT id, {} FROM flows WHERE id > ?1 ORDER BY id LIMIT ?2",
        index.source
    ))?;
    let mut last = 0i64;
    let mut count = 0u64;
    loop {
        let tx = conn.transaction()?;
        let mut seen = 0;
        {
            let mut insert =
                tx.prepare_cached("INSERT INTO search (rowid, text) VALUES (?1, ?2)")?;
            let mut rows = read.query(params![last, BATCH])?;
            while let Some(row) = rows.next()? {
                last = row.get(0)?;
                let text: Option<String> = row.get(1).ok();
                insert.execute(params![last, text.unwrap_or_default()])?;
                seen += 1;
            }
        }
        tx.commit()?;
        count += seen;
        if seen < BATCH as u64 {
            break;
        }
    }
    let took = started.elapsed();
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
    println!(
        "built: {:.1} s, {:.0} rows/s, {:.0} MB",
        took.as_secs_f64(),
        count as f64 / took.as_secs_f64(),
        size_mb(path)
    );
    Ok(conn)
}

/// The first page, as the list wants it, then every id, as a scrollbar wants them.
fn report(conn: &Connection, name: &str, sql: &str, args: &[&String]) -> Result<()> {
    let bind = |stmt: &mut rusqlite::Statement| -> Result<()> {
        for (i, arg) in args.iter().enumerate() {
            stmt.raw_bind_parameter(i + 1, arg.as_str())?;
        }
        Ok(())
    };
    let mut best_first = Duration::MAX;
    let mut best_all = Duration::MAX;
    let mut total = 0usize;
    for _ in 0..3 {
        let mut stmt = conn.prepare_cached(sql)?;
        bind(&mut stmt)?;
        let started = Instant::now();
        let mut rows = stmt.raw_query();
        let mut seen = 0i64;
        let mut first = None;
        while rows.next()?.is_some() {
            seen += 1;
            if seen == PAGE {
                first = Some(started.elapsed());
            }
        }
        let all = started.elapsed();
        best_first = best_first.min(first.unwrap_or(all));
        best_all = best_all.min(all);
        total = seen as usize;
    }
    println!(
        "  {name:<38} {total:>8} found · first {PAGE} in {:>8.2} ms · all in {:>8.1} ms",
        best_first.as_secs_f64() * 1000.,
        best_all.as_secs_f64() * 1000.
    );
    Ok(())
}

fn size_mb(path: &Path) -> f64 {
    std::fs::metadata(path).map_or(0., |m| m.len() as f64 / 1e6)
}
