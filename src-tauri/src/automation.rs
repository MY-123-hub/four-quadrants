//! Headless adapters only; all persisted rules and transactions stay in storage.
use crate::storage::{Task, TaskChange, TaskDatabase, TaskSnapshot};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    io::{self, BufRead, Read, Write},
    path::{Path, PathBuf},
};

const HELP: &str = r#"四象限 CLI / MCP（与桌面应用共用本机任务）
four-quadrants --cli [--database /absolute/tasks.sqlite3] list [JSON]
four-quadrants --cli [--database /absolute/tasks.sqlite3] add JSON
four-quadrants --cli [--database /absolute/tasks.sqlite3] update JSON
four-quadrants --cli [--database /absolute/tasks.sqlite3] delete JSON
four-quadrants --mcp [--database /absolute/tasks.sqlite3]

add:    {"title":"准备方案","quadrant":"plan","id":"可选的唯一请求ID"}
update: {"id":"任务ID","title":"可选","quadrant":"plan","completed":true}
delete: {"id":"任务ID"}
list:   {"quadrant":"plan","completed":false,"query":"方案"}（所有筛选均可省略）
象限：do=重要且紧急，plan=重要不紧急，delegate=紧急不重要，eliminate=不重要不紧急。
成功时 stdout 输出 JSON；失败时 stderr 输出错误，退出码为 1。
MCP 使用 stdio，不监听网络端口，不需要启动桌面窗口。
"#;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct ListArgs {
    quadrant: Option<String>,
    completed: Option<bool>,
    query: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AddArgs {
    title: String,
    quadrant: String,
    id: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateArgs {
    id: String,
    title: Option<String>,
    quadrant: Option<String>,
    completed: Option<bool>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdArgs {
    id: String,
}

fn decode<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, String> {
    serde_json::from_value(value).map_err(|e| format!("参数不正确：{e}"))
}
fn execute(db: &TaskDatabase, action: &str, args: Value) -> Result<TaskSnapshot, String> {
    let change = match action {
        "list" => {
            let args: ListArgs = decode(args)?;
            if let Some(q) = &args.quadrant {
                crate::storage::validate_quadrant(q)?;
            }
            let mut snapshot = db.snapshot()?;
            snapshot.tasks.retain(|t| {
                args.quadrant.as_ref().is_none_or(|q| q == &t.quadrant)
                    && args.completed.is_none_or(|c| c == t.completed)
                    && args
                        .query
                        .as_ref()
                        .is_none_or(|q| t.title.to_lowercase().contains(&q.to_lowercase()))
            });
            snapshot
                .mindmaps
                .retain(|id, _| snapshot.tasks.iter().any(|task| &task.id == id));
            return Ok(snapshot);
        }
        "add" => {
            let args: AddArgs = decode(args)?;
            TaskChange::Add {
                task: Task {
                    id: args.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                    title: args.title,
                    quadrant: args.quadrant,
                    completed: false,
                },
            }
        }
        "update" => {
            let args: UpdateArgs = decode(args)?;
            if args.title.is_none() && args.quadrant.is_none() && args.completed.is_none() {
                return Err("请至少提供一个要修改的字段。".into());
            }
            TaskChange::Patch {
                id: args.id,
                title: args.title,
                quadrant: args.quadrant,
                completed: args.completed,
            }
        }
        "delete" => TaskChange::Remove {
            id: decode::<IdArgs>(args)?.id,
        },
        _ => return Err("未知操作，请使用 list / add / update / delete。".into()),
    };
    db.commit_external(&[change])
}

fn default_database() -> Result<PathBuf, String> {
    dirs::data_dir()
        .map(|p| p.join("com.local.fourquadrants/tasks.sqlite3"))
        .ok_or_else(|| "无法定位本机数据目录，请显式指定 --database。".into())
}
fn open_database(path: &Path) -> Result<TaskDatabase, String> {
    if !path.is_absolute() {
        return Err("--database 必须是绝对路径。".into());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    TaskDatabase::open(path)
}

/// None means an ordinary GUI launch. Headless modes run before Tauri's
/// single-instance plugin, so an already-open application cannot swallow calls.
pub fn entry() -> Option<Result<(), String>> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if !matches!(args.first().map(String::as_str), Some("--cli" | "--mcp")) {
        return None;
    }
    Some((|| {
        let mode = args.remove(0);
        if matches!(args.first().map(String::as_str), Some("--help" | "-h")) {
            print!("{HELP}");
            return Ok(());
        }
        let path = if args.first().map(String::as_str) == Some("--database") {
            if args.len() < 2 {
                return Err("--database 后需要提供绝对路径。".into());
            }
            args.remove(0);
            PathBuf::from(args.remove(0))
        } else {
            default_database()?
        };
        if mode == "--mcp" {
            if !args.is_empty() {
                return Err("--mcp 不接受额外参数。".into());
            }
            return serve(&path, io::stdin().lock(), io::stdout().lock());
        }
        if args.is_empty() || args.len() > 2 {
            return Err(HELP.into());
        }
        let action = args.remove(0);
        let value = serde_json::from_str(args.first().map(String::as_str).unwrap_or("{}"))
            .map_err(|e| format!("JSON 格式不正确：{e}"))?;
        let snapshot = execute(&open_database(&path)?, &action, value)?;
        println!(
            "{}",
            serde_json::to_string(&snapshot).map_err(|e| e.to_string())?
        );
        Ok(())
    })())
}

fn tools() -> Value {
    let quadrant = json!({"type":"string","enum":["do","plan","delegate","eliminate"],"description":"do=重要且紧急；plan=重要不紧急；delegate=紧急不重要；eliminate=不重要不紧急。根据用户意图选择，不清楚时询问用户。"});
    let title = json!({"type":"string","minLength":1,"maxLength":1000,"description":"任务标题，非空，最多 1000 个 UTF-16 代码单元。"});
    let id = json!({"type":"string","minLength":1,"description":"精确的任务 ID；修改或删除前通过 list_tasks 查询，不能用标题代替。"});
    let specs = [
        ("list_tasks", "查询本机四象限任务。写入前先查询以避免重复添加；省略筛选时返回全部任务及 revision。", json!({"quadrant":quadrant,"completed":{"type":"boolean"},"query":{"type":"string","description":"标题包含的文字，不区分大小写。"}}), json!([]), true, false),
        ("add_task", "把用户明确要做的事记入本机四象限。不要把随口讨论当作任务。可提供稳定唯一的 id，使相同内容重试不会重复创建；成功返回已保存的任务和 revision。", json!({"title":title,"quadrant":quadrant,"id":{"type":"string","minLength":1,"description":"可选的唯一请求 ID，用于相同新增请求的重试；不可复用给其他任务。"}}), json!(["title","quadrant"]), false, false),
        ("update_task", "按任务 ID 修改标题、象限或完成状态，只提交需要改变的字段。completed=true 表示完成，false 表示取消完成。至少提供一个修改字段。", json!({"id":id,"title":title,"quadrant":quadrant,"completed":{"type":"boolean"}}), json!(["id"]), false, true),
        ("delete_task", "按精确任务 ID 删除用户要求删除的任务。应先查询核对目标，不能根据相近标题猜测。", json!({"id":id}), json!(["id"]), false, true),
    ];
    Value::Array(specs.into_iter().map(|(name, description, properties, required, read_only, destructive)| json!({
        "name":name,"description":description,
        "inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},
        "annotations":{"readOnlyHint":read_only,"destructiveHint":destructive,"openWorldHint":false}
    })).collect())
}

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
fn handle(path: &Path, state: &mut u8, request: Value) -> Option<Value> {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let Some(method) = request.get("method").and_then(Value::as_str) else {
        return Some(rpc_error(id, -32600, "Invalid Request"));
    };
    if request.get("jsonrpc") != Some(&json!("2.0"))
        || !(id.is_string() || id.is_number() || id.is_null())
    {
        return Some(rpc_error(Value::Null, -32600, "Invalid Request"));
    }
    if request.get("id").is_none() {
        if method == "notifications/initialized" && *state == 1 {
            *state = 2;
        }
        return None;
    }
    let result = match method {
        "initialize" => {
            let params = &request["params"];
            let Some(version) = params["protocolVersion"].as_str() else {
                return Some(rpc_error(id, -32602, "Missing protocolVersion"));
            };
            if *state != 0
                || !params["capabilities"].is_object()
                || !params["clientInfo"]["name"].is_string()
                || !params["clientInfo"]["version"].is_string()
            {
                return Some(rpc_error(id, -32602, "Invalid initialization"));
            }
            let version =
                if ["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"].contains(&version) {
                    version
                } else {
                    "2025-11-25"
                };
            *state = 1;
            json!({"protocolVersion":version,"capabilities":{"tools":{}},"serverInfo":{"name":"four-quadrants","version":env!("CARGO_PKG_VERSION")},"instructions":"本机四象限任务接口。仅在用户明确要求记录或修改任务时写入；写入前先查询避免重复。象限不清楚时询问用户。成功结果来自正式 SQLite 事务，桌面窗口会自动刷新；不提供日期、提醒、账号或云同步。"})
        }
        "ping" => json!({}),
        _ if *state != 2 => return Some(rpc_error(id, -32002, "Server not initialized")),
        "tools/list" => json!({"tools":tools()}),
        "tools/call" => {
            let params = &request["params"];
            let action = match params["name"].as_str() {
                Some("list_tasks") => "list",
                Some("add_task") => "add",
                Some("update_task") => "update",
                Some("delete_task") => "delete",
                _ => return Some(rpc_error(id, -32602, "Unknown tool")),
            };
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            match open_database(path).and_then(|db| execute(&db, action, args)) {
                Ok(snapshot) => {
                    json!({"content":[{"type":"text","text":serde_json::to_string(&snapshot).unwrap()}],"structuredContent":snapshot,"isError":false})
                }
                Err(error) => json!({"content":[{"type":"text","text":error}],"isError":true}),
            }
        }
        _ => return Some(rpc_error(id, -32601, "Method not found")),
    };
    Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
}

fn serve(path: &Path, mut input: impl BufRead, mut output: impl Write) -> Result<(), String> {
    let mut state = 0;
    loop {
        let mut line = String::new();
        // Bound a single request to 1 MiB. stdout is exclusively JSON-RPC.
        let size = input
            .by_ref()
            .take(1_048_577)
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        if size == 0 {
            return Ok(());
        }
        if size > 1_048_576 {
            return Err("MCP 请求超过 1 MiB。".into());
        }
        let response = match serde_json::from_str(&line) {
            Ok(request) => handle(path, &mut state, request),
            Err(_) => Some(rpc_error(Value::Null, -32700, "Parse error")),
        };
        if let Some(response) = response {
            writeln!(output, "{response}")
                .and_then(|_| output.flush())
                .map_err(|e| e.to_string())?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_actions_validate_retry_and_only_patch_requested_fields() {
        let db = TaskDatabase::open(Path::new(":memory:")).unwrap();
        let add = json!({"id":"request-1","title":"准备方案","quadrant":"plan"});
        execute(&db, "add", add.clone()).unwrap();
        assert_eq!(execute(&db, "add", add).unwrap().tasks.len(), 1);
        assert!(execute(
            &db,
            "add",
            json!({"id":"request-1","title":"其他任务","quadrant":"do"})
        )
        .is_err());
        assert!(execute(&db, "update", json!({"id":"missing","completed":true})).is_err());
        assert!(execute(&db, "update", json!({"id":"request-1"})).is_err());
        assert!(execute(&db, "add", json!({"title":" ","quadrant":"do"})).is_err());
        assert!(execute(
            &db,
            "add",
            json!({"title":"😀".repeat(501),"quadrant":"do"})
        )
        .is_err());
        assert!(execute(&db, "add", json!({"title":"x","quadrant":"bad"})).is_err());
        assert!(execute(
            &db,
            "add",
            json!({"title":"x","quadrant":"do","deadline":"today"})
        )
        .is_err());
        assert!(execute(&db, "list", json!({"quadrant":"bad"})).is_err());
        let result = execute(&db, "update", json!({"id":"request-1","completed":true})).unwrap();
        assert_eq!(result.tasks[0].title, "准备方案");
        assert_eq!(result.tasks[0].quadrant, "plan");
        assert!(result.tasks[0].completed);
        assert!(execute(&db, "list", json!({"completed":false}))
            .unwrap()
            .tasks
            .is_empty());
        assert_eq!(
            execute(&db, "list", json!({"query":"方案"}))
                .unwrap()
                .tasks
                .len(),
            1
        );
        assert!(execute(&db, "delete", json!({"id":"request-1"}))
            .unwrap()
            .tasks
            .is_empty());
    }
    #[test]
    fn stdio_handshake_errors_notifications_and_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.sqlite3");
        let input = [
            "bad json".to_string(),
            json!({"jsonrpc":"2.0","id":0,"method":"tools/list"}).to_string(),
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}).to_string(),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}).to_string(),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}).to_string(),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"add_task","arguments":{"id":"test","title":"测试","quadrant":"plan"}}}).to_string(),
            json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"update_task","arguments":{"id":"missing","completed":true}}}).to_string(),
            json!({"jsonrpc":"2.0","id":5,"method":"unknown"}).to_string(),
            json!({"jsonrpc":"2.0","id":6,"method":"ping"}).to_string(),
        ].join("\n") + "\n";
        let mut output = Vec::new();
        serve(&path, input.as_bytes(), &mut output).unwrap();
        let replies: Vec<Value> = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        assert_eq!(replies.len(), 8); // initialized notification has no reply
        assert_eq!(replies[0]["error"]["code"], -32700);
        assert_eq!(replies[1]["error"]["code"], -32002);
        assert_eq!(replies[3]["result"]["tools"].as_array().unwrap().len(), 4);
        assert_eq!(
            replies[4]["result"]["structuredContent"]["tasks"][0]["id"],
            "test"
        );
        assert_eq!(replies[5]["result"]["isError"], true);
        assert_eq!(replies[6]["error"]["code"], -32601);
        assert_eq!(
            TaskDatabase::open(&path)
                .unwrap()
                .snapshot()
                .unwrap()
                .tasks
                .len(),
            1
        );
    }
}
