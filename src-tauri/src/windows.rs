use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct WidgetPreferences {
    pub visible: bool,
    pub pinned: bool,
}
pub struct Preferences(pub Mutex<WidgetPreferences>);
fn preferences_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("widget.json"))
}
pub fn load_preferences(app: &AppHandle) -> WidgetPreferences {
    preferences_path(app)
        .ok()
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|data| serde_json::from_slice(&data).ok())
        .unwrap_or_default()
}
fn persist(app: &AppHandle, preferences: &WidgetPreferences) -> Result<(), String> {
    let path = preferences_path(app)?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(
        &temporary,
        serde_json::to_vec(preferences).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    std::fs::rename(temporary, path).map_err(|e| e.to_string())
}

// Native AppKit public API: keep the system's rounded, resizable window and shadow,
// hide only the three standard buttons in the compact window.
#[cfg(target_os = "macos")]
pub fn prepare_widget(window: &WebviewWindow) -> tauri::Result<()> {
    let pointer = window.ns_window()? as usize;
    window.run_on_main_thread(move || {
        let ns_window = unsafe { &*(pointer as *const objc2_app_kit::NSWindow) };
        for kind in [
            objc2_app_kit::NSWindowButton::CloseButton,
            objc2_app_kit::NSWindowButton::MiniaturizeButton,
            objc2_app_kit::NSWindowButton::ZoomButton,
        ] {
            if let Some(button) = ns_window.standardWindowButton(kind) {
                button.setHidden(true);
            }
        }
    })
}
#[cfg(not(target_os = "macos"))]
pub fn prepare_widget(_: &WebviewWindow) -> tauri::Result<()> {
    Ok(())
}

// Clamp the whole window to a connected display's work area. Recheck on show,
// so disconnecting an external display cannot strand the drag handle offscreen.
pub fn ensure_visible(window: &WebviewWindow) -> tauri::Result<()> {
    let monitors = window.available_monitors()?;
    let position = window.outer_position()?;
    let size = window.outer_size()?;
    let monitor = monitors
        .iter()
        .find(|m| {
            let a = m.work_area();
            position.x >= a.position.x
                && position.y >= a.position.y
                && position.x < a.position.x + a.size.width as i32
                && position.y < a.position.y + a.size.height as i32
        })
        .or_else(|| monitors.first());
    if let Some(monitor) = monitor {
        let area = monitor.work_area();
        let width = size.width.min(area.size.width);
        let height = size.height.min(area.size.height);
        if width != size.width || height != size.height {
            window.set_size(tauri::PhysicalSize::new(width, height))?;
        }
        let x = position.x.clamp(
            area.position.x,
            area.position.x + (area.size.width - width) as i32,
        );
        let y = position.y.clamp(
            area.position.y,
            area.position.y + (area.size.height - height) as i32,
        );
        window.set_position(PhysicalPosition::new(x, y))?;
    }
    Ok(())
}
#[tauri::command]
pub fn show_main(app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("主窗口未打开")?;
    ensure_visible(&window).map_err(|e| e.to_string())?;
    window
        .unminimize()
        .and_then(|_| window.show())
        .and_then(|_| window.set_focus())
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn show_widget(app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("widget").ok_or("小组件未打开")?;
    ensure_visible(&window).map_err(|e| e.to_string())?;
    let mut preferences = app
        .state::<Preferences>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    window
        .set_always_on_top(preferences.pinned)
        .map_err(|e| e.to_string())?;
    window
        .show()
        .and_then(|_| window.set_focus())
        .map_err(|e| e.to_string())?;
    let _ = window.emit("widget-shown", ());
    preferences.visible = true;
    persist(&app, &preferences)?;
    *app.state::<Preferences>()
        .0
        .lock()
        .map_err(|e| e.to_string())? = preferences;
    Ok(())
}
#[tauri::command]
pub fn hide_window(window: WebviewWindow, app: AppHandle) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())?;
    if window.label() == "widget" {
        let mut preferences = app
            .state::<Preferences>()
            .0
            .lock()
            .map_err(|e| e.to_string())?
            .clone();
        preferences.visible = false;
        persist(&app, &preferences)?;
        *app.state::<Preferences>()
            .0
            .lock()
            .map_err(|e| e.to_string())? = preferences;
    }
    app.save_window_state(StateFlags::POSITION | StateFlags::SIZE)
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn widget_preferences(app: AppHandle) -> WidgetPreferences {
    app.state::<Preferences>().0.lock().unwrap().clone()
}
#[tauri::command]
pub fn set_widget_pinned(app: AppHandle, pinned: bool) -> Result<(), String> {
    let window = app.get_webview_window("widget").ok_or("小组件未打开")?;
    window
        .set_always_on_top(pinned)
        .map_err(|e| e.to_string())?;
    let mut preferences = app
        .state::<Preferences>()
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    preferences.pinned = pinned;
    persist(&app, &preferences)?;
    *app.state::<Preferences>()
        .0
        .lock()
        .map_err(|e| e.to_string())? = preferences;
    Ok(())
}
