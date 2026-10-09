use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path, sync::Mutex};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub quadrant: String,
}

pub struct TaskDatabase(pub Mutex<Connection>);

fn validate(tasks: &[Task]) -> Result<(), String> {
    let mut ids = HashSet::new();
    for task in tasks {
        if task.id.is_empty()
            || !ids.insert(&task.id)
            || task.title.trim().is_empty()
            // JavaScript counts UTF-16 code units; match that limit exactly.
            || task.title.encode_utf16().count() > 1000
            || !["do", "plan", "delegate", "eliminate"].contains(&task.quadrant.as_str())
        {
            return Err("任务数据格式不正确，未写入磁盘。".into());
        }
    }
    Ok(())
}

impl TaskDatabase {
    pub fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path).map_err(|e| e.to_string())?;
        connection
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = FULL;
                 PRAGMA busy_timeout = 5000;
                 CREATE TABLE IF NOT EXISTS task_document (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    version INTEGER NOT NULL,
                    content TEXT NOT NULL
                 );",
            )
            .map_err(|e| e.to_string())?;
        Ok(Self(Mutex::new(connection)))
    }

    pub fn load(&self) -> Result<Vec<Task>, String> {
        let connection = self.0.lock().map_err(|e| e.to_string())?;
        let mut statement = connection
            .prepare("SELECT version, content FROM task_document WHERE id = 1")
            .map_err(|e| e.to_string())?;
        let mut rows = statement.query([]).map_err(|e| e.to_string())?;
        match rows.next().map_err(|e| e.to_string())? {
            None => Ok(vec![]),
            Some(row) => {
                let version: i64 = row.get(0).map_err(|e| e.to_string())?;
                if version != 1 {
                    return Err("任务文件版本不受支持，原文件已保留。".into());
                }
                let content: String = row.get(1).map_err(|e| e.to_string())?;
                let tasks: Vec<Task> = serde_json::from_str(&content).map_err(|e| e.to_string())?;
                validate(&tasks)?;
                Ok(tasks)
            }
        }
    }

    pub fn save(&self, tasks: &[Task]) -> Result<(), String> {
        validate(tasks)?;
        let content = serde_json::to_string(tasks).map_err(|e| e.to_string())?;
        let mut connection = self.0.lock().map_err(|e| e.to_string())?;
        let transaction = connection.transaction().map_err(|e| e.to_string())?;
        transaction
            .execute(
                "INSERT INTO task_document (id, version, content) VALUES (1, 1, ?1)
                 ON CONFLICT(id) DO UPDATE SET content = excluded.content, version = excluded.version",
                params![content],
            )
            .map_err(|e| e.to_string())?;
        transaction.commit().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn task(id: &str) -> Task {
        Task {
            id: id.into(),
            title: "核对 Mac 交付清单".into(),
            completed: false,
            quadrant: "do".into(),
        }
    }
    #[test]
    fn persists_order_completion_and_quadrants_after_reopening() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.sqlite3");
        let mut moved = task("b");
        moved.quadrant = "plan".into();
        moved.completed = true;
        let tasks = vec![moved, task("a")];
        {
            let db = TaskDatabase::open(&path).unwrap();
            db.save(&tasks).unwrap();
        }
        assert_eq!(TaskDatabase::open(&path).unwrap().load().unwrap(), tasks);
    }
    #[test]
    fn invalid_write_preserves_saved_document() {
        let db = TaskDatabase::open(Path::new(":memory:")).unwrap();
        db.save(&[task("a")]).unwrap();
        assert!(db.save(&[task("a"), task("a")]).is_err());
        let mut invalid = task("b");
        invalid.title = " ".into();
        assert!(db.save(&[invalid]).is_err());
        assert_eq!(db.load().unwrap(), vec![task("a")]);
    }
    #[test]
    fn unknown_schema_and_corruption_are_reported_not_reset() {
        let db = TaskDatabase::open(Path::new(":memory:")).unwrap();
        db.0.lock()
            .unwrap()
            .execute("INSERT INTO task_document VALUES (1, 2, '[]')", [])
            .unwrap();
        assert!(db.load().is_err());
        db.0.lock()
            .unwrap()
            .execute(
                "UPDATE task_document SET version = 1, content = 'broken'",
                [],
            )
            .unwrap();
        assert!(db.load().is_err());
    }
    #[test]
    fn deleting_all_tasks_persists_empty_document() {
        let db = TaskDatabase::open(Path::new(":memory:")).unwrap();
        db.save(&[task("a")]).unwrap();
        db.save(&[]).unwrap();
        assert_eq!(db.load().unwrap(), Vec::<Task>::new());
    }
}
