mod storage;
mod windows;
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use storage::{TaskChange, TaskDatabase, TaskSnapshot};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{Emitter, Manager};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};
struct ExitGuard {
    allowed: AtomicBool,
    waiting: Mutex<(u64, Option<HashSet<String>>)>,
}

#[tauri::command]
fn load_tasks(
    database: tauri::State<'_, Result<TaskDatabase, String>>,
) -> Result<TaskSnapshot, String> {
    database.inner().as_ref().map_err(Clone::clone)?.snapshot()
}
#[tauri::command]
fn commit_tasks(
    changes: Vec<TaskChange>,
    database: tauri::State<'_, Result<TaskDatabase, String>>,
    app: tauri::AppHandle,
) -> Result<TaskSnapshot, String> {
    let snapshot = database
        .inner()
        .as_ref()
        .map_err(Clone::clone)?
        .commit(&changes)?;
    // Revision lets listeners reject delayed broadcasts and replies.
    let _ = app.emit("tasks-updated", &snapshot);
    Ok(snapshot)
}
fn request_exit(app: &tauri::AppHandle) {
    let guard = app.state::<ExitGuard>();
    let mut waiting = guard.waiting.lock().unwrap();
    if waiting.1.is_some() {
        return;
    }
    waiting.0 += 1;
    waiting.1 = Some(app.webview_windows().keys().cloned().collect());
    let exit_id = waiting.0;
    drop(waiting);
    let _ = app.emit("request-shutdown", exit_id);
}
#[tauri::command]
fn finish_exit(app: tauri::AppHandle, window: tauri::WebviewWindow, success: bool, exit_id: u64) {
    let guard = app.state::<ExitGuard>();
    let mut waiting = guard.waiting.lock().unwrap();
    if waiting.0 != exit_id || waiting.1.is_none() {
        return;
    }
    if !success {
        waiting.1 = None;
        let _ = app.emit("shutdown-cancelled", ());
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    if let Some(labels) = waiting.1.as_mut() {
        labels.remove(window.label());
        if labels.is_empty() {
            let _ = app.save_window_state(StateFlags::POSITION | StateFlags::SIZE);
            guard.allowed.store(true, Ordering::SeqCst);
            app.exit(0);
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::POSITION | StateFlags::SIZE)
                .build(),
        )
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            app.manage(ExitGuard {
                allowed: AtomicBool::new(false),
                waiting: Mutex::new((0, None)),
            });
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
            let widget = MenuItem::with_id(
                app,
                "show-widget",
                "显示悬浮小组件",
                true,
                Some("CmdOrCtrl+Shift+W"),
            )?;
            let main = MenuItem::with_id(
                app,
                "show-main",
                "打开主窗口",
                true,
                Some("CmdOrCtrl+Shift+M"),
            )?;
            let window_menu = Submenu::with_items(
                app,
                "窗口",
                true,
                &[
                    &PredefinedMenuItem::minimize(app, Some("最小化"))?,
                    &PredefinedMenuItem::maximize(app, Some("缩放"))?,
                    &PredefinedMenuItem::close_window(app, Some("关闭窗口"))?,
                    &PredefinedMenuItem::separator(app)?,
                    &widget,
                    &main,
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
            let preferences = windows::load_preferences(app.handle());
            app.manage(windows::Preferences(Mutex::new(preferences.clone())));
            if let Some(widget) = app.get_webview_window("widget") {
                windows::prepare_widget(&widget)?;
            }
            if preferences.visible {
                windows::show_widget(app.handle().clone()).map_err(std::io::Error::other)?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_tasks,
            commit_tasks,
            finish_exit,
            windows::show_main,
            windows::show_widget,
            windows::hide_window,
            windows::widget_preferences,
            windows::set_widget_pinned
        ])
        .on_menu_event(|app, event| match event.id().as_ref() {
            "quit" => request_exit(app),
            "show-widget" => {
                let _ = windows::show_widget(app.clone());
            }
            "show-main" => {
                let _ = windows::show_main(app.clone());
            }
            _ => {}
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.state::<ExitGuard>().allowed.load(Ordering::SeqCst) {
                    return;
                }
                api.prevent_close();
                let _ = window.emit_to(window.label(), "request-hide", ());
            }
        })
        .build(tauri::generate_context!())
        .expect("无法启动四象限")
        .run(|app, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                if !app.state::<ExitGuard>().allowed.load(Ordering::SeqCst) {
                    api.prevent_exit();
                    request_exit(app);
                }
            }
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => {
                let _ = windows::show_main(app.clone());
            }
            _ => {}
        });
}
