//! Local Xmind files and app launching. Only storage owns task associations.
use crate::storage::{TaskDatabase, TaskSnapshot};
use serde::Deserialize;
use serde_json::json;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MindmapAction {
    Open,
    Create,
    Associate,
}

fn validate_file(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err("找不到导图文件，可能已移动或删除。请重新关联已有导图。".into());
    }
    if path
        .extension()
        .and_then(|v| v.to_str())
        .is_none_or(|v| !v.eq_ignore_ascii_case("xmind"))
    {
        return Err("请选择 .xmind 文件。".into());
    }
    let file = File::open(path).map_err(|e| format!("无法读取导图：{e}"))?;
    let archive = ZipArchive::new(file).map_err(|_| "文件不是有效的 Xmind 导图。")?;
    if !archive
        .file_names()
        .any(|name| matches!(name, "content.json" | "content.xml"))
    {
        return Err("文件中没有找到 Xmind 导图内容。".into());
    }
    Ok(())
}

// Native .xmind package follows Xmind's SDK: sheet JSON, metadata and manifest.
// No CLI, sign-in or network service is needed to create a blank map.
fn create_file(directory: &Path, title: &str) -> Result<PathBuf, String> {
    fs::create_dir_all(directory).map_err(|e| format!("无法创建导图目录：{e}"))?;
    let directory = directory.canonicalize().map_err(|e| e.to_string())?;
    let name: String = title
        .chars()
        .filter(|c| !c.is_control() && !"/\\:*?\"<>|".contains(*c))
        .take(48)
        .collect();
    let name = name.trim().trim_matches('.');
    let name = if name.is_empty() {
        "思维导图"
    } else {
        name
    };
    let unique = uuid::Uuid::new_v4().to_string();
    let path = directory.join(format!("{name}-{unique}.xmind"));
    let temporary = directory.join(format!(".{unique}.partial"));
    let result = (|| {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        let mut archive = ZipWriter::new(file);
        let content = json!([{"id":uuid::Uuid::new_v4().to_string(),"class":"sheet","title":"画布 1","rootTopic":{"id":uuid::Uuid::new_v4().to_string(),"class":"topic","title":title,"structureClass":"org.xmind.ui.logic.right"}}]);
        let metadata = json!({"dataStructureVersion":"3","layoutEngineVersion":"5","creator":{"name":"四象限","version":env!("CARGO_PKG_VERSION")}});
        let manifest = json!({"file-entries":{"content.json":{},"metadata.json":{}}});
        for (name, value) in [
            ("content.json", content),
            ("metadata.json", metadata),
            ("manifest.json", manifest),
        ] {
            archive
                .start_file(
                    name,
                    SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
                )
                .map_err(|e| e.to_string())?;
            archive
                .write_all(value.to_string().as_bytes())
                .map_err(|e| e.to_string())?;
        }
        archive
            .finish()
            .map_err(|e| e.to_string())?
            .sync_all()
            .map_err(|e| e.to_string())?;
        fs::rename(&temporary, &path).map_err(|e| e.to_string())?;
        Ok(path.clone())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|e: String| format!("无法保存新导图：{e}"))
}

fn create_and_link(db: &TaskDatabase, id: &str, directory: &Path) -> Result<TaskSnapshot, String> {
    let snapshot = db.snapshot()?;
    let task = snapshot
        .tasks
        .iter()
        .find(|task| task.id == id)
        .ok_or("任务已删除，无法创建导图。")?;
    if snapshot.mindmaps.contains_key(id) {
        return Ok(snapshot);
    }
    let path = create_file(directory, &task.title)?;
    let result = db.link_mindmap(id, &path, false);
    // A second window may have won the association, or the task was deleted.
    // Remove only the new, unopened file from this attempt, never a user's map.
    if result
        .as_ref()
        .ok()
        .and_then(|s| s.mindmaps.get(id))
        .is_none_or(|selected| Path::new(selected) != path)
    {
        let _ = fs::remove_file(&path);
    }
    result
}

fn launch(path: &Path) -> Result<(), String> {
    validate_file(path)?;
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("/usr/bin/open")
            .args(["-a", "Xmind"])
            .arg(path)
            .output()
            .map_err(|e| format!("无法启动 Xmind：{e}"))?;
        if status.status.success() {
            return Ok(());
        }
        Err("无法启动 Xmind。请确认已安装并能正常打开；导图和关联已保留，可稍后重试。".into())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("当前版本的导图联动仅支持 macOS。".into())
    }
}

#[tauri::command]
pub async fn mindmap_action(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    id: String,
    action: MindmapAction,
) -> Result<Option<TaskSnapshot>, String> {
    // File pickers and disk operations never block the event loop. In-flight
    // commands are also awaited by the frontend's hide/exit save barrier.
    tauri::async_runtime::spawn_blocking(move || {
        let database = app.state::<Result<TaskDatabase, String>>();
        let db = database.inner().as_ref().map_err(Clone::clone)?;
        let snapshot = match action {
            MindmapAction::Open => db.snapshot()?,
            MindmapAction::Create => {
                let directory = app
                    .path()
                    .document_dir()
                    .map_err(|e| e.to_string())?
                    .join("四象限导图");
                create_and_link(db, &id, &directory)?
            }
            MindmapAction::Associate => {
                if !db.snapshot()?.tasks.iter().any(|task| task.id == id) {
                    return Err("任务已删除，无法关联导图。".into());
                }
                let Some(file) = app
                    .dialog()
                    .file()
                    .set_parent(&window)
                    .set_title("关联已有思维导图")
                    .add_filter("Xmind 思维导图", &["xmind"])
                    .blocking_pick_file()
                else {
                    return Ok(None);
                };
                let path = file
                    .into_path()
                    .map_err(|e| e.to_string())?
                    .canonicalize()
                    .map_err(|e| format!("无法读取文件：{e}"))?;
                validate_file(&path)?;
                db.link_mindmap(&id, &path, true)?
            }
        };
        let _ = app.emit("tasks-updated", &snapshot);
        let path = snapshot
            .mindmaps
            .get(&id)
            .ok_or("任务尚未关联导图，请先新建或关联已有导图。")?;
        launch(Path::new(path))?;
        Ok(Some(snapshot))
    })
    .await
    .map_err(|e| format!("导图操作未完成：{e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Task, TaskChange};
    use std::io::Read;
    fn add(db: &TaskDatabase) {
        db.commit(&[TaskChange::Add {
            task: Task {
                id: "test".into(),
                title: "测试 / 中文与 emoji 🌱".into(),
                completed: false,
                quadrant: "plan".into(),
            },
        }])
        .unwrap();
    }
    #[test]
    fn creates_native_map_once_and_keeps_file_when_task_is_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let db = TaskDatabase::open(&dir.path().join("tasks.sqlite3")).unwrap();
        add(&db);
        let first = create_and_link(&db, "test", &dir.path().join("maps")).unwrap();
        let path = Path::new(&first.mindmaps["test"]);
        validate_file(path).unwrap();
        let mut zip = ZipArchive::new(File::open(path).unwrap()).unwrap();
        let mut content = String::new();
        zip.by_name("content.json")
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&content).unwrap()[0]["rootTopic"]["title"],
            first.tasks[0].title
        );
        let metadata: serde_json::Value =
            serde_json::from_reader(zip.by_name("metadata.json").unwrap()).unwrap();
        assert_eq!(metadata["dataStructureVersion"], "3");
        assert_eq!(metadata["layoutEngineVersion"], "5");
        let second = create_and_link(&db, "test", &dir.path().join("maps")).unwrap();
        assert_eq!(first.mindmaps, second.mindmaps);
        let deleted = db
            .commit(&[TaskChange::Remove { id: "test".into() }])
            .unwrap();
        assert!(deleted.mindmaps.is_empty());
        assert!(path.is_file());
        assert!(create_and_link(&db, "test", &dir.path().join("maps")).is_err());
    }
    #[test]
    fn concurrent_creation_and_old_clients_preserve_associations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.sqlite3");
        let db = TaskDatabase::open(&path).unwrap();
        add(&db);
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let path = &path;
                let directory = dir.path().join("maps");
                scope.spawn(move || {
                    let db = TaskDatabase::open(path).unwrap();
                    create_and_link(&db, "test", &directory).unwrap();
                });
            }
        });
        assert_eq!(fs::read_dir(dir.path().join("maps")).unwrap().count(), 1);
        let saved = db.snapshot().unwrap();
        let file = saved.mindmaps["test"].clone();
        let old = rusqlite::Connection::open(&path).unwrap();
        // Old clients know only the original v1 task JSON document.
        old.execute(
            "UPDATE task_document SET content = replace(content, 'plan', 'do') WHERE id=1",
            [],
        )
        .unwrap();
        assert_eq!(db.snapshot().unwrap().mindmaps["test"], file);
        old.execute("UPDATE task_document SET content='[]' WHERE id=1", [])
            .unwrap();
        assert!(db.snapshot().unwrap().mindmaps.is_empty());
        assert!(Path::new(&file).is_file());
    }
    #[test]
    fn relinks_existing_file_and_rejects_invalid_or_deleted_targets() {
        let dir = tempfile::tempdir().unwrap();
        let db = TaskDatabase::open(&dir.path().join("tasks.sqlite3")).unwrap();
        add(&db);
        let first = create_and_link(&db, "test", dir.path()).unwrap();
        let previous = first.mindmaps["test"].clone();
        let file = create_file(dir.path(), "已有导图").unwrap();
        validate_file(&file).unwrap();
        let linked = db.link_mindmap("test", &file, true).unwrap();
        assert_eq!(linked.mindmaps["test"], file.to_str().unwrap());
        assert!(Path::new(&previous).is_file());
        assert!(db.link_mindmap("missing", &file, true).is_err());
        let invalid = dir.path().join("invalid.xmind");
        fs::write(&invalid, "not a map").unwrap();
        assert!(validate_file(&invalid).is_err());
        fs::remove_file(&file).unwrap();
        assert!(validate_file(&file).is_err());
        assert_eq!(db.snapshot().unwrap().mindmaps, linked.mindmaps);
    }
}
