//! # rookery-platform
//!
//! Thin OS layer for the Rookery desktop host:
//!
//! - [`DesktopLayer`]: attaches a window below desktop icons (click-through) and promotes/demotes
//!   it for Direct Interaction Mode.
//! - [`OcclusionProbe`]: queries whether each display is covered by maximized/fullscreen windows.
//! - [`PowerProbe`]: queries AC vs. battery vs. low-power / battery-saver mode.
//! - [`screens_asleep`]: queries whether the displays are asleep.
//! - [`cursor_position`]: returns the global cursor position in native display coordinates.

use anyhow::Result;
use raw_window_handle::HasWindowHandle;
use rookery_core::governor::PowerState;
use rookery_core::{DisplayId, Rect};
use serde::{Deserialize, Serialize};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod stub;
#[cfg(target_os = "windows")]
mod windows;

/// Metadata for a physical display, in the platform's native global screen coordinates
/// (top-left of primary monitor = `(0, 0)`, in logical points on macOS and physical pixels
/// on Windows, matching `CGWindowListCopyWindowInfo` / `GetWindowRect` and `tauri::Monitor`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DisplayInfo {
    pub id: DisplayId,
    pub name: String,
    /// Bounds in the platform's native global coordinate space (logical points on macOS,
    /// physical pixels on Windows).
    pub bounds_native: Rect,
    /// Backing pixel size of the display surface.
    pub width_px: u32,
    pub height_px: u32,
    pub scale_factor: f64,
}

/// Low-level OS wallpaper window operations.
pub trait DesktopLayer: Send + Sync {
    /// Attach `window` to the desktop wallpaper layer on `display` (below desktop icons,
    /// click-through, hidden from taskbar/Switcher/Mission Control).
    fn attach_wallpaper(&self, window: &dyn HasWindowHandle, display: &DisplayInfo) -> Result<()>;

    /// Promote `window` above desktop icons and enable mouse/keyboard input for
    /// Direct Interaction Mode.
    fn promote_interactive(
        &self,
        window: &dyn HasWindowHandle,
        display: &DisplayInfo,
    ) -> Result<()>;

    /// Return `window` to passive, click-through wallpaper mode below desktop icons.
    fn demote_wallpaper(&self, window: &dyn HasWindowHandle, display: &DisplayInfo) -> Result<()>;
}

/// Queries per-display occlusion state.
pub trait OcclusionProbe: Send {
    /// Returns a `bool` per entry in `displays` (`true` = covered >= threshold or fullscreen active).
    /// `own_windows` may be passed if needed, though implementations typically filter by PID.
    fn query(&mut self, displays: &[DisplayInfo]) -> Vec<bool>;
}

/// Queries system power state.
pub trait PowerProbe: Send {
    fn query(&mut self) -> PowerState;
}

/// Instantiate the platform's [`DesktopLayer`].
pub fn desktop_layer() -> Box<dyn DesktopLayer> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacDesktopLayer)
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WinDesktopLayer::new())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Box::new(stub::StubDesktopLayer)
    }
}

/// Instantiate the platform's [`OcclusionProbe`].
pub fn occlusion_probe() -> Box<dyn OcclusionProbe> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOcclusionProbe::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WinOcclusionProbe::new())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Box::new(stub::StubOcclusionProbe)
    }
}

/// Instantiate the platform's [`PowerProbe`].
pub fn power_probe() -> Box<dyn PowerProbe> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacPowerProbe)
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WinPowerProbe)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Box::new(stub::StubPowerProbe)
    }
}

/// Returns `true` when the system displays are asleep.
pub fn screens_asleep() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::screens_asleep()
    }
    #[cfg(target_os = "windows")]
    {
        windows::screens_asleep()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        false
    }
}

/// Mouse button pressed state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MouseButtons {
    pub left: bool,
    pub right: bool,
    pub middle: bool,
}

/// Returns the global cursor position in the same coordinate space as [`DisplayInfo::bounds_native`].
pub fn cursor_position() -> Option<(f64, f64)> {
    #[cfg(target_os = "macos")]
    {
        macos::cursor_position()
    }
    #[cfg(target_os = "windows")]
    {
        windows::cursor_position()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

/// Returns the global mouse button state.
pub fn mouse_buttons() -> MouseButtons {
    #[cfg(target_os = "macos")]
    {
        macos::mouse_buttons()
    }
    #[cfg(target_os = "windows")]
    {
        windows::mouse_buttons()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        stub::mouse_buttons()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probes_run_without_panicking() {
        let mut occ = occlusion_probe();
        let displays = [DisplayInfo {
            id: DisplayId::new("test-0"),
            name: "Primary".into(),
            bounds_native: Rect::new(0.0, 0.0, 1920.0, 1080.0),
            width_px: 1920,
            height_px: 1080,
            scale_factor: 1.0,
        }];
        let res = occ.query(&displays);
        assert_eq!(res.len(), 1);

        let mut pwr = power_probe();
        let _state = pwr.query();
        let _asleep = screens_asleep();
        let _cursor = cursor_position();
    }
}
