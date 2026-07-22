use std::sync::Arc;

use super::*;
use crate::app::test_support::make_test_app_with_event_receiver;
use crate::history_cell;
use crate::test_support::PathBufExt;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::Turn;
use codex_app_server_protocol::TurnStatus;
use codex_config::ConfigLayerStack;
use codex_protocol::ThreadId;
use pretty_assertions::assert_eq;
use ratatui::text::Line;
use tempfile::tempdir;

#[tokio::test]
async fn set_full_transparency_event_persists_and_syncs_runtime_state() -> Result<()> {
    let (mut app, mut app_event_rx) = make_test_app_with_event_receiver().await;
    let mut app_server = Box::pin(crate::start_embedded_app_server_for_picker(&app.config)).await?;
    let codex_home = tempdir()?;
    app.config.codex_home = codex_home.path().to_path_buf().abs();
    app.transcript_cells
        .push(Arc::new(history_cell::new_info_event(
            "Existing transcript entry.".to_string(),
            /*hint*/ None,
        )));
    let mut tui = crate::tui::test_support::make_test_tui()?;

    let control = Box::pin(app.handle_event(
        &mut tui,
        &mut app_server,
        AppEvent::SetFullTransparency { enabled: true },
    ))
    .await?;

    assert!(matches!(control, AppRunControl::Continue));
    assert!(app.config.tui_full_transparency);
    assert!(app.chat_widget.config_ref().tui_full_transparency);
    assert!(tui.terminal.full_transparency());
    assert!(app.has_emitted_history_lines);
    let config = std::fs::read_to_string(codex_home.path().join("config.toml"))?;
    let config: toml::Value = toml::from_str(&config)?;
    let persisted = config
        .get("tui")
        .and_then(toml::Value::as_table)
        .and_then(|tui| tui.get("full_transparency"));
    assert_eq!(persisted, Some(&toml::Value::Boolean(true)));

    let mut next_history_text = || loop {
        match app_event_rx.try_recv() {
            Ok(AppEvent::InsertHistoryCell(cell)) => {
                return cell
                    .display_lines(/*width*/ 80)
                    .iter()
                    .map(|line| {
                        line.spans
                            .iter()
                            .map(|span| span.content.as_ref())
                            .collect::<String>()
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
            }
            Ok(_) => continue,
            Err(err) => panic!("expected transparency feedback history cell: {err}"),
        }
    };
    let enabled_feedback = next_history_text();

    let control = Box::pin(app.handle_event(
        &mut tui,
        &mut app_server,
        AppEvent::SetFullTransparency { enabled: false },
    ))
    .await?;

    assert!(matches!(control, AppRunControl::Continue));
    assert!(!app.config.tui_full_transparency);
    assert!(!app.chat_widget.config_ref().tui_full_transparency);
    assert!(!tui.terminal.full_transparency());
    let disabled_feedback = next_history_text();
    insta::assert_snapshot!(
        "full_transparency_toggle_feedback",
        format!("{enabled_feedback}\n{disabled_feedback}")
    );
    let config = std::fs::read_to_string(codex_home.path().join("config.toml"))?;
    let config: toml::Value = toml::from_str(&config)?;
    let persisted = config
        .get("tui")
        .and_then(toml::Value::as_table)
        .and_then(|tui| tui.get("full_transparency"));
    assert_eq!(persisted, Some(&toml::Value::Boolean(false)));
    app_server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn update_full_transparency_is_rejected_after_task_starts() -> Result<()> {
    let (mut app, mut app_event_rx) = make_test_app_with_event_receiver().await;
    let codex_home = tempdir()?;
    app.config.codex_home = codex_home.path().to_path_buf().abs();
    let mut tui = crate::tui::test_support::make_test_tui()?;
    app.chat_widget.handle_server_notification(
        ServerNotification::TurnStarted(codex_app_server_protocol::TurnStartedNotification {
            thread_id: ThreadId::new().to_string(),
            turn: Turn {
                id: "turn-1".to_string(),
                items_view: codex_app_server_protocol::TurnItemsView::Full,
                items: Vec::new(),
                status: TurnStatus::InProgress,
                error: None,
                started_at: Some(0),
                completed_at: None,
                duration_ms: None,
            },
        }),
        /*replay_kind*/ None,
    );
    assert!(app.chat_widget.is_user_turn_pending_or_running());

    app.update_full_transparency(&mut tui, /*enabled*/ true)
        .await;

    assert!(!app.config.tui_full_transparency);
    assert!(!app.chat_widget.config_ref().tui_full_transparency);
    assert!(!tui.terminal.full_transparency());
    assert!(!codex_home.path().join("config.toml").exists());
    let event = app_event_rx
        .try_recv()
        .expect("expected task-running transparency error");
    let AppEvent::InsertHistoryCell(cell) = event else {
        panic!("expected task-running history cell, got {event:?}");
    };
    let rendered = cell
        .display_lines(/*width*/ 80)
        .iter()
        .flat_map(|line| line.spans.iter())
        .map(|span| span.content.as_ref())
        .collect::<String>();
    assert_eq!(
        rendered,
        "■ '/transparent' is disabled while a task is in progress."
    );
    Ok(())
}

#[tokio::test]
async fn update_full_transparency_preserves_pending_history_when_transcript_is_empty() -> Result<()>
{
    let (mut app, _app_event_rx) = make_test_app_with_event_receiver().await;
    let codex_home = tempdir()?;
    app.config.codex_home = codex_home.path().to_path_buf().abs();
    assert!(app.transcript_cells.is_empty());
    let mut tui = crate::tui::test_support::make_test_tui()?;
    let pending_history = vec![Line::from("Queued session header.")];
    tui.insert_history_lines(pending_history.clone());

    app.update_full_transparency(&mut tui, /*enabled*/ true)
        .await;

    assert!(app.config.tui_full_transparency);
    assert!(app.chat_widget.config_ref().tui_full_transparency);
    assert!(tui.terminal.full_transparency());
    assert_eq!(
        crate::tui::test_support::pending_history_lines(&tui),
        pending_history
    );
    Ok(())
}

#[tokio::test]
async fn update_full_transparency_does_not_reflow_when_persist_fails() -> Result<()> {
    let (mut app, mut app_event_rx) = make_test_app_with_event_receiver().await;
    let home = tempdir()?;
    // Match ConfigEditsBuilder::for_config: with no user layer it writes
    // `$CODEX_HOME/config.toml`. Clear layers and put a directory at that path so the
    // write fails on every platform.
    app.config.config_layer_stack = ConfigLayerStack::default();
    app.config.codex_home = home.path().to_path_buf().abs();
    std::fs::create_dir(home.path().join("config.toml"))?;
    assert!(
        app.config
            .config_layer_stack
            .get_user_config_file()
            .is_none(),
        "persist path must be controlled by codex_home for this test"
    );
    app.transcript_cells
        .push(Arc::new(history_cell::new_info_event(
            "Existing transcript entry.".to_string(),
            /*hint*/ None,
        )));
    let mut tui = crate::tui::test_support::make_test_tui()?;

    force_next_reflow_failure_for_test();
    app.update_full_transparency(&mut tui, /*enabled*/ true)
        .await;

    assert!(!app.config.tui_full_transparency);
    assert!(!app.chat_widget.config_ref().tui_full_transparency);
    assert!(!tui.terminal.full_transparency());
    assert!(
        take_forced_reflow_failure(),
        "persistence failure must prevent destructive transcript reflow"
    );
    assert!(!app.has_emitted_history_lines);
    let event = app_event_rx
        .try_recv()
        .expect("expected persist-failure history cell");
    let AppEvent::InsertHistoryCell(cell) = event else {
        panic!("expected history cell, got {event:?}");
    };
    let rendered = cell
        .display_lines(/*width*/ 80)
        .iter()
        .flat_map(|line| line.spans.iter())
        .map(|span| span.content.as_ref())
        .collect::<String>();
    assert!(
        rendered.contains("Failed to save full transparency setting:"),
        "expected persist failure message, got {rendered:?}"
    );
    Ok(())
}

#[tokio::test]
async fn update_full_transparency_restores_absent_user_value_when_reflow_fails() -> Result<()> {
    let (mut app, mut app_event_rx) = make_test_app_with_event_receiver().await;
    let codex_home = tempdir()?;
    app.config.codex_home = codex_home.path().to_path_buf().abs();
    app.transcript_cells
        .push(Arc::new(history_cell::AgentMessageCell::new(
            vec![Line::from("Transient streamed answer.")],
            /*is_first_line*/ true,
        )));
    let mut tui = crate::tui::test_support::make_test_tui()?;

    // Fail only the first reflow (the apply attempt). Restore reflow succeeds.
    force_next_reflow_failure_for_test();
    app.update_full_transparency(&mut tui, /*enabled*/ true)
        .await;

    assert!(!app.config.tui_full_transparency);
    assert!(!app.chat_widget.config_ref().tui_full_transparency);
    assert!(!tui.terminal.full_transparency());
    let config = std::fs::read_to_string(codex_home.path().join("config.toml"))?;
    let config: toml::Value = toml::from_str(&config)?;
    let persisted = config
        .get("tui")
        .and_then(toml::Value::as_table)
        .and_then(|tui| tui.get("full_transparency"));
    assert_eq!(persisted, None);
    assert!(app.transcript_reflow.take_stream_finish_reflow_needed());
    let event = app_event_rx
        .try_recv()
        .expect("expected reflow-failure history cell");
    let AppEvent::InsertHistoryCell(cell) = event else {
        panic!("expected history cell, got {event:?}");
    };
    let rendered = cell
        .display_lines(/*width*/ 80)
        .iter()
        .flat_map(|line| line.spans.iter())
        .map(|span| span.content.as_ref())
        .collect::<String>();
    assert!(
        rendered.contains("Failed to redraw transcript:"),
        "expected reflow failure message, got {rendered:?}"
    );
    Ok(())
}

#[tokio::test]
async fn update_full_transparency_restores_user_value_masked_by_effective_override() -> Result<()> {
    let (mut app, _app_event_rx) = make_test_app_with_event_receiver().await;
    let codex_home = tempdir()?;
    let config_path = codex_home.path().join("config.toml").abs();
    let original = "[tui]\nfull_transparency = false # keep user preference\n";
    std::fs::write(&config_path, original)?;
    app.config.codex_home = codex_home.path().to_path_buf().abs();
    app.config.config_layer_stack =
        ConfigLayerStack::default().with_user_config(&config_path, toml::from_str(original)?)?;
    app.transcript_cells
        .push(Arc::new(history_cell::new_info_event(
            "Existing transcript entry.".to_string(),
            /*hint*/ None,
        )));
    let mut tui = crate::tui::test_support::make_test_tui()?;
    // Simulate an effective `true` supplied by a higher-precedence layer while
    // the writable user layer retains its own `false` preference.
    app.set_runtime_full_transparency(&mut tui, /*enabled*/ true);

    force_next_reflow_failure_for_test();
    app.update_full_transparency(&mut tui, /*enabled*/ true)
        .await;

    assert!(app.config.tui_full_transparency);
    assert!(app.chat_widget.config_ref().tui_full_transparency);
    assert!(tui.terminal.full_transparency());
    assert_eq!(std::fs::read_to_string(config_path)?, original);
    Ok(())
}
