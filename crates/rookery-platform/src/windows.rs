//! Windows implementation of [`DesktopLayer`], [`OcclusionProbe`], and [`PowerProbe`].
//!
//! Handles both the classic Windows 10 / Windows 11 <=23H2 desktop hierarchy (top-level
//! `WorkerW` sibling following the `WorkerW` that hosts `SHELLDLL_DefView`) and the
//! Windows 11 24H2+ hierarchy (`WorkerW` child inside `Progman` below `SHELLDLL_DefView`).

use std::ffi::c_void;
use std::mem::size_of;

use anyhow::{Result, bail};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use rookery_core::Rect;
use rookery_core::governor::PowerState;
use rookery_core::occlusion::{DEFAULT_COVERAGE_THRESHOLD, is_covered};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute,
};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::UI::Shell::{
    QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowExW, FindWindowW, GWL_EXSTYLE, GWL_STYLE, GetClientRect, GetCursorPos,
    GetSystemMetrics, GetWindowLongPtrW, GetWindowRect, GetWindowThreadProcessId, HWND_BOTTOM,
    HWND_TOP, IsIconic, IsWindowVisible, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SMTO_NORMAL,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_SHOWWINDOW, SendMessageTimeoutW, SetForegroundWindow,
    SetParent, SetWindowLongPtrW, SetWindowPos, WS_CAPTION, WS_CHILD, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP, WS_THICKFRAME, WS_VISIBLE,
};
use windows::core::BOOL;
use windows::core::w;

use crate::{DesktopLayer, DisplayInfo, OcclusionProbe, PowerProbe};

const WM_SPAWN_WORKERW: u32 = 0x052C;

pub struct WinDesktopLayer;

impl WinDesktopLayer {
    pub fn new() -> Self {
        Self
    }
}

fn hwnd_from_handle(handle: &dyn HasWindowHandle) -> Result<HWND> {
    let raw = handle.window_handle()?.as_raw();
    let RawWindowHandle::Win32(win32) = raw else {
        bail!("expected Win32 window handle on Windows");
    };
    Ok(HWND(win32.hwnd.get() as *mut c_void))
}

/// Locate the wallpaper host `WorkerW` and (if present) the `SHELLDLL_DefView` child inside
/// `Progman` (Windows 11 24H2+).
unsafe fn find_wallpaper_host() -> Result<(HWND, Option<HWND>)> {
    let progman = unsafe { FindWindowW(w!("Progman"), None)? };
    if progman.0.is_null() {
        bail!("Progman window not found");
    }

    // Instruct Explorer to spawn the wallpaper WorkerW behind SHELLDLL_DefView.
    let mut _result: usize = 0;
    let _ = unsafe {
        SendMessageTimeoutW(
            progman,
            WM_SPAWN_WORKERW,
            WPARAM(0xD),
            LPARAM(0x1),
            SMTO_NORMAL,
            1000,
            Some(&mut _result),
        )
    };

    // Windows 11 24H2+: Progman hosts both `SHELLDLL_DefView` and a child `WorkerW`.
    let defview_in_progman =
        unsafe { FindWindowExW(Some(progman), None, w!("SHELLDLL_DefView"), None).ok() }
            .filter(|h| !h.0.is_null());
    let workerw_in_progman =
        unsafe { FindWindowExW(Some(progman), None, w!("WorkerW"), None).ok() }
            .filter(|h| !h.0.is_null());

    if let Some(defview) = defview_in_progman
        && workerw_in_progman.is_some()
    {
        return Ok((progman, Some(defview)));
    }

    // Classic layout (Win10 / Win11 <=23H2): enumerate top-level windows to find the WorkerW
    // that sits immediately after the one containing `SHELLDLL_DefView`.
    let mut legacy_workerw = HWND::default();
    let _ = unsafe {
        EnumWindows(
            Some(find_legacy_workerw_proc),
            LPARAM((&mut legacy_workerw as *mut HWND) as isize),
        )
    };
    if !legacy_workerw.0.is_null() {
        return Ok((legacy_workerw, None));
    }

    if let Some(w) = workerw_in_progman {
        return Ok((w, None));
    }

    Ok((progman, None))
}

unsafe extern "system" fn find_legacy_workerw_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let defview = unsafe { FindWindowExW(Some(hwnd), None, w!("SHELLDLL_DefView"), None) };
    if let Ok(dv) = defview
        && !dv.0.is_null()
        && let Ok(worker) = unsafe { FindWindowExW(None, Some(hwnd), w!("WorkerW"), None) }
        && !worker.0.is_null()
    {
        let out = lparam.0 as *mut HWND;
        unsafe { *out = worker };
        return BOOL(0);
    }
    BOOL(1)
}

impl DesktopLayer for WinDesktopLayer {
    fn attach_wallpaper(&self, window: &dyn HasWindowHandle, display: &DisplayInfo) -> Result<()> {
        let hwnd = hwnd_from_handle(window)?;
        unsafe {
            let (parent, defview_on_24h2) = find_wallpaper_host()?;

            let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
            let new_style = (style & !(WS_POPUP.0 | WS_CAPTION.0 | WS_THICKFRAME.0))
                | WS_CHILD.0
                | WS_VISIBLE.0;
            SetWindowLongPtrW(hwnd, GWL_STYLE, new_style as isize);

            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            let new_ex = ex_style | WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0 | WS_EX_TRANSPARENT.0;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex as isize);

            let _ = SetParent(hwnd, Some(parent));

            let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let x = display.bounds_native.x as i32 - vx;
            let y = display.bounds_native.y as i32 - vy;
            let w = display.bounds_native.width as i32;
            let h = display.bounds_native.height as i32;

            // On 24H2, position right after SHELLDLL_DefView in Z-order so icons remain on top.
            let insert_after = defview_on_24h2.unwrap_or(HWND_BOTTOM);
            SetWindowPos(
                hwnd,
                Some(insert_after),
                x,
                y,
                w,
                h,
                SWP_NOACTIVATE | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
            )?;
        }
        log::info!("attached Windows wallpaper window on {}", display.id);
        Ok(())
    }

    fn promote_interactive(
        &self,
        window: &dyn HasWindowHandle,
        display: &DisplayInfo,
    ) -> Result<()> {
        let hwnd = hwnd_from_handle(window)?;
        unsafe {
            let _ = SetParent(hwnd, None);
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
            let new_style = (style & !WS_CHILD.0) | WS_POPUP.0 | WS_VISIBLE.0;
            SetWindowLongPtrW(hwnd, GWL_STYLE, new_style as isize);

            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            let new_ex =
                (ex_style & !(WS_EX_NOACTIVATE.0 | WS_EX_TRANSPARENT.0)) | WS_EX_TOOLWINDOW.0;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex as isize);

            SetWindowPos(
                hwnd,
                Some(HWND_TOP),
                display.bounds_native.x as i32,
                display.bounds_native.y as i32,
                display.bounds_native.width as i32,
                display.bounds_native.height as i32,
                SWP_FRAMECHANGED | SWP_SHOWWINDOW,
            )?;
            let _ = SetForegroundWindow(hwnd);
        }
        log::info!(
            "promoted Windows wallpaper window on {} to interactive",
            display.id
        );
        Ok(())
    }

    fn demote_wallpaper(&self, window: &dyn HasWindowHandle, display: &DisplayInfo) -> Result<()> {
        self.attach_wallpaper(window, display)
    }
}

pub struct WinOcclusionProbe {
    own_pid: u32,
}

struct EnumCtx {
    own_pid: u32,
    rects: Vec<Rect>,
}

impl WinOcclusionProbe {
    pub fn new() -> Self {
        Self {
            own_pid: std::process::id(),
        }
    }

    fn fullscreen_or_busy() -> bool {
        let Ok(state) = (unsafe { SHQueryUserNotificationState() }) else {
            return false;
        };
        matches!(
            state,
            QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_BUSY | QUNS_PRESENTATION_MODE
        )
    }

    fn visible_window_rects(&self) -> Vec<Rect> {
        let mut ctx = EnumCtx {
            own_pid: self.own_pid,
            rects: Vec::with_capacity(32),
        };
        let _ = unsafe {
            EnumWindows(
                Some(collect_windows_proc),
                LPARAM((&mut ctx as *mut EnumCtx) as isize),
            )
        };
        ctx.rects
    }
}

unsafe extern "system" fn collect_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = unsafe { &mut *(lparam.0 as *mut EnumCtx) };
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return BOOL(1);
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == ctx.own_pid {
            return BOOL(1);
        }
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        if (ex_style & WS_EX_TOOLWINDOW.0) != 0 {
            return BOOL(1);
        }
        let mut cloaked: u32 = 0;
        if DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&mut cloaked as *mut u32).cast::<c_void>(),
            size_of::<u32>() as u32,
        )
        .is_ok()
            && cloaked != 0
        {
            return BOOL(1);
        }
        // Skip empty client-rect shells (like invisible system hosts).
        let mut client = RECT::default();
        if GetClientRect(hwnd, &mut client).is_ok()
            && (client.right - client.left <= 1 || client.bottom - client.top <= 1)
        {
            return BOOL(1);
        }
        let mut r = RECT::default();
        let got_dwm = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            (&mut r as *mut RECT).cast::<c_void>(),
            size_of::<RECT>() as u32,
        )
        .is_ok();
        if !got_dwm && GetWindowRect(hwnd, &mut r).is_err() {
            return BOOL(1);
        }
        let rect = Rect::new(
            r.left as f64,
            r.top as f64,
            (r.right - r.left) as f64,
            (r.bottom - r.top) as f64,
        );
        if !rect.is_empty() {
            ctx.rects.push(rect);
        }
    }
    BOOL(1)
}

impl OcclusionProbe for WinOcclusionProbe {
    fn query(&mut self, displays: &[DisplayInfo]) -> Vec<bool> {
        if Self::fullscreen_or_busy() {
            return vec![true; displays.len()];
        }
        let windows = self.visible_window_rects();
        displays
            .iter()
            .map(|d| is_covered(d.bounds_native, &windows, DEFAULT_COVERAGE_THRESHOLD))
            .collect()
    }
}

pub struct WinPowerProbe;

impl PowerProbe for WinPowerProbe {
    fn query(&mut self) -> PowerState {
        let mut status = SYSTEM_POWER_STATUS::default();
        if unsafe { GetSystemPowerStatus(&mut status) }.is_ok() {
            // SystemStatusFlag == 1 when Windows Battery Saver / Energy Saver is active.
            if status.SystemStatusFlag == 1 {
                return PowerState::LowPower;
            }
            // ACLineStatus == 0 means running on battery.
            if status.ACLineStatus == 0 {
                return PowerState::Battery;
            }
        }
        PowerState::Ac
    }
}

pub fn screens_asleep() -> bool {
    false
}

pub fn cursor_position() -> Option<(f64, f64)> {
    let mut pt = POINT::default();
    unsafe { GetCursorPos(&mut pt).ok()? };
    Some((pt.x as f64, pt.y as f64))
}
