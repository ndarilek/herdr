use super::*;

fn focus_state() -> ClientShellState {
    let mut projected = snapshot();
    projected.workspaces = (1..=3)
        .map(|number| {
            let mut workspace = projected.workspaces[0].clone();
            workspace.workspace_id = format!("ws_{number}");
            workspace.number = number;
            workspace.focused = number == 1;
            workspace
        })
        .collect();
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state
}

fn hidden_cursor_at(state: &mut ClientShellState, rect: Rect) {
    let frame = state.compose(106, 30).expect("composed frame");
    assert_eq!(
        frame.cursor,
        Some(crate::protocol::CursorState {
            x: rect.x,
            y: rect.y,
            visible: false,
            shape: 0,
        })
    );
}

fn row(rows: &[(Rect, usize)], index: usize) -> Rect {
    rows.iter()
        .find(|(_, candidate)| *candidate == index)
        .map(|(rect, _)| *rect)
        .expect("row hit")
}

fn workspace_row(state: &ClientShellState, workspace_id: &str) -> Rect {
    state
        .hits
        .workspaces
        .iter()
        .find(|hit| hit.workspace_id == workspace_id)
        .map(|hit| hit.rect)
        .expect("workspace hit")
}

#[test]
fn sidebar_navigation_parks_hidden_cursor_on_selected_workspace() {
    let mut state = focus_state();
    let before = state.compose(106, 30).expect("pane frame");
    let pane_cursor = before.cursor.clone();
    assert!(pane_cursor.as_ref().is_some_and(|cursor| cursor.visible));

    state.handle_input_bytes(&[0x02]);
    state.handle_input_bytes(b"w");
    assert_eq!(state.mode, ClientShellMode::Navigate);
    state.compose(106, 30).expect("navigation frame");
    let first = workspace_row(&state, "ws_1");
    hidden_cursor_at(&mut state, first);

    state.handle_input_bytes(b"\x1b[B");
    state.compose(106, 30).expect("moved frame");
    let second = workspace_row(&state, "ws_2");
    assert_ne!(first.y, second.y);
    hidden_cursor_at(&mut state, second);

    state.handle_input_bytes(b"\x1b");
    assert_ne!(state.mode, ClientShellMode::Navigate);
    let after = state.compose(106, 30).expect("restored frame");
    assert_eq!(after.cursor, pane_cursor);
}

#[test]
fn menus_park_hidden_cursor_on_highlighted_row() {
    let mut state = focus_state();
    state.compose(106, 30).expect("pane frame");
    state.open_pane_context_menu("pane_1".into(), 10, 5);
    state.compose(106, 30).expect("context menu frame");
    let first = row(&state.hits.context_menu_rows, 0);
    hidden_cursor_at(&mut state, first);
    state.handle_input_bytes(b"\x1b[B");
    state.compose(106, 30).expect("moved context menu");
    let second = row(&state.hits.context_menu_rows, 1);
    hidden_cursor_at(&mut state, second);

    state.overlay = None;
    state.toggle_global_menu();
    state.compose(106, 30).expect("global menu frame");
    let first = row(&state.hits.global_menu_rows, 0);
    hidden_cursor_at(&mut state, first);
    state.move_global_menu_selection(1);
    state.compose(106, 30).expect("moved global menu");
    let second = row(&state.hits.global_menu_rows, 1);
    hidden_cursor_at(&mut state, second);
}

#[test]
fn navigator_parks_hidden_cursor_on_default_selection() {
    let mut state = focus_state();
    state.compose(106, 30).expect("pane frame");
    state.open_navigator_overlay();
    state.compose(106, 30).expect("navigator frame");
    let selected = {
        let Some(ClientShellOverlay::Navigator(navigator)) = state.overlay.as_ref() else {
            panic!("expected navigator");
        };
        let rows =
            render::client_navigator_rows(&state.endpoints, &state.active_endpoint_id, navigator);
        aggregate_navigation::selected_navigator_target(&rows, navigator).expect("default row")
    };
    let rect = state
        .hits
        .navigator_rows
        .iter()
        .find(|(_, target)| *target == selected)
        .map(|(rect, _)| *rect)
        .expect("selected row hit");
    hidden_cursor_at(&mut state, rect);
}

#[test]
fn settings_park_hidden_cursor_on_selected_choice() {
    let mut state = focus_state();
    state.compose(106, 30).expect("pane frame");
    state.open_settings_overlay();
    state.compose(106, 30).expect("settings frame");
    let selected = match state.overlay.as_ref() {
        Some(ClientShellOverlay::Settings(settings)) => settings.selected,
        _ => panic!("expected settings"),
    };
    let rect = row(&state.hits.settings_choices, selected);
    hidden_cursor_at(&mut state, rect);
}

#[test]
fn dialogs_without_rows_park_hidden_cursor_on_primary_button() {
    let mut state = focus_state();
    state.compose(106, 30).expect("pane frame");
    state.overlay = Some(ClientShellOverlay::ConfirmClose(
        ClientConfirmCloseOverlay {
            workspace_id: "ws_1".into(),
            close_group: false,
            tab_target: None,
            title: "Close workspace?".into(),
            detail: "client-shell".into(),
        },
    ));
    state.compose(106, 30).expect("confirm frame");
    let primary = state.hits.overlay_primary;
    assert!(!primary.is_empty());
    hidden_cursor_at(&mut state, primary);
}

#[test]
fn copy_mode_parks_hidden_cursor_on_copy_cell() {
    let mut state = focus_state();
    let mut pane_surface = surface();
    pane_surface.panes[0].scroll = Some(crate::protocol::PaneSurfaceScrollMetrics {
        offset_from_bottom: 0,
        max_offset_from_bottom: 20,
        viewport_rows: 2,
    });
    state.set_pane_surface(pane_surface);
    state.compose(106, 30).expect("pane frame");
    state.record_binding(
        crate::input::KeybindMatch::Action(crate::input::KeybindAction::CopyMode),
        &mut ClientShellInput::default(),
    );
    assert_eq!(state.mode, ClientShellMode::Copy);
    let frame = state.compose(106, 30).expect("copy frame");
    let pane = state.hits.panes[0].inner_rect;
    let copy = state.copy_mode.as_ref().expect("copy mode").cursor;
    let cursor = frame.cursor.clone().expect("copy cursor");
    assert!(!cursor.visible);
    assert_eq!(cursor.x, pane.x + copy.col);
    assert_eq!(cursor.y, pane.y + (copy.row - 20) as u16);
}
