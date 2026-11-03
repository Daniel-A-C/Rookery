//! Direct Interaction Mode state machine (VISION.md §2.2).
//!
//! The host feeds [`InteractionEvent`]s in and executes the returned [`Effect`]s against
//! the OS layer and renderer. Keeping this logic OS-free makes it trivially testable.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionMode {
    /// Wallpaper sits below desktop icons and is fully click-through.
    #[default]
    Passive,
    /// Wallpaper is promoted above the icons and captures mouse/keyboard input.
    Interactive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractionEvent {
    /// The global hotkey was pressed (toggles).
    HotkeyPressed,
    /// Explicit request from tray menu / popover.
    SetInteractive(bool),
    /// `Esc` pressed while interactive.
    EscPressed,
    /// A wallpaper surface lost keyboard focus (user clicked into another app).
    FocusLost,
    /// Displays were added/removed; new surfaces must match the current mode.
    DisplaysChanged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Raise wallpaper windows above desktop icons and accept input.
    Promote,
    /// Return wallpaper windows below desktop icons, click-through.
    Demote,
    /// Toggle the "interactive" visual indicator (vignette + corner badge).
    ShowIndicator(bool),
    /// Start listening for `Esc` (only while interactive, so we never steal it globally).
    ListenForEscape(bool),
}

#[derive(Debug, Default)]
pub struct Interaction {
    mode: InteractionMode,
}

impl Interaction {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mode(&self) -> InteractionMode {
        self.mode
    }

    pub fn is_interactive(&self) -> bool {
        self.mode == InteractionMode::Interactive
    }

    /// Apply an event, returning the effects the host must execute (in order).
    pub fn handle(&mut self, event: InteractionEvent) -> Vec<Effect> {
        use InteractionEvent::*;
        use InteractionMode::*;
        let target = match (self.mode, event) {
            (Passive, HotkeyPressed) => Interactive,
            (Interactive, HotkeyPressed) => Passive,
            (_, SetInteractive(true)) => Interactive,
            (_, SetInteractive(false)) => Passive,
            (Interactive, EscPressed | FocusLost) => Passive,
            (Passive, EscPressed | FocusLost) => Passive,
            (mode, DisplaysChanged) => {
                // Re-apply the current mode so freshly created surfaces match it.
                return Self::effects_for(mode);
            }
        };
        if target == self.mode {
            return Vec::new();
        }
        self.mode = target;
        Self::effects_for(target)
    }

    fn effects_for(mode: InteractionMode) -> Vec<Effect> {
        match mode {
            InteractionMode::Interactive => vec![
                Effect::Promote,
                Effect::ShowIndicator(true),
                Effect::ListenForEscape(true),
            ],
            InteractionMode::Passive => vec![
                Effect::ListenForEscape(false),
                Effect::ShowIndicator(false),
                Effect::Demote,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Effect::*;
    use InteractionEvent::*;

    #[test]
    fn starts_passive() {
        assert_eq!(Interaction::new().mode(), InteractionMode::Passive);
    }

    #[test]
    fn hotkey_toggles() {
        let mut i = Interaction::new();
        assert_eq!(
            i.handle(HotkeyPressed),
            vec![Promote, ShowIndicator(true), ListenForEscape(true)]
        );
        assert!(i.is_interactive());
        assert_eq!(
            i.handle(HotkeyPressed),
            vec![ListenForEscape(false), ShowIndicator(false), Demote]
        );
        assert!(!i.is_interactive());
    }

    #[test]
    fn esc_and_focus_loss_exit_interactive() {
        for exit in [EscPressed, FocusLost] {
            let mut i = Interaction::new();
            i.handle(HotkeyPressed);
            assert!(!i.handle(exit).is_empty());
            assert_eq!(i.mode(), InteractionMode::Passive);
        }
    }

    #[test]
    fn esc_and_focus_loss_are_noops_when_passive() {
        let mut i = Interaction::new();
        assert!(i.handle(EscPressed).is_empty());
        assert!(i.handle(FocusLost).is_empty());
    }

    #[test]
    fn explicit_set_is_idempotent() {
        let mut i = Interaction::new();
        assert!(!i.handle(SetInteractive(true)).is_empty());
        assert!(i.handle(SetInteractive(true)).is_empty());
        assert!(!i.handle(SetInteractive(false)).is_empty());
        assert!(i.handle(SetInteractive(false)).is_empty());
    }

    #[test]
    fn displays_changed_reapplies_current_mode() {
        let mut i = Interaction::new();
        assert_eq!(i.handle(DisplaysChanged).last(), Some(&Demote));
        i.handle(HotkeyPressed);
        assert_eq!(i.handle(DisplaysChanged).first(), Some(&Promote));
        assert!(i.is_interactive());
    }
}
