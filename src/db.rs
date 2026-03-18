use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection};
use tracing::info;

use crate::scheduler::{ExecutionRecord, task::TaskStatus};

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(data_dir: &str) -> anyhow::Result<Self> {
        let db_path = Path::new(data_dir).join("courier.db");
        let conn = Connection::open(&db_path)?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS execution_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_name TEXT NOT NULL,
                status TEXT NOT NULL,
                executed_at TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                articles_count INTEGER NOT NULL DEFAULT 0,
                error_message TEXT,
                digest_content TEXT
            )",
        )?;

        info!("📂 Database opened: {}", db_path.display());
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert_record(&self, record: &ExecutionRecord) -> anyhow::Result<()> {
        let conn = self.conn.lock()
            .map_err(|e| anyhow::anyhow!("Database lock poisoned: {}", e))?;
        conn.execute(
            "INSERT INTO execution_history (task_name, status, executed_at, duration_ms, articles_count, error_message, digest_content)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.task_name,
                format!("{:?}", record.status),
                record.executed_at.to_rfc3339(),
                record.duration_ms,
                record.articles_count,
                record.error_message,
                record.digest_content,
            ],
        )?;
        Ok(())
    }

    pub fn get_history(&self, limit: usize) -> anyhow::Result<Vec<ExecutionRecord>> {
        let conn = self.conn.lock()
            .map_err(|e| anyhow::anyhow!("Database lock poisoned: {}", e))?;
        let mut stmt = conn.prepare(
            "SELECT task_name, status, executed_at, duration_ms, articles_count, error_message, digest_content
             FROM execution_history ORDER BY id DESC LIMIT ?1",
        )?;

        let records = stmt
            .query_map(params![limit], |row| {
                let status_str: String = row.get(1)?;
                let status = if status_str == "Success" {
                    TaskStatus::Success
                } else {
                    TaskStatus::Failed
                };

                let executed_at_str: String = row.get(2)?;
                let executed_at = chrono::DateTime::parse_from_rfc3339(&executed_at_str)
                    .map(|dt| dt.with_timezone(&chrono::Local))
                    .unwrap_or_else(|_| chrono::Local::now());

                Ok(ExecutionRecord {
                    task_name: row.get(0)?,
                    status,
                    executed_at,
                    duration_ms: row.get::<_, i64>(3)? as u64,
                    articles_count: row.get::<_, i64>(4)? as usize,
                    error_message: row.get(5)?,
                    digest_content: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(records)
    }

    #[allow(dead_code)]
    pub fn get_record_content(&self, id: i64) -> anyhow::Result<Option<String>> {
        let conn = self.conn.lock()
            .map_err(|e| anyhow::anyhow!("Database lock poisoned: {}", e))?;
        let content = conn
            .query_row(
                "SELECT digest_content FROM execution_history WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .ok();
        Ok(content)
    }

    pub fn delete_history_by_timestamps(&self, timestamps: &[String]) -> anyhow::Result<usize> {
        let conn = self.conn.lock()
            .map_err(|e| anyhow::anyhow!("Database lock poisoned: {}", e))?;
        let placeholders: Vec<String> = timestamps.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
        let sql = format!(
            "DELETE FROM execution_history WHERE executed_at IN ({})",
            placeholders.join(", ")
        );
        let params: Vec<&dyn rusqlite::types::ToSql> = timestamps.iter().map(|t| t as &dyn rusqlite::types::ToSql).collect();
        let deleted = conn.execute(&sql, params.as_slice())?;
        Ok(deleted)
    }

    pub fn clear_all_history(&self) -> anyhow::Result<usize> {
        let conn = self.conn.lock()
            .map_err(|e| anyhow::anyhow!("Database lock poisoned: {}", e))?;
        let deleted = conn.execute("DELETE FROM execution_history", [])?;
        Ok(deleted)
    }
}
