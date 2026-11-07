//! Lightweight WebView / Canvas App Sandbox (VISION.md §4).
//!
//! Provides a sandboxed webview wallpaper layer for HTML5 Canvas, WebGL, and TypeScript apps.
//! Mounts a secure custom protocol (`rookery-app://`) that serves local app assets with path
//! traversal validation, manages the lifecycle of transparent desktop-level WebviewWindows,
//! and dispatches DOM CustomEvents for popover controls and direct interaction.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use rookery_core::interaction::InteractionEvent;
use rookery_platform::DisplayInfo;
use tauri::http::{Request, Response, StatusCode, header};
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};

use crate::state::HostState;

static APPS_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Initialize the global apps directory for custom protocol resolution.
pub fn init_apps_dir(dir: PathBuf) {
    let _ = APPS_DIR.set(dir);
}

/// Retrieve the resolved apps directory, falling back to local `apps` folder.
pub fn get_apps_dir() -> PathBuf {
    APPS_DIR
        .get()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("apps"))
}

/// Check if an app is a Web Canvas app (contains `index.html`).
pub fn is_web_canvas_app(app_id: &str, apps_dir: Option<&Path>) -> bool {
    let base = match apps_dir {
        Some(p) => p.to_path_buf(),
        None => get_apps_dir(),
    };
    base.join(app_id).join("index.html").is_file()
}

/// Map a display ID to a valid Tauri window label.
pub fn webview_wallpaper_label(display_id: &str) -> String {
    let sanitized: String = display_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("webview-wallpaper-{sanitized}")
}

/// Construct the custom protocol URL for a web canvas app.
pub fn app_webview_url(app_id: &str) -> Result<tauri::Url, tauri::Url> {
    #[cfg(windows)]
    let raw = format!("http://rookery-app.localhost/{app_id}/index.html");
    #[cfg(not(windows))]
    let raw = format!("rookery-app://localhost/{app_id}/index.html");

    tauri::Url::parse(&raw).map_err(|_| tauri::Url::parse("about:blank").unwrap())
}

/// Custom URI scheme protocol handler for `rookery-app://`.
pub fn handle_custom_protocol(request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    handle_custom_protocol_with_app(None, request)
}

/// Custom URI scheme protocol handler with optional `AppHandle` for live SDK telemetry & state persistence.
pub fn handle_custom_protocol_with_app(
    app: Option<&AppHandle>,
    request: &Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let path = request.uri().path().trim_start_matches('/');
    let mut parts = path.splitn(2, '/');
    let app_id = parts.next().unwrap_or("");
    let rel_path = parts.next().unwrap_or("index.html");
    let file_subpath = if rel_path.is_empty() {
        "index.html"
    } else {
        rel_path
    };

    // Handle sandboxed SDK bridge endpoints (`/__sdk/stats` and `/__sdk/state`)
    if let Some(sdk_endpoint) = file_subpath.strip_prefix("__sdk/") {
        return handle_sdk_endpoint(app, app_id, sdk_endpoint, request);
    }

    let apps_dir = get_apps_dir();
    let app_dir = apps_dir.join(app_id);
    let target = app_dir.join(file_subpath);

    // Security guard: verify target canonical path starts with canonical app_dir to prevent directory traversal
    if let (Ok(canon_app), Ok(canon_target)) = (app_dir.canonicalize(), target.canonicalize())
        && canon_target.starts_with(&canon_app)
        && canon_target.is_file()
        && let Ok(bytes) = std::fs::read(&canon_target)
    {
        let mime = mime_for_path(&canon_target);
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .header(header::CACHE_CONTROL, "no-cache")
            .body(bytes)
            .unwrap_or_else(|_| not_found_response());
    }

    not_found_response()
}

fn handle_sdk_endpoint(
    app: Option<&AppHandle>,
    app_id: &str,
    endpoint: &str,
    request: &Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    // Validate app_id is safe identifier
    if app_id.is_empty()
        || !app_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return not_found_response();
    }

    match endpoint {
        "stats" => {
            if let Some(app_handle) = app
                && let Ok(parsed) = serde_json::from_slice::<
                    std::collections::BTreeMap<String, serde_json::Value>,
                >(request.body())
            {
                let mut string_stats = std::collections::BTreeMap::new();
                for (k, v) in parsed {
                    let s = match v {
                        serde_json::Value::String(s) => s,
                        other => other.to_string(),
                    };
                    string_stats.insert(k, s);
                }
                let state = app_handle.state::<HostState>();
                state.catalog_state.set_stats(app_id, string_stats);
                state.emit_status_if_popover_open(app_handle);
            }
            json_ok_response(b"{\"ok\":true}".to_vec())
        }
        "state" => {
            let state_path = app_state_file_path(app, app_id);
            if request.method() == "POST" {
                let body = request.body();
                if body.len() <= 512 * 1024
                    && serde_json::from_slice::<serde_json::Value>(body).is_ok()
                {
                    if let Some(parent) = state_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(&state_path, body);
                }
                json_ok_response(b"{\"ok\":true}".to_vec())
            } else {
                let bytes = std::fs::read(&state_path).unwrap_or_else(|_| b"{}".to_vec());
                json_ok_response(bytes)
            }
        }
        _ => not_found_response(),
    }
}

fn app_state_file_path(app: Option<&AppHandle>, app_id: &str) -> PathBuf {
    if let Some(app_handle) = app {
        let state = app_handle.state::<HostState>();
        if let Some(parent) = state.settings_path.parent() {
            return parent.join("apps_state").join(format!("{app_id}.json"));
        }
    }
    get_apps_dir()
        .join(".state")
        .join(format!("{app_id}.json"))
}

fn json_ok_response(body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(body)
        .unwrap_or_else(|_| not_found_response())
}

/// Generate the `window.Rookery` client SDK initialization script injected into Web Canvas apps.
pub fn sdk_init_script(app_id: &str) -> String {
    let escaped = app_id.replace('\\', "\\\\").replace('\'', "\\'");
    format!(
        r#"(function() {{
  const appId = '{escaped}';
  const sdkBase = window.location.origin + '/' + appId + '/__sdk';
  window.Rookery = {{
    appId,
    onAction(cb) {{
      window.addEventListener('rookery-action', (e) => cb(e.detail?.action, e.detail));
    }},
    onControl(cb) {{
      window.addEventListener('rookery-control', (e) => cb(e.detail?.id, e.detail?.value));
    }},
    onInteractive(cb) {{
      window.addEventListener('rookery-interactive', (e) => cb(!!e.detail?.interactive));
    }},
    onWake(cb) {{
      window.addEventListener('rookery-wake', (e) => cb(Number(e.detail?.elapsedSeconds || 0)));
    }},
    async setStats(stats) {{
      try {{
        await fetch(sdkBase + '/stats', {{
          method: 'POST',
          headers: {{ 'Content-Type': 'application/json' }},
          body: JSON.stringify(stats || {{}})
        }});
      }} catch {{}}
    }},
    async saveState(state) {{
      try {{
        await fetch(sdkBase + '/state', {{
          method: 'POST',
          headers: {{ 'Content-Type': 'application/json' }},
          body: JSON.stringify(state ?? {{}})
        }});
      }} catch {{}}
    }},
    async loadState() {{
      try {{
        const res = await fetch(sdkBase + '/state');
        if (res.ok) return await res.json();
      }} catch {{}}
      return null;
    }}
  }};
}})();"#
    )
}

fn not_found_response() -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(b"404 Not Found".to_vec())
        .unwrap()
}

fn mime_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|s| s.to_str()).unwrap_or("") {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "wasm" => "application/wasm",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        _ => "application/octet-stream",
    }
}

/// Reconcile webview wallpaper windows on the main thread.
pub fn update_webview_wallpaper(app: &AppHandle, active_app: &str, apps_dir: Option<&Path>) {
    let is_canvas = is_web_canvas_app(active_app, apps_dir);
    let state = app.state::<HostState>();

    if !is_canvas {
        // Destroy all webview wallpaper windows to free WebKit / WebView2 memory
        let mut destroyed_any = false;
        for (label, window) in app.webview_windows() {
            if label.starts_with("webview-wallpaper-") {
                destroyed_any = true;
                let _ = window.destroy();
            }
        }
        // Ensure native windows are visible only if we previously hid them for a web canvas app
        if destroyed_any && let Ok(inner) = state.inner.lock() {
            for d in inner.displays.values() {
                let is_vis = d.window.is_visible().unwrap_or(false);
                if !is_vis {
                    let _ = d.window.show();
                }
                let _ = state.layer.demote_wallpaper(&d.window, &d.info);
            }
        }
        return;
    }

    // Web canvas app is active: hide native bare windows and display webview wallpaper windows
    let url = match app_webview_url(active_app) {
        Ok(u) => u,
        Err(u) => u,
    };

    let Ok(inner) = state.inner.lock() else {
        return;
    };

    for d in inner.displays.values() {
        let _ = d.window.hide();

        let init_script = sdk_init_script(active_app);
        let label = webview_wallpaper_label(&d.info.id.0);
        if let Some(existing) = app.get_webview_window(&label) {
            let _ = existing.navigate(url.clone());
            let _ = existing.eval(&init_script);
            let _ = existing.show();
            continue;
        }

        let logical_w = d.info.width_px as f64 / d.info.scale_factor;
        let logical_h = d.info.height_px as f64 / d.info.scale_factor;

        let window =
            match WebviewWindowBuilder::new(app, &label, WebviewUrl::CustomProtocol(url.clone()))
                .title(format!("Rookery Web Canvas - {}", d.info.name))
                .initialization_script(&init_script)
                .decorations(false)
                .resizable(false)
                .skip_taskbar(true)
                .shadow(false)
                .focused(false)
                .visible(true)
                .transparent(true)
                .inner_size(logical_w, logical_h)
                .build()
            {
                Ok(w) => w,
                Err(e) => {
                    log::error!(
                        "failed to create webview wallpaper window for {}: {e}",
                        d.info.id
                    );
                    continue;
                }
            };

        if let Ok(pos) = d.window.outer_position() {
            let _ = window.set_position(pos);
        } else {
            let _ = window.set_position(PhysicalPosition::new(
                d.info.bounds_native.x as i32,
                d.info.bounds_native.y as i32,
            ));
        }

        if let Ok(sz) = d.window.inner_size() {
            let _ = window.set_size(sz);
        } else {
            let _ = window.set_size(PhysicalSize::new(d.info.width_px, d.info.height_px));
        }

        if let Err(e) = state.layer.attach_wallpaper(&window, &d.info) {
            log::error!("failed to attach webview wallpaper on {}: {e:#}", d.info.id);
        }

        let app_for_ev = app.clone();
        window.on_window_event(move |ev| {
            if let WindowEvent::Focused(false) = ev {
                let st = app_for_ev.state::<HostState>();
                st.apply_interaction_event(&app_for_ev, InteractionEvent::FocusLost);
            }
        });
    }
}

/// Promote or demote webview wallpaper windows during direct interaction transitions.
pub fn set_interactive_webviews(app: &AppHandle, promote: bool, info: &DisplayInfo) {
    let label = webview_wallpaper_label(&info.id.0);
    if let Some(win) = app.get_webview_window(&label) {
        let state = app.state::<HostState>();
        if promote {
            let _ = state.layer.promote_interactive(&win, info);
            let _ = win.set_focus();
            let _ = win.eval("window.dispatchEvent(new CustomEvent('rookery-interactive', { detail: { interactive: true } }));");
        } else {
            let _ = state.layer.demote_wallpaper(&win, info);
            let _ = win.eval("window.dispatchEvent(new CustomEvent('rookery-interactive', { detail: { interactive: false } }));");
        }
    }
}

/// Broadcast an action event to all active webview wallpaper windows.
pub fn dispatch_action(app: &AppHandle, action: &str) {
    let escaped = action.replace('\\', "\\\\").replace('\'', "\\'");
    let script = format!(
        "window.dispatchEvent(new CustomEvent('rookery-action', {{ detail: {{ action: '{}' }} }}));",
        escaped
    );
    for (label, win) in app.webview_windows() {
        if label.starts_with("webview-wallpaper-") {
            let _ = win.eval(&script);
        }
    }
}

/// Broadcast a control change event to all active webview wallpaper windows.
pub fn dispatch_control(app: &AppHandle, control_id: &str, value: &serde_json::Value) {
    let val_json = serde_json::to_string(value).unwrap_or_else(|_| "null".into());
    let escaped_id = control_id.replace('\\', "\\\\").replace('\'', "\\'");
    let script = format!(
        "window.dispatchEvent(new CustomEvent('rookery-control', {{ detail: {{ id: '{}', value: {} }} }}));",
        escaped_id, val_json
    );
    for (label, win) in app.webview_windows() {
        if label.starts_with("webview-wallpaper-") {
            let _ = win.eval(&script);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_sanitization() {
        assert_eq!(
            webview_wallpaper_label("display-1:0@120"),
            "webview-wallpaper-display-1-0-120"
        );
        assert_eq!(
            webview_wallpaper_label("My Display (Main)"),
            "webview-wallpaper-My-Display--Main-"
        );
    }

    #[test]
    fn custom_protocol_url_generation() {
        let url = app_webview_url("aurora-waves").expect("valid url");
        assert_eq!(url.path(), "/aurora-waves/index.html");
        #[cfg(not(windows))]
        assert_eq!(url.scheme(), "rookery-app");
        #[cfg(windows)]
        assert_eq!(url.scheme(), "http");
    }

    #[test]
    fn mime_mapping() {
        assert_eq!(
            mime_for_path(Path::new("index.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            mime_for_path(Path::new("app.js")),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(
            mime_for_path(Path::new("style.css")),
            "text/css; charset=utf-8"
        );
        assert_eq!(mime_for_path(Path::new("data.json")), "application/json");
        assert_eq!(mime_for_path(Path::new("canvas.wasm")), "application/wasm");
        assert_eq!(mime_for_path(Path::new("image.png")), "image/png");
        assert_eq!(mime_for_path(Path::new("vector.svg")), "image/svg+xml");
    }

    #[test]
    fn custom_protocol_security_and_serving() {
        let temp = tempfile::tempdir().unwrap();
        let app_dir = temp.path().join("demo-app");
        std::fs::create_dir_all(&app_dir).unwrap();
        std::fs::write(
            app_dir.join("index.html"),
            "<html><body>Hello Canvas</body></html>",
        )
        .unwrap();
        init_apps_dir(temp.path().to_path_buf());

        // Valid request
        let req = Request::builder()
            .uri("rookery-app://localhost/demo-app/index.html")
            .body(Vec::new())
            .unwrap();
        let res = handle_custom_protocol(&req);
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
        assert_eq!(res.body(), b"<html><body>Hello Canvas</body></html>");

        // Attempt directory traversal
        let malicious_req = Request::builder()
            .uri("rookery-app://localhost/demo-app/../../outside.txt")
            .body(Vec::new())
            .unwrap();
        let bad_res = handle_custom_protocol(&malicious_req);
        assert_eq!(bad_res.status(), StatusCode::NOT_FOUND);

        // Non-existent file
        let missing_req = Request::builder()
            .uri("rookery-app://localhost/demo-app/missing.js")
            .body(Vec::new())
            .unwrap();
        let missing_res = handle_custom_protocol(&missing_req);
        assert_eq!(missing_res.status(), StatusCode::NOT_FOUND);

        // SDK state round-trip
        let post_state = Request::builder()
            .method("POST")
            .uri("rookery-app://localhost/demo-app/__sdk/state")
            .body(b"{\"score\":42}".to_vec())
            .unwrap();
        let post_res = handle_custom_protocol(&post_state);
        assert_eq!(post_res.status(), StatusCode::OK);

        let get_state = Request::builder()
            .method("GET")
            .uri("rookery-app://localhost/demo-app/__sdk/state")
            .body(Vec::new())
            .unwrap();
        let get_res = handle_custom_protocol(&get_state);
        assert_eq!(get_res.status(), StatusCode::OK);
        assert_eq!(get_res.body(), b"{\"score\":42}");
    }
}
