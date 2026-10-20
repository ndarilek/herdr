use super::*;

use ratatui::layout::Position;

use crate::protocol::CursorState;

impl ClientShellState {
    /// Keyboard focus for screen readers, which follow the host terminal's caret rather than
    /// highlighting. Returns `None` when a visible cursor already marks focus.
    pub(super) fn focus_anchor(
        &self,
        frame_cursor: Option<&CursorState>,
        sidebar_navigation: bool,
    ) -> Option<Position> {
        match self.overlay.as_ref() {
            Some(overlay) if frame_cursor.is_none() => self.overlay_focus(overlay),
            Some(_) => None,
            None if sidebar_navigation => self.sidebar_focus(),
            None => None,
        }
    }

    /// The row Enter would act on, falling back to the overlay's primary button.
    fn overlay_focus(&self, overlay: &ClientShellOverlay) -> Option<Position> {
        let hits = &self.hits;
        let row = match overlay {
            ClientShellOverlay::Navigator(_) => hits
                .navigator_selected
                .as_ref()
                .and_then(|target| row_for(&hits.navigator_rows, target)),
            ClientShellOverlay::ContextMenu(menu) => {
                row_for(&hits.context_menu_rows, &menu.highlighted)
            }
            ClientShellOverlay::GlobalMenu(menu) => {
                row_for(&hits.global_menu_rows, &menu.highlighted)
            }
            ClientShellOverlay::Settings(settings) => {
                row_for(&hits.settings_choices, &settings.selected)
            }
            ClientShellOverlay::WorktreeOpen(open) => open
                .selected_entry_index()
                .and_then(|index| row_for(&hits.worktree_rows, &index)),
            _ => None,
        };
        row.or_else(|| (!hits.overlay_primary.is_empty()).then_some(hits.overlay_primary))
            .map(Rect::as_position)
    }

    fn sidebar_focus(&self) -> Option<Position> {
        let target = self.navigate_workspace_id.as_ref()?;
        self.hits
            .workspaces
            .iter()
            .find(|hit| target.matches(&hit.endpoint_id, &hit.workspace_id))
            .map(|hit| hit.rect)
            .or_else(|| {
                self.hits
                    .mobile_targets
                    .iter()
                    .find_map(|(rect, mobile)| match mobile {
                        ClientMobileTarget::Workspace {
                            endpoint_id,
                            workspace_id,
                        } if target.matches(endpoint_id, workspace_id) => Some(*rect),
                        _ => None,
                    })
            })
            .map(Rect::as_position)
    }
}

fn row_for<K: PartialEq>(rows: &[(Rect, K)], key: &K) -> Option<Rect> {
    rows.iter()
        .find(|(_, candidate)| candidate == key)
        .map(|(rect, _)| *rect)
}

/// A cursor the host terminal positions without drawing.
pub(super) fn hidden_cursor(position: Position) -> CursorState {
    CursorState {
        x: position.x,
        y: position.y,
        visible: false,
        shape: 0,
    }
}
