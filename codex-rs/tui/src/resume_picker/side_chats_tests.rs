use super::super::tests::local_db_first_state;
use super::super::tests::make_row;
use super::super::tests::page;
use super::super::tests::page_only_loader;
use super::super::*;
use insta::assert_snapshot;
use pretty_assertions::assert_eq;

#[test]
fn archived_picker_hides_side_conversations_by_default_and_can_show_them() {
    let loader = page_only_loader(|_| {});
    let mut state = PickerState::new(
        FrameRequester::test_dummy(),
        loader,
        ProviderFilter::Any,
        /*show_all*/ true,
        /*filter_cwd*/ None,
        SessionPickerAction::Resume,
    );
    state.status = SessionStatus::Archived;
    let mut side_row = make_row("/side.jsonl", "2025-01-01T00:00:00Z", "side chat");
    side_row.is_side_conversation = true;
    let regular_row = make_row("/main.jsonl", "2025-01-01T00:00:00Z", "main chat");

    assert!(!state.row_matches_filter(&side_row));
    assert!(state.row_matches_filter(&regular_row));

    state.toolbar_focus = ToolbarControl::SideChats;
    state.change_focused_toolbar_value();
    assert!(state.row_matches_filter(&side_row));

    let mut snapshots = Vec::new();
    for density in [SessionListDensity::Comfortable, SessionListDensity::Dense] {
        state.density = density;
        state.relative_time_reference = parse_timestamp_str("2025-01-02T00:00:00Z");
        let rows = render_session_lines(
            &side_row, &state, /*is_selected*/ true, /*is_expanded*/ false,
            /*is_zebra*/ false, /*width*/ 80,
        )
        .into_iter()
        .map(|line| line.to_string().trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n");
        snapshots.push(format!(
            "{density:?}\n{}\n{rows}",
            toolbar_line(&state, false)
        ));
    }
    assert_snapshot!(snapshots.join("\n\n"));
}

#[test]
fn archived_toolbar_cycles_through_side_chat_filter_in_both_directions() {
    let controls = [
        ToolbarControl::Filter,
        ToolbarControl::Status,
        ToolbarControl::SideChats,
        ToolbarControl::Sort,
    ];
    for (index, control) in controls.into_iter().enumerate() {
        assert_eq!(
            control.next(SessionPickerAction::Resume, SessionStatus::Archived),
            controls[(index + 1) % controls.len()]
        );
        assert_eq!(
            control.previous(SessionPickerAction::Resume, SessionStatus::Archived),
            controls[(index + controls.len() - 1) % controls.len()]
        );
    }
}

#[tokio::test]
async fn archived_picker_loads_past_a_page_containing_only_hidden_side_chats() {
    let (mut state, requests) = local_db_first_state();
    state.status = SessionStatus::Archived;
    state.start_initial_load();
    let token = state.next_request_token - 1;
    let mut side_row = make_row("/side.jsonl", "2025-01-01T00:00:00Z", "side chat");
    side_row.is_side_conversation = true;
    state
        .handle_background_event(BackgroundEvent::Page {
            request_token: token,
            search_token: None,
            page: Ok(page(
                vec![side_row],
                Some("next"),
                /*num_scanned_files*/ 1,
                /*reached_scan_cap*/ false,
            )),
        })
        .await
        .unwrap();
    assert!(state.filtered_rows.is_empty());
    assert_eq!(requests.lock().unwrap().len(), 2);
    assert!(
        matches!(&requests.lock().unwrap()[1].cursor, Some(PageCursor::AppServer(cursor)) if cursor == "next")
    );
}
