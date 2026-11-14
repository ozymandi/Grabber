//! Update rules that need no Tauri (spec app-distribution, design D3): which launches may
//! check for updates and when an install may start.

use crate::launch::Mode;

/// Only a standalone launch checks for updates. ShareX mode never touches the network.
pub fn may_check(mode: &Mode) -> bool {
    *mode == Mode::Standalone
}

/// Why `install_update` refused to start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    SharexMode,
    NoUpdate,
}

impl Refusal {
    pub fn text(self) -> &'static str {
        match self {
            Refusal::SharexMode => "Updates are not installed while ripper runs from ShareX",
            Refusal::NoUpdate => "No update is available",
        }
    }
}

/// An install may start only in standalone mode and only once an update has been found.
pub fn may_install(mode: &Mode, has_update: bool) -> Result<(), Refusal> {
    if *mode != Mode::Standalone {
        return Err(Refusal::SharexMode);
    }
    if !has_update {
        return Err(Refusal::NoUpdate);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sharex_mode_never_checks() {
        assert!(!may_check(&Mode::Sharex));
    }

    #[test]
    fn standalone_mode_always_checks() {
        assert!(may_check(&Mode::Standalone));
    }

    #[test]
    fn install_refused_in_sharex_mode_even_with_an_update() {
        assert_eq!(may_install(&Mode::Sharex, true), Err(Refusal::SharexMode));
        assert_eq!(may_install(&Mode::Sharex, false), Err(Refusal::SharexMode));
    }

    #[test]
    fn install_refused_without_an_update() {
        assert_eq!(may_install(&Mode::Standalone, false), Err(Refusal::NoUpdate));
    }

    #[test]
    fn install_allowed_standalone_with_an_update() {
        assert_eq!(may_install(&Mode::Standalone, true), Ok(()));
    }
}
