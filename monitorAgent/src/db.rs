use rusqlite::{Connection, Result, params};
use serde::Serialize;

pub fn init(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS snapshots (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp    INTEGER NOT NULL,
            cpu_total    REAL    NOT NULL,
            ram_used_gb  REAL    NOT NULL,
            ram_total_gb REAL    NOT NULL
        );

        CREATE TABLE IF NOT EXISTS service_snapshots (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
            name        TEXT    NOT NULL,
            display_name TEXT   NOT NULL,
            running     INTEGER NOT NULL,
            cpu_usage   REAL    NOT NULL,
            memory_mb   INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp ON snapshots(timestamp);
    ")?;
    Ok(conn)
}

#[derive(Debug, Serialize, Clone)]
pub struct ServiceRow {
    pub name:         String,
    pub display_name: String,
    pub running:      bool,
    pub cpu_usage:    f32,
    pub memory_mb:    u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct SnapshotRow {
    pub timestamp:    i64,
    pub cpu_total:    f32,
    pub ram_used_gb:  f32,
    pub ram_total_gb: f32,
    pub services:     Vec<ServiceRow>,
}

pub fn insert_snapshot(
    conn:        &Connection,
    timestamp:   i64,
    cpu_total:   f32,
    ram_used_gb: f32,
    ram_total_gb: f32,
    services:    &[ServiceRow],
) -> Result<()> {
    conn.execute(
        "INSERT INTO snapshots (timestamp, cpu_total, ram_used_gb, ram_total_gb)
         VALUES (?1, ?2, ?3, ?4)",
        params![timestamp, cpu_total, ram_used_gb, ram_total_gb],
    )?;

    let snapshot_id = conn.last_insert_rowid();

    for svc in services {
        conn.execute(
            "INSERT INTO service_snapshots (snapshot_id, name, display_name, running, cpu_usage, memory_mb)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                snapshot_id,
                svc.name,
                svc.display_name,
                svc.running as i32,
                svc.cpu_usage,
                svc.memory_mb,
            ],
        )?;
    }

    Ok(())
}

pub fn get_history(conn: &Connection, since_ts: i64) -> Result<Vec<SnapshotRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, cpu_total, ram_used_gb, ram_total_gb
         FROM snapshots WHERE timestamp >= ?1 ORDER BY timestamp ASC"
    )?;

    let rows: Vec<(i64, i64, f32, f32, f32)> = stmt.query_map(params![since_ts], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
    })?.filter_map(|r| r.ok()).collect();

    let mut snapshots = Vec::new();
    for (id, timestamp, cpu_total, ram_used_gb, ram_total_gb) in rows {
        let mut svc_stmt = conn.prepare(
            "SELECT name, display_name, running, cpu_usage, memory_mb
             FROM service_snapshots WHERE snapshot_id = ?1"
        )?;

        let services: Vec<ServiceRow> = svc_stmt.query_map(params![id], |row| {
            Ok(ServiceRow {
                name:         row.get(0)?,
                display_name: row.get(1)?,
                running:      row.get::<_, i32>(2)? != 0,
                cpu_usage:    row.get(3)?,
                memory_mb:    row.get(4)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        snapshots.push(SnapshotRow {
            timestamp,
            cpu_total,
            ram_used_gb,
            ram_total_gb,
            services,
        });
    }

    Ok(snapshots)
}

pub fn purge_old(conn: &Connection, cutoff_ts: i64) -> Result<()> {
    conn.execute("DELETE FROM snapshots WHERE timestamp < ?1", params![cutoff_ts])?;
    Ok(())
}
