//! macOS implementation of [`DesktopLayer`], [`OcclusionProbe`], and [`PowerProbe`].

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{LazyLock, Mutex};

use anyhow::{Context, Result, bail};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSApplication, NSView, NSWindow, NSWindowCollectionBehavior, NSWindowOcclusionState,
};
use objc2_core_foundation::{CFDictionary, CFNumber, CFString, CFType, CGPoint, CGRect, CGSize};
use objc2_core_graphics::{
    CGDisplayIsAsleep, CGEvent, CGEventSource, CGEventSourceStateID, CGMainDisplayID,
    CGMouseButton, CGRectMakeWithDictionaryRepresentation, CGWindowLevelForKey, CGWindowLevelKey,
    CGWindowListCopyWindowInfo, CGWindowListOption, kCGDesktopIconWindowLevel, kCGNullWindowID,
    kCGWindowAlpha, kCGWindowBounds, kCGWindowLayer, kCGWindowOwnerPID,
};
use objc2_foundation::NSProcessInfo;
use objc2_io_kit::{IOPSCopyPowerSourcesInfo, IOPSGetProvidingPowerSourceType};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use rookery_core::governor::PowerState;
use rookery_core::occlusion::{DEFAULT_COVERAGE_THRESHOLD, is_covered};
use rookery_core::{DisplayId, Rect};

use crate::{DesktopLayer, DisplayInfo, MouseButtons, OcclusionProbe, PowerProbe};

/// Map of `DisplayId` -> raw `NSWindow` pointer (only dereferenced on the main thread or for
/// thread-safe `occlusionState` reads while the window is alive).
static WALLPAPER_WINDOWS: LazyLock<Mutex<HashMap<DisplayId, usize>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub struct MacDesktopLayer;

fn ns_window_from_handle(handle: &dyn HasWindowHandle) -> Result<Retained<NSWindow>> {
    let raw = handle.window_handle()?.as_raw();
    let RawWindowHandle::AppKit(appkit) = raw else {
        bail!("expected AppKit window handle on macOS");
    };
    let ns_view_ptr = appkit.ns_view.as_ptr().cast::<NSView>();
    // SAFETY: Tauri gives us a valid `NSView` pointer on the main thread.
    let view: &NSView = unsafe { &*ns_view_ptr };
    view.window().context("NSView has no owning NSWindow")
}

fn desktop_collection_behavior() -> NSWindowCollectionBehavior {
    NSWindowCollectionBehavior::CanJoinAllSpaces
        | NSWindowCollectionBehavior::Stationary
        | NSWindowCollectionBehavior::IgnoresCycle
        | NSWindowCollectionBehavior::FullScreenNone
}

impl DesktopLayer for MacDesktopLayer {
    fn attach_wallpaper(&self, window: &dyn HasWindowHandle, display: &DisplayInfo) -> Result<()> {
        let ns_win = ns_window_from_handle(window)?;
        let desktop_level = CGWindowLevelForKey(CGWindowLevelKey::DesktopWindowLevelKey) as isize;
        ns_win.setLevel(desktop_level);
        ns_win.setCollectionBehavior(desktop_collection_behavior());
        ns_win.setIgnoresMouseEvents(true);
        ns_win.setHasShadow(false);
        ns_win.orderBack(None);

        if let Ok(mut map) = WALLPAPER_WINDOWS.lock() {
            map.insert(display.id.clone(), Retained::as_ptr(&ns_win) as usize);
        }
        log::info!(
            "attached macOS wallpaper window on {} at level {}",
            display.id,
            desktop_level
        );
        Ok(())
    }

    fn promote_interactive(
        &self,
        window: &dyn HasWindowHandle,
        display: &DisplayInfo,
    ) -> Result<()> {
        let ns_win = ns_window_from_handle(window)?;
        // Sit just above desktop icons (`kCGDesktopIconWindowLevel + 1`), still below normal apps.
        let level = (kCGDesktopIconWindowLevel + 1) as isize;
        ns_win.setLevel(level);
        ns_win.setCollectionBehavior(desktop_collection_behavior());
        ns_win.setIgnoresMouseEvents(false);
        if let Some(mtm) = MainThreadMarker::new() {
            let app = NSApplication::sharedApplication(mtm);
            app.activate();
        }
        ns_win.makeKeyAndOrderFront(None);
        log::info!(
            "promoted macOS wallpaper window on {} to interactive level {}",
            display.id,
            level
        );
        Ok(())
    }

    fn demote_wallpaper(&self, window: &dyn HasWindowHandle, display: &DisplayInfo) -> Result<()> {
        let ns_win = ns_window_from_handle(window)?;
        let desktop_level = CGWindowLevelForKey(CGWindowLevelKey::DesktopWindowLevelKey) as isize;
        ns_win.setLevel(desktop_level);
        ns_win.setCollectionBehavior(desktop_collection_behavior());
        ns_win.setIgnoresMouseEvents(true);
        ns_win.orderBack(None);
        log::info!(
            "demoted macOS wallpaper window on {} back to level {}",
            display.id,
            desktop_level
        );
        Ok(())
    }
}

pub struct MacOcclusionProbe {
    own_pid: i32,
}

impl MacOcclusionProbe {
    pub fn new() -> Self {
        Self {
            own_pid: std::process::id() as i32,
        }
    }

    /// Collect on-screen normal-layer window rectangles (in global Quartz coordinates).
    fn visible_window_rects(&self) -> Vec<Rect> {
        let options =
            CGWindowListOption::OptionOnScreenOnly | CGWindowListOption::ExcludeDesktopElements;
        let Some(array) = CGWindowListCopyWindowInfo(options, kCGNullWindowID) else {
            return Vec::new();
        };
        let count = array.count();
        let mut rects = Vec::with_capacity(count.max(0) as usize);
        for i in 0..count {
            let item_ptr = unsafe { array.value_at_index(i) };
            if item_ptr.is_null() {
                continue;
            }
            let dict: &CFDictionary = unsafe { &*(item_ptr.cast::<CFDictionary>()) };

            // 1. Only consider normal application windows (layer == 0).
            let Some(layer) = dict_i32(dict, unsafe { kCGWindowLayer }) else {
                continue;
            };
            if layer != 0 {
                continue;
            }

            // 2. Skip our own windows.
            if let Some(pid) = dict_i32(dict, unsafe { kCGWindowOwnerPID })
                && pid == self.own_pid
            {
                continue;
            }

            // 3. Skip nearly-transparent overlays.
            if let Some(alpha) = dict_f64(dict, unsafe { kCGWindowAlpha })
                && alpha < 0.85
            {
                continue;
            }

            // 4. Parse bounds dictionary.
            if let Some(r) = dict_bounds(dict)
                && !r.is_empty()
            {
                rects.push(r);
            }
        }
        rects
    }

    fn is_window_occluded_by_os(display_id: &DisplayId) -> bool {
        let Ok(map) = WALLPAPER_WINDOWS.lock() else {
            return false;
        };
        let Some(&ptr) = map.get(display_id) else {
            return false;
        };
        if ptr == 0 {
            return false;
        }
        // SAFETY: `ptr` points to a live `NSWindow` managed by the host. `occlusionState` is an
        // atomic property read in AppKit.
        let ns_win: &NSWindow = unsafe { &*(ptr as *const NSWindow) };
        !ns_win
            .occlusionState()
            .contains(NSWindowOcclusionState::Visible)
    }
}

fn dict_raw(dict: &CFDictionary, key: &CFString) -> Option<*const c_void> {
    let key_ptr = (key as *const CFString).cast::<c_void>();
    let val = unsafe { CFDictionary::value(dict, key_ptr) };
    (!val.is_null()).then_some(val)
}

fn dict_i32(dict: &CFDictionary, key: &CFString) -> Option<i32> {
    let raw = dict_raw(dict, key)?;
    let num: &CFNumber = unsafe { &*(raw.cast::<CFNumber>()) };
    num.as_i32()
}

fn dict_f64(dict: &CFDictionary, key: &CFString) -> Option<f64> {
    let raw = dict_raw(dict, key)?;
    let num: &CFNumber = unsafe { &*(raw.cast::<CFNumber>()) };
    num.as_f64()
}

fn dict_bounds(dict: &CFDictionary) -> Option<Rect> {
    let raw = dict_raw(dict, unsafe { kCGWindowBounds })?;
    let bounds_dict: &CFDictionary = unsafe { &*(raw.cast::<CFDictionary>()) };
    let mut cg = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize {
            width: 0.0,
            height: 0.0,
        },
    };
    let ok = unsafe { CGRectMakeWithDictionaryRepresentation(Some(bounds_dict), &mut cg) };
    ok.then(|| Rect::new(cg.origin.x, cg.origin.y, cg.size.width, cg.size.height))
}

impl OcclusionProbe for MacOcclusionProbe {
    fn query(&mut self, displays: &[DisplayInfo]) -> Vec<bool> {
        let windows = self.visible_window_rects();
        displays
            .iter()
            .map(|d| {
                Self::is_window_occluded_by_os(&d.id)
                    || is_covered(d.bounds_native, &windows, DEFAULT_COVERAGE_THRESHOLD)
            })
            .collect()
    }
}

pub struct MacPowerProbe;

impl PowerProbe for MacPowerProbe {
    fn query(&mut self) -> PowerState {
        let info = NSProcessInfo::processInfo();
        if info.isLowPowerModeEnabled() {
            return PowerState::LowPower;
        }
        if let Some(blob) = IOPSCopyPowerSourcesInfo() {
            let cf_type: &CFType = &blob;
            if let Some(source) = unsafe { IOPSGetProvidingPowerSourceType(Some(cf_type)) } {
                let s = source.to_string();
                if s.contains("Battery") {
                    return PowerState::Battery;
                }
            }
        }
        PowerState::Ac
    }
}

pub fn screens_asleep() -> bool {
    CGDisplayIsAsleep(CGMainDisplayID())
}

pub fn cursor_position() -> Option<(f64, f64)> {
    let event = CGEvent::new(None)?;
    let pt = CGEvent::location(Some(&event));
    Some((pt.x, pt.y))
}

pub fn mouse_buttons() -> MouseButtons {
    let left = CGEventSource::button_state(
        CGEventSourceStateID::CombinedSessionState,
        CGMouseButton::Left,
    );
    let right = CGEventSource::button_state(
        CGEventSourceStateID::CombinedSessionState,
        CGMouseButton::Right,
    );
    let middle = CGEventSource::button_state(
        CGEventSourceStateID::CombinedSessionState,
        CGMouseButton::Center,
    );
    MouseButtons {
        left,
        right,
        middle,
    }
}

pub fn activate_app_and_focus_window(window: &dyn HasWindowHandle) -> Result<()> {
    let ns_win = ns_window_from_handle(window)?;
    if let Some(mtm) = MainThreadMarker::new() {
        let app = NSApplication::sharedApplication(mtm);
        app.activate();
    }
    ns_win.makeKeyAndOrderFront(None);
    Ok(())
}
