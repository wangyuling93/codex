//! Persist and apply fully transparent TUI backgrounds.

use super::*;
use crate::legacy_core::config::edit::ConfigEdit;

impl App {
    fn set_runtime_full_transparency(&mut self, tui: &mut tui::Tui, enabled: bool) {
        self.config.tui_full_transparency = enabled;
        self.chat_widget.set_full_transparency(enabled);
        tui.set_full_transparency(enabled);
    }

    fn restore_full_transparency_after_failure(&mut self, tui: &mut tui::Tui, previous: bool) {
        self.set_runtime_full_transparency(tui, previous);
        if let Err(reflow_err) = self.reflow_for_full_transparency(tui) {
            tracing::warn!(
                error = %reflow_err,
                "failed to reflow transcript after rolling back full transparency"
            );
        }
    }

    fn reflow_for_full_transparency(&mut self, tui: &mut tui::Tui) -> Result<()> {
        if self.transcript_cells.is_empty() {
            return Ok(());
        }

        #[cfg(test)]
        if take_forced_reflow_failure() {
            return Err(color_eyre::eyre::eyre!(
                "forced reflow failure for full transparency test"
            ));
        }

        let reflow_ran_during_stream = self.should_mark_reflow_as_stream_time();
        self.reflow_transcript_now(tui)?;
        if reflow_ran_during_stream {
            self.transcript_reflow.mark_ran_during_stream();
        }
        Ok(())
    }

    async fn read_stored_full_transparency(&self) -> anyhow::Result<Option<bool>> {
        let config_path = self
            .config
            .config_layer_stack
            .get_user_config_file()
            .map(codex_utils_absolute_path::AbsolutePathBuf::to_path_buf)
            .unwrap_or_else(|| {
                self.config
                    .codex_home
                    .join(codex_config::CONFIG_TOML_FILE)
                    .to_path_buf()
            });
        let serialized = match tokio::fs::read_to_string(config_path).await {
            Ok(serialized) => serialized,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err.into()),
        };
        let config = toml::from_str::<toml::Value>(&serialized)?;
        Ok(config
            .get("tui")
            .and_then(toml::Value::as_table)
            .and_then(|tui| tui.get("full_transparency"))
            .and_then(toml::Value::as_bool))
    }

    async fn apply_full_transparency_edit(&self, edit: ConfigEdit) -> anyhow::Result<()> {
        ConfigEditsBuilder::for_config(&self.config)
            .with_edits([edit])
            .apply()
            .await
    }

    pub(super) async fn update_full_transparency(&mut self, tui: &mut tui::Tui, enabled: bool) {
        if self.chat_widget.is_user_turn_pending_or_running() {
            self.chat_widget.add_error_message(
                "'/transparent' is disabled while a task is in progress.".to_string(),
            );
            return;
        }

        let previous = self.config.tui_full_transparency;
        let stored = match self.read_stored_full_transparency().await {
            Ok(stored) => stored,
            Err(err) => {
                tracing::error!(error = %err, "failed to read full transparency setting");
                self.chat_widget
                    .add_error_message(format!("Failed to save full transparency setting: {err}"));
                return;
            }
        };

        let edit = crate::legacy_core::config::edit::full_transparency_edit(enabled);
        if let Err(err) = self.apply_full_transparency_edit(edit).await {
            tracing::error!(error = %err, "failed to persist full transparency setting");
            self.chat_widget
                .add_error_message(format!("Failed to save full transparency setting: {err}"));
            return;
        }

        self.set_runtime_full_transparency(tui, enabled);
        if let Err(err) = self.reflow_for_full_transparency(tui) {
            let rollback_edit = match stored {
                Some(stored) => crate::legacy_core::config::edit::full_transparency_edit(stored),
                None => ConfigEdit::ClearPath {
                    segments: vec!["tui".to_string(), "full_transparency".to_string()],
                },
            };
            if let Err(rollback_err) = self.apply_full_transparency_edit(rollback_edit).await {
                tracing::error!(
                    error = %rollback_err,
                    "failed to restore full transparency setting after redraw failure"
                );
                self.chat_widget.add_error_message(format!(
                    "Failed to restore full transparency setting after redraw failure: {rollback_err}"
                ));
            } else {
                self.restore_full_transparency_after_failure(tui, previous);
            }
            tracing::warn!(
                error = %err,
                "failed to reflow transcript after full transparency toggle"
            );
            self.chat_widget
                .add_error_message(format!("Failed to redraw transcript: {err}"));
            return;
        }

        let message = if enabled {
            "Full transparency enabled."
        } else {
            "Full transparency disabled."
        };
        self.chat_widget
            .add_info_message(message.to_string(), /*hint*/ None);
    }
}

#[cfg(test)]
fn take_forced_reflow_failure() -> bool {
    FORCE_REFLOW_FAILURES_REMAINING.with(|remaining| {
        let current = remaining.get();
        if current == 0 {
            return false;
        }
        remaining.set(current - 1);
        true
    })
}

#[cfg(test)]
fn force_next_reflow_failure_for_test() {
    FORCE_REFLOW_FAILURES_REMAINING.with(|remaining| remaining.set(1));
}

#[cfg(test)]
thread_local! {
    static FORCE_REFLOW_FAILURES_REMAINING: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
#[path = "full_transparency_tests.rs"]
mod tests;
