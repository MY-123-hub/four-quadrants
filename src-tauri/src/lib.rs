mod storage;
use std::sync::atomic::{AtomicBool, Ordering};
use storage::{Task, TaskDatabase};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{Emitter, Manager};
struct ExitGuard(AtomicBool);

#[tauri::command]
fn load_tasks(
    database: tauri::State<'_, Result<TaskDatabase, String>>,
) -> Result<Vec<Task>, String> {
    database.inner().as_ref().map_err(Clone::clone)?.load()
}
#[tauri::command]
fn save_tasks(
    tasks: Vec<Task>,
    database: tauri::State<'_, Result<TaskDatabase, String>>,
) -> Result<(), String> {
    database
        .inner()
        .as_ref()
        .map_err(Clone::clone)?
        .save(&tasks)
}
#[tauri::command]
fn finish_exit(app: tauri::AppHandle) {
    app.state::<ExitGuard>().0.store(true, Ordering::SeqCst);
    app.exit(0);
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            app.manage(ExitGuard(AtomicBool::new(false)));
            let quit = MenuItem::with_id(app, "quit", "退出四象限", true, Some("CmdOrCtrl+Q"))?;
            let application_menu = Submenu::with_items(
                app,
                "四象限",
                true,
                &[
                    &PredefinedMenuItem::hide(app, Some("隐藏四象限"))?,
                    &PredefinedMenuItem::hide_others(app, Some("隐藏其他"))?,
                    &PredefinedMenuItem::show_all(app, Some("显示全部"))?,
                    &PredefinedMenuItem::separator(app)?,
                    &quit,
                ],
            )?;
            let edit_menu = Submenu::with_items(
                app,
                "编辑",
                true,
                &[
                    &PredefinedMenuItem::undo(app, Some("撤销"))?,
                    &PredefinedMenuItem::redo(app, Some("重做"))?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::cut(app, Some("剪切"))?,
                    &PredefinedMenuItem::copy(app, Some("复制"))?,
                    &PredefinedMenuItem::paste(app, Some("粘贴"))?,
                    &PredefinedMenuItem::select_all(app, Some("全选"))?,
                ],
            )?;
            let window_menu = Submenu::with_items(
                app,
                "窗口",
                true,
                &[
                    &PredefinedMenuItem::minimize(app, Some("最小化"))?,
                    &PredefinedMenuItem::maximize(app, Some("缩放"))?,
                    &PredefinedMenuItem::close_window(app, Some("关闭窗口"))?,
                ],
            )?;
            app.set_menu(Menu::with_items(
                app,
                &[&application_menu, &edit_menu, &window_menu],
            )?)?;
            let database = (|| {
                let path = app.path().app_data_dir().map_err(|e| e.to_string())?;
                std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
                TaskDatabase::open(&path.join("tasks.sqlite3"))
            })();
            app.manage(database);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_tasks,
            save_tasks,
            finish_exit
        ])
        .on_menu_event(|app, event| {
            if event.id().as_ref() == "quit" {
                let _ = app.emit("request-shutdown", ());
            }
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.state::<ExitGuard>().0.load(Ordering::SeqCst) {
                    return;
                }
                api.prevent_close();
                let _ = window.emit("request-shutdown", ());
            }
        })
        .build(tauri::generate_context!())
        .expect("无法启动四象限")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                // Only finish_exit after successful frontend flush permits termination.
                if !app.state::<ExitGuard>().0.load(Ordering::SeqCst) {
                    api.prevent_exit();
                    let _ = app.emit("request-shutdown", ());
                }
            }
        });
}
