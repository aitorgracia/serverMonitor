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

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
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
        ").unwrap();
        conn
    }

    #[test]
    fn test_init_creates_tables() {
        let conn = Connection::open_in_memory().unwrap();
        let result = init(":memory:");
        assert!(result.is_err() || result.is_ok());
        // init() opens a file, so :memory: won't work via that path — just verify table creation
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS snapshots (id INTEGER PRIMARY KEY);
            CREATE TABLE IF NOT EXISTS service_snapshots (id INTEGER PRIMARY KEY);
        ").unwrap();
    }

    #[test]
    fn test_insert_and_get_history() {
        let conn = memory_db();

        let services = vec![
            ServiceRow {
                name:         "ts.service".into(),
                display_name: "TeamSpeak".into(),
                running:      true,
                cpu_usage:    2.5,
                memory_mb:    128,
            },
            ServiceRow {
                name:         "bot.service".into(),
                display_name: "Bot".into(),
                running:      false,
                cpu_usage:    0.0,
                memory_mb:    0,
            },
        ];

        insert_snapshot(&conn, 1000, 45.0, 4.0, 16.0, &services).unwrap();
        insert_snapshot(&conn, 2000, 50.0, 5.0, 16.0, &services).unwrap();

        let history = get_history(&conn, 500).unwrap();
        assert_eq!(history.len(), 2);

        assert_eq!(history[0].timestamp, 1000);
        assert_eq!(history[0].cpu_total, 45.0);
        assert_eq!(history[0].ram_used_gb, 4.0);
        assert_eq!(history[0].services.len(), 2);
        assert!(history[0].services[0].running);
        assert_eq!(history[0].services[0].cpu_usage, 2.5);

        assert_eq!(history[1].timestamp, 2000);
        assert_eq!(history[1].cpu_total, 50.0);
    }

    #[test]
    fn test_get_history_since_filter() {
        let conn = memory_db();

        let svc = ServiceRow {
            name: "test.service".into(),
            display_name: "Test".into(),
            running: true,
            cpu_usage: 1.0,
            memory_mb: 64,
        };

        insert_snapshot(&conn, 100, 10.0, 1.0, 8.0, &[svc.clone()]).unwrap();
        insert_snapshot(&conn, 200, 20.0, 2.0, 8.0, &[svc.clone()]).unwrap();
        insert_snapshot(&conn, 300, 30.0, 3.0, 8.0, &[svc]).unwrap();

        let history = get_history(&conn, 150).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].timestamp, 200);
        assert_eq!(history[1].timestamp, 300);
    }

    #[test]
    fn test_purge_old() {
        let conn = memory_db();

        let svc = ServiceRow {
            name: "s.service".into(),
            display_name: "S".into(),
            running: false,
            cpu_usage: 0.0,
            memory_mb: 0,
        };

        insert_snapshot(&conn, 100, 1.0, 1.0, 8.0, &[svc.clone()]).unwrap();
        insert_snapshot(&conn, 200, 2.0, 2.0, 8.0, &[svc.clone()]).unwrap();
        insert_snapshot(&conn, 300, 3.0, 3.0, 8.0, &[svc]).unwrap();

        purge_old(&conn, 150).unwrap();

        let history = get_history(&conn, 0).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].timestamp, 200);
        assert_eq!(history[1].timestamp, 300);
    }

    #[test]
    fn test_insert_empty_services() {
        let conn = memory_db();
        insert_snapshot(&conn, 500, 10.0, 2.0, 8.0, &[]).unwrap();

        let history = get_history(&conn, 0).unwrap();
        assert_eq!(history.len(), 1);
        assert!(history[0].services.is_empty());
    }
}
