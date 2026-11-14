use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppState { Off, Starting, On, Stopping, Error(String) }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToggleAction { Enable, Disable }

impl AppState {
    pub fn label(&self) -> String {
        match self {
            AppState::Off => "已关闭".into(),
            AppState::Starting => "正在开启…".into(),
            AppState::On => "加速中".into(),
            AppState::Stopping => "正在关闭…".into(),
            AppState::Error(e) => format!("错误：{e}"),
        }
    }
    pub fn toggle_target(&self) -> Option<ToggleAction> {
        match self {
            AppState::Off | AppState::Error(_) => Some(ToggleAction::Enable),
            AppState::On => Some(ToggleAction::Disable),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn labels() {
        assert_eq!(AppState::Off.label(), "已关闭");
        assert_eq!(AppState::On.label(), "加速中");
        assert_eq!(AppState::Error("x".into()).label(), "错误：x");
    }
    #[test]
    fn toggle_targets() {
        assert_eq!(AppState::Off.toggle_target(), Some(ToggleAction::Enable));
        assert_eq!(AppState::On.toggle_target(), Some(ToggleAction::Disable));
        assert_eq!(AppState::Error("e".into()).toggle_target(), Some(ToggleAction::Enable));
        assert_eq!(AppState::Starting.toggle_target(), None);
        assert_eq!(AppState::Stopping.toggle_target(), None);
    }
}