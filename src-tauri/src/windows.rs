use tauri::{AppHandle, Manager, PhysicalPosition, WebviewWindow};
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

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
pub fn hide_window(window: WebviewWindow, app: AppHandle) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())?;
    app.save_window_state(StateFlags::POSITION | StateFlags::SIZE)
        .map_err(|e| e.to_string())
}
