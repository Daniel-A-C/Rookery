//! Lightweight WebView / Canvas App Sandbox (VISION.md §4).
//!
//! Provides a sandboxed webview wallpaper layer for HTML5 Canvas, WebGL, and TypeScript apps.
//! Mounts a secure custom protocol (`rookery-app://`) that serves local app assets with path
//! traversal validation, manages the lifecycle of transparent desktop-level WebviewWindows,
//! and dispatches DOM CustomEvents for popover controls and direct interaction.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use rookery_core::interaction::InteractionEvent;
use rookery_platform::DisplayInfo;
use tauri::http::{Request, Response, StatusCode, header};
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};

use crate::state::HostState;

static APPS_DIR: OnceLock<PathBuf> = OnceLock::new();
static LOADED_WEBVIEW_APPS: OnceLock<Mutex<BTreeMap<String, String>>> = OnceLock::new();

fn loaded_webview_apps() -> &'static Mutex<BTreeMap<String, String>> {
    LOADED_WEBVIEW_APPS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Clear tracked webview app IDs so the next `update_webview_wallpaper` call forces a reload.
pub fn clear_loaded_webview_cache() {
    if let Ok(mut map) = loaded_webview_apps().lock() {
        map.clear();
    }
}

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

type StatsCallback<'a> = &'a dyn Fn(&str, BTreeMap<String, String>);

/// Custom URI scheme protocol handler for `rookery-app://`.
pub fn handle_custom_protocol(request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    handle_custom_protocol_inner(None, None, request)
}

/// Custom URI scheme protocol handler with optional `AppHandle` for live SDK telemetry & state persistence.
pub fn handle_custom_protocol_with_app(
    app: Option<&AppHandle>,
    request: &Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let Some(app_handle) = app else {
        return handle_custom_protocol_inner(None, None, request);
    };
    let state = app_handle.state::<HostState>();
    let state_dir = state.settings_path.parent().map(|p| p.join("apps_state"));
    let on_stats = |app_id: &str, string_stats: BTreeMap<String, String>| {
        state.catalog_state.set_stats(app_id, string_stats);
        state.emit_status_if_popover_open(app_handle);
    };
    handle_custom_protocol_inner(state_dir.as_deref(), Some(&on_stats), request)
}

fn handle_custom_protocol_inner(
    state_dir: Option<&Path>,
    on_stats: Option<StatsCallback<'_>>,
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
        return handle_sdk_endpoint(state_dir, on_stats, app_id, sdk_endpoint, request);
    }

    let apps_dir = get_apps_dir();
    let app_dir = apps_dir.join(app_id);
    let decoded_subpath = percent_decode(file_subpath);
    let target = app_dir.join(&decoded_subpath);

    // Security guard: verify target canonical path starts with canonical app_dir to prevent directory traversal
    if let (Ok(canon_app), Ok(canon_target)) = (app_dir.canonicalize(), target.canonicalize())
        && canon_target.starts_with(&canon_app)
        && canon_target.is_file()
    {
        return serve_static_file(&canon_target, request);
    }

    not_found_response()
}

fn serve_static_file(path: &Path, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};

    let Ok(mut file) = std::fs::File::open(path) else {
        return not_found_response();
    };
    let Ok(meta) = file.metadata() else {
        return not_found_response();
    };
    let total_len = meta.len();
    let mime = mime_for_path(path);
    let cache_control = if mime.starts_with("image/") || mime.starts_with("video/") {
        "public, max-age=3600"
    } else {
        "no-cache"
    };

    if let Some(range_header) = request
        .headers()
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        && let Some((start, end)) = parse_byte_range(range_header, total_len)
    {
        // Cap single range chunk to 2 MiB to keep memory & latency bounded for large videos
        const MAX_CHUNK: u64 = 2 * 1024 * 1024;
        let clamped_end = end
            .min(start.saturating_add(MAX_CHUNK - 1))
            .min(total_len - 1);
        let len = clamped_end.saturating_sub(start) + 1;
        let mut buf = vec![0u8; len as usize];
        if file.seek(SeekFrom::Start(start)).is_ok() && file.read_exact(&mut buf).is_ok() {
            return Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(header::CONTENT_TYPE, mime)
                .header(header::ACCEPT_RANGES, "bytes")
                .header(
                    header::CONTENT_RANGE,
                    format!("bytes {start}-{clamped_end}/{total_len}"),
                )
                .header(header::CONTENT_LENGTH, len.to_string())
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .header(header::CACHE_CONTROL, cache_control)
                .body(buf)
                .unwrap_or_else(|_| not_found_response());
        }
    }

    let Ok(bytes) = std::fs::read(path) else {
        return not_found_response();
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, bytes.len().to_string())
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::CACHE_CONTROL, cache_control)
        .body(bytes)
        .unwrap_or_else(|_| not_found_response())
}

fn parse_byte_range(header_val: &str, total_len: u64) -> Option<(u64, u64)> {
    if total_len == 0 {
        return None;
    }
    let spec = header_val.strip_prefix("bytes=")?.split(',').next()?.trim();
    let (start_str, end_str) = spec.split_once('-')?;
    if start_str.is_empty() {
        let suffix: u64 = end_str.parse().ok()?;
        if suffix == 0 {
            return None;
        }
        let start = total_len.saturating_sub(suffix);
        return Some((start, total_len - 1));
    }
    let start: u64 = start_str.parse().ok()?;
    if start >= total_len {
        return None;
    }
    let end = if end_str.is_empty() {
        total_len - 1
    } else {
        end_str.parse::<u64>().ok()?.min(total_len - 1)
    };
    if start > end {
        return None;
    }
    Some((start, end))
}

fn handle_sdk_endpoint(
    state_dir: Option<&Path>,
    on_stats: Option<StatsCallback<'_>>,
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
            if let Some(cb) = on_stats
                && let Ok(parsed) =
                    serde_json::from_slice::<BTreeMap<String, serde_json::Value>>(request.body())
            {
                let mut string_stats = BTreeMap::new();
                for (k, v) in parsed {
                    let s = match v {
                        serde_json::Value::String(s) => s,
                        other => other.to_string(),
                    };
                    string_stats.insert(k, s);
                }
                cb(app_id, string_stats);
            }
            json_ok_response(b"{\"ok\":true}".to_vec())
        }
        "state" => {
            let state_path = app_state_file_path(state_dir, app_id);
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
        "media" => {
            let media_dir = get_apps_dir().join(app_id).join("media");
            let payload = list_app_media_json(&media_dir);
            json_ok_response(payload)
        }
        _ => not_found_response(),
    }
}

/// List supported GIF, image, and video files in `<app_dir>/media` as JSON bytes.
pub fn list_app_media_json(media_dir: &Path) -> Vec<u8> {
    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct MediaEntry {
        name: String,
        kind: &'static str,
        url: String,
        size_bytes: u64,
    }

    #[derive(serde::Serialize)]
    struct MediaListResponse {
        folder: String,
        files: Vec<MediaEntry>,
    }

    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(media_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
                continue;
            };
            let kind = match ext.to_ascii_lowercase().as_str() {
                "gif" | "webp" | "png" | "jpg" | "jpeg" => "image",
                "mp4" | "webm" | "mov" | "m4v" => "video",
                _ => continue,
            };
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let url = format!("media/{}", percent_encode_component(name));
            files.push(MediaEntry {
                name: name.to_string(),
                kind,
                url,
                size_bytes,
            });
        }
    }
    files.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
            .then_with(|| a.name.cmp(&b.name))
    });

    let resp = MediaListResponse {
        folder: media_dir.display().to_string(),
        files,
    };
    serde_json::to_vec(&resp).unwrap_or_else(|_| b"{\"folder\":\"\",\"files\":[]}".to_vec())
}

fn percent_encode_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        let ch = *byte as char;
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~') {
            out.push(ch);
        } else {
            out.push('%');
            out.push_str(&format!("{byte:02X}"));
        }
    }
    out
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2]))
        {
            out.push((hi << 4) | lo);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn app_state_file_path(state_dir: Option<&Path>, app_id: &str) -> PathBuf {
    if let Some(dir) = state_dir {
        return dir.join(format!("{app_id}.json"));
    }
    get_apps_dir().join(".state").join(format!("{app_id}.json"))
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
    sdk_init_script_with_controls(app_id, None)
}

/// Generate the `window.Rookery` client SDK initialization script with optional preloaded control values.
pub fn sdk_init_script_with_controls(
    app_id: &str,
    initial_controls: Option<&BTreeMap<String, String>>,
) -> String {
    let escaped = app_id.replace('\\', "\\\\").replace('\'', "\\'");
    let controls_json = initial_controls
        .and_then(|c| serde_json::to_string(c).ok())
        .unwrap_or_else(|| "{}".to_string());
    format!(
        r#"(function() {{
  const appId = '{escaped}';
  const initialControls = {controls_json};
  window.__ROOKERY_INITIAL_CONTROLS__ = initialControls;
  const sdkBase = window.location.origin + '/' + appId + '/__sdk';
  window.Rookery = {{
    appId,
    initialControls,
    getControl(id, fallback) {{
      if (initialControls && id in initialControls) {{
        return initialControls[id];
      }}
      return fallback;
    }},
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
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
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
        "mp4" | "m4v" => "video/mp4",
        "mov" => "video/quicktime",
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
        clear_loaded_webview_cache();
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
                let _ = state.layer.attach_wallpaper(&d.window, &d.info);
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

    // Clean up any webview wallpaper windows belonging to unplugged displays
    let expected_labels: BTreeSet<String> = inner
        .displays
        .values()
        .map(|d| webview_wallpaper_label(&d.info.id.0))
        .collect();
    for (label, window) in app.webview_windows() {
        if label.starts_with("webview-wallpaper-") && !expected_labels.contains(&label) {
            if let Ok(mut map) = loaded_webview_apps().lock() {
                map.remove(&label);
            }
            let _ = window.destroy();
        }
    }

    for d in inner.displays.values() {
        if d.window.is_visible().unwrap_or(true) {
            let _ = d.window.hide();
        }

        let logical_w = d.info.width_px as f64 / d.info.scale_factor;
        let logical_h = d.info.height_px as f64 / d.info.scale_factor;
        let phys_x = (d.info.bounds_native.x * d.info.scale_factor).round() as i32;
        let phys_y = (d.info.bounds_native.y * d.info.scale_factor).round() as i32;

        let controls = state
            .catalog_state
            .custom_controls
            .lock()
            .ok()
            .and_then(|m| m.get(active_app).cloned());
        let init_script = sdk_init_script_with_controls(active_app, controls.as_ref());
        let label = webview_wallpaper_label(&d.info.id.0);
        if let Some(existing) = app.get_webview_window(&label) {
            let already_loaded = loaded_webview_apps()
                .lock()
                .ok()
                .and_then(|m| m.get(&label).cloned())
                .as_deref()
                == Some(active_app);

            if !already_loaded {
                if let Ok(mut map) = loaded_webview_apps().lock() {
                    map.insert(label.clone(), active_app.to_string());
                }
                let _ = existing.navigate(url.clone());
                let _ = existing.eval(&init_script);
            }
            if !existing.is_visible().unwrap_or(true) {
                let _ = existing.show();
            }
            continue;
        }

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
                .position(phys_x as f64, phys_y as f64)
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

        let _ = window.set_position(PhysicalPosition::new(phys_x, phys_y));
        let _ = window.set_size(PhysicalSize::new(d.info.width_px, d.info.height_px));

        if let Err(e) = state.layer.attach_wallpaper(&window, &d.info) {
            log::error!("failed to attach webview wallpaper on {}: {e:#}", d.info.id);
        }

        if let Ok(mut map) = loaded_webview_apps().lock() {
            map.insert(label, active_app.to_string());
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
/// Returns `true` if a webview wallpaper window existed for `info` and was updated.
pub fn set_interactive_webviews(app: &AppHandle, promote: bool, info: &DisplayInfo) -> bool {
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
        true
    } else {
        false
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

    #[test]
    fn sdk_init_script_with_controls_injection() {
        let mut controls = BTreeMap::new();
        controls.insert("wave_speed".to_string(), "2.5".to_string());
        controls.insert("color_speed".to_string(), "0.8".to_string());

        let script = sdk_init_script_with_controls("aurora-waves", Some(&controls));
        assert!(
            script.contains(
                "const initialControls = {\"color_speed\":\"0.8\",\"wave_speed\":\"2.5\"};"
            )
        );
        assert!(script.contains("window.__ROOKERY_INITIAL_CONTROLS__ = initialControls;"));
        assert!(script.contains("getControl(id, fallback)"));
    }

    #[test]
    fn list_app_media_filters_and_encodes_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let media_dir = temp.path().join("media");
        std::fs::create_dir_all(&media_dir).unwrap();
        std::fs::write(media_dir.join("space loop.gif"), b"gif").unwrap();
        std::fs::write(media_dir.join("Alpha.MP4"), b"mp4").unwrap();
        std::fs::write(media_dir.join("README.md"), b"ignore").unwrap();

        let bytes = list_app_media_json(&media_dir);
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let files = parsed["files"].as_array().unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0]["name"], "Alpha.MP4");
        assert_eq!(files[0]["kind"], "video");
        assert_eq!(files[1]["name"], "space loop.gif");
        assert_eq!(files[1]["kind"], "image");
        assert_eq!(files[1]["url"], "media/space%20loop.gif");
        assert_eq!(
            percent_decode("media/space%20loop.gif"),
            "media/space loop.gif"
        );
    }

    #[test]
    fn byte_range_parsing_and_partial_content() {
        assert_eq!(parse_byte_range("bytes=0-9", 100), Some((0, 9)));
        assert_eq!(parse_byte_range("bytes=50-", 100), Some((50, 99)));
        assert_eq!(parse_byte_range("bytes=-10", 100), Some((90, 99)));
        assert_eq!(parse_byte_range("bytes=100-", 100), None);
        assert_eq!(parse_byte_range("bytes=20-10", 100), None);

        let temp = tempfile::tempdir().expect("tempdir");
        let file_path = temp.path().join("clip.mp4");
        std::fs::write(&file_path, b"0123456789abcdef").unwrap();

        let req = Request::builder()
            .uri("rookery-app://localhost/demo/clip.mp4")
            .header(header::RANGE, "bytes=4-9")
            .body(Vec::new())
            .unwrap();
        let res = serve_static_file(&file_path, &req);
        assert_eq!(res.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(res.body(), b"456789");
        assert_eq!(
            res.headers().get(header::CONTENT_RANGE).unwrap(),
            "bytes 4-9/16"
        );
    }
}
