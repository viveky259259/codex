//! Archive side-chat controls and toolbar navigation.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ToolbarControl {
    Filter,
    Status,
    SideChats,
    Sort,
}

impl ToolbarControl {
    pub(super) fn previous(self, action: SessionPickerAction, status: SessionStatus) -> Self {
        match self {
            Self::Filter => Self::Sort,
            Self::Status => Self::Filter,
            Self::SideChats => Self::Status,
            Self::Sort
                if matches!(action, SessionPickerAction::Resume)
                    && status == SessionStatus::Archived =>
            {
                Self::SideChats
            }
            Self::Sort if matches!(action, SessionPickerAction::Resume) => Self::Status,
            Self::Sort => Self::Filter,
        }
    }

    pub(super) fn next(self, action: SessionPickerAction, status: SessionStatus) -> Self {
        match self {
            Self::Filter if matches!(action, SessionPickerAction::Resume) => Self::Status,
            Self::Status
                if matches!(action, SessionPickerAction::Resume)
                    && status == SessionStatus::Archived =>
            {
                Self::SideChats
            }
            Self::Filter | Self::Status | Self::SideChats => Self::Sort,
            Self::Sort => Self::Filter,
        }
    }
}

pub(super) fn side_chats_control_spans(state: &PickerState, compact: bool) -> Vec<Span<'static>> {
    let focused = state.toolbar_focus == ToolbarControl::SideChats;
    let value = if state.show_side_conversations {
        "Shown"
    } else {
        "Hidden"
    };
    let label = if compact {
        "Side chats:"
    } else {
        "Side chats: "
    };
    vec![
        label.set_style(secondary_text_style()),
        toolbar_value(value, /*active*/ true, focused),
    ]
}

#[cfg(test)]
#[path = "side_chats_tests.rs"]
mod tests;
