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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskSnapshot {
    pub revision: i64,
    pub tasks: Vec<Task>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum TaskChange {
    Add {
        task: Task,
    },
    Patch {
        id: String,
        title: Option<String>,
        completed: Option<bool>,
        quadrant: Option<String>,
    },
    Remove {
        id: String,
    },
    Move {
        id: String,
        quadrant: String,
        before: Option<String>,
    },
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
        connection.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; PRAGMA busy_timeout = 5000;
             CREATE TABLE IF NOT EXISTS task_document (id INTEGER PRIMARY KEY CHECK (id = 1), version INTEGER NOT NULL, content TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS task_revision (id INTEGER PRIMARY KEY CHECK (id = 1), revision INTEGER NOT NULL);
             INSERT OR IGNORE INTO task_revision VALUES (1, 0);"
        ).map_err(|e| e.to_string())?;
        Ok(Self(Mutex::new(connection)))
    }

    fn read(connection: &Connection) -> Result<Vec<Task>, String> {
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
    pub fn snapshot(&self) -> Result<TaskSnapshot, String> {
        let connection = self.0.lock().map_err(|e| e.to_string())?;
        let tasks = Self::read(&connection)?;
        let revision = connection
            .query_row(
                "SELECT revision FROM task_revision WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        Ok(TaskSnapshot { tasks, revision })
    }
    // Read, merge and write under one lock and transaction. A window only sends its intent.
    pub fn commit(&self, changes: &[TaskChange]) -> Result<TaskSnapshot, String> {
        let mut connection = self.0.lock().map_err(|e| e.to_string())?;
        let transaction = connection.transaction().map_err(|e| e.to_string())?;
        let mut tasks = Self::read(&transaction)?;
        for change in changes {
            match change {
                TaskChange::Add { task } => {
                    if !tasks.iter().any(|t| t.id == task.id) {
                        tasks.push(task.clone());
                    }
                }
                TaskChange::Patch {
                    id,
                    title,
                    completed,
                    quadrant,
                } => {
                    if let Some(task) = tasks.iter_mut().find(|t| &t.id == id) {
                        if let Some(value) = title {
                            task.title = value.clone();
                        }
                        if let Some(value) = completed {
                            task.completed = *value;
                        }
                        if let Some(value) = quadrant {
                            task.quadrant = value.clone();
                        }
                    }
                }
                TaskChange::Remove { id } => tasks.retain(|t| &t.id != id),
                TaskChange::Move {
                    id,
                    quadrant,
                    before,
                } => {
                    if before.as_ref() == Some(id) {
                        continue;
                    }
                    if let Some(index) = tasks.iter().position(|t| &t.id == id) {
                        let mut task = tasks.remove(index);
                        task.quadrant = quadrant.clone();
                        let index = before
                            .as_ref()
                            .and_then(|anchor| {
                                tasks
                                    .iter()
                                    .position(|t| &t.id == anchor && &t.quadrant == quadrant)
                            })
                            .unwrap_or_else(|| {
                                tasks
                                    .iter()
                                    .rposition(|t| &t.quadrant == quadrant)
                                    .map(|i| i + 1)
                                    .unwrap_or(tasks.len())
                            });
                        tasks.insert(index, task);
                    }
                }
            }
        }
        validate(&tasks)?;
        let content = serde_json::to_string(&tasks).map_err(|e| e.to_string())?;
        transaction.execute(
            "INSERT INTO task_document (id, version, content) VALUES (1, 1, ?1) ON CONFLICT(id) DO UPDATE SET content = excluded.content, version = excluded.version", params![content]
        ).map_err(|e| e.to_string())?;
        transaction
            .execute(
                "UPDATE task_revision SET revision = revision + 1 WHERE id = 1",
                [],
            )
            .map_err(|e| e.to_string())?;
        let revision = transaction
            .query_row(
                "SELECT revision FROM task_revision WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        transaction.commit().map_err(|e| e.to_string())?;
        Ok(TaskSnapshot { revision, tasks })
    }
    #[cfg(test)]
    pub fn load(&self) -> Result<Vec<Task>, String> {
        Ok(self.snapshot()?.tasks)
    }
    #[cfg(test)]
    pub fn save(&self, tasks: &[Task]) -> Result<(), String> {
        validate(tasks)?;
        let existing = self.load()?;
        let changes: Vec<TaskChange> = existing
            .into_iter()
            .map(|t| TaskChange::Remove { id: t.id })
            .chain(tasks.iter().map(|t| TaskChange::Add { task: t.clone() }))
            .collect();
        self.commit(&changes).map(|_| ())
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

    #[test]
    fn two_windows_merge_fields_and_do_not_resurrect_deleted_tasks() {
        let db = TaskDatabase::open(Path::new(":memory:")).unwrap();
        db.commit(&[TaskChange::Add { task: task("a") }]).unwrap();
        db.commit(&[TaskChange::Patch {
            id: "a".into(),
            title: Some("Edited in main".into()),
            completed: None,
            quadrant: None,
        }])
        .unwrap();
        let snapshot = db
            .commit(&[TaskChange::Patch {
                id: "a".into(),
                title: None,
                completed: Some(true),
                quadrant: None,
            }])
            .unwrap();
        assert_eq!(snapshot.tasks[0].title, "Edited in main");
        assert!(snapshot.tasks[0].completed);
        db.commit(&[TaskChange::Remove { id: "a".into() }]).unwrap();
        assert!(db
            .commit(&[TaskChange::Patch {
                id: "a".into(),
                title: Some("late edit".into()),
                completed: None,
                quadrant: None
            }])
            .unwrap()
            .tasks
            .is_empty());
    }
    #[test]
    fn parallel_commits_keep_both_windows_tasks_and_revision() {
        let db = std::sync::Arc::new(TaskDatabase::open(Path::new(":memory:")).unwrap());
        let threads: Vec<_> = (0..20)
            .map(|i| {
                let db = db.clone();
                std::thread::spawn(move || {
                    db.commit(&[TaskChange::Add {
                        task: task(&i.to_string()),
                    }])
                    .unwrap()
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        let snapshot = db.snapshot().unwrap();
        assert_eq!(snapshot.tasks.len(), 20);
        assert_eq!(snapshot.revision, 20);
    }
    #[test]
    fn invalid_batch_rolls_back_all_changes_and_revision() {
        let db = TaskDatabase::open(Path::new(":memory:")).unwrap();
        let mut invalid = task("b");
        invalid.quadrant = "unknown".into();
        assert!(db
            .commit(&[
                TaskChange::Add { task: task("a") },
                TaskChange::Add { task: invalid }
            ])
            .is_err());
        assert_eq!(db.snapshot().unwrap().revision, 0);
        assert!(db.load().unwrap().is_empty());
    }
    #[test]
    fn moving_uses_latest_order_and_preserves_other_window_additions() {
        let db = TaskDatabase::open(Path::new(":memory:")).unwrap();
        db.save(&[task("a"), task("b"), task("c")]).unwrap();
        db.commit(&[TaskChange::Move {
            id: "a".into(),
            quadrant: "do".into(),
            before: None,
        }])
        .unwrap();
        db.commit(&[TaskChange::Add { task: task("d") }]).unwrap();
        let snapshot = db
            .commit(&[TaskChange::Move {
                id: "c".into(),
                quadrant: "plan".into(),
                before: None,
            }])
            .unwrap();
        assert_eq!(
            snapshot
                .tasks
                .iter()
                .filter(|t| t.quadrant == "do")
                .map(|t| t.id.as_str())
                .collect::<Vec<_>>(),
            vec!["b", "a", "d"]
        );
        assert_eq!(
            snapshot
                .tasks
                .iter()
                .find(|t| t.id == "c")
                .unwrap()
                .quadrant,
            "plan"
        );
    }
}
