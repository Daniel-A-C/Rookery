//! Fallback no-op implementation for unsupported platforms (e.g. Linux stub for future work).

use anyhow::Result;
use raw_window_handle::HasWindowHandle;
use rookery_core::governor::PowerState;

use crate::{DesktopLayer, DisplayInfo, OcclusionProbe, PowerProbe};

pub struct StubDesktopLayer;

impl DesktopLayer for StubDesktopLayer {
    fn attach_wallpaper(
        &self,
        _window: &dyn HasWindowHandle,
        _display: &DisplayInfo,
    ) -> Result<()> {
        Ok(())
    }

    fn promote_interactive(
        &self,
        _window: &dyn HasWindowHandle,
        _display: &DisplayInfo,
    ) -> Result<()> {
        Ok(())
    }

    fn demote_wallpaper(
        &self,
        _window: &dyn HasWindowHandle,
        _display: &DisplayInfo,
    ) -> Result<()> {
        Ok(())
    }
}

pub struct StubOcclusionProbe;

impl OcclusionProbe for StubOcclusionProbe {
    fn query(&mut self, displays: &[DisplayInfo]) -> Vec<bool> {
        vec![false; displays.len()]
    }
}

pub struct StubPowerProbe;

impl PowerProbe for StubPowerProbe {
    fn query(&mut self) -> PowerState {
        PowerState::Ac
    }
}
