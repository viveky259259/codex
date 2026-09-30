//! Restores saved side conversations and their parent navigation after a restart.

use super::*;

impl App {
    pub(super) async fn attach_resumed_thread(
        &mut self,
        tui: &mut tui::Tui,
        app_server: &mut AppServerSession,
        started: AppServerStartedThread,
    ) -> Result<()> {
        // The resume RPC succeeded, so this attachment can receive live events again.
        self.abandoned_side_threads
            .remove(&started.session.thread_id);
        let Some(parent_thread_id) = started.side_parent_thread_id else {
            return self
                .replace_chat_widget_with_app_server_thread(
                    tui,
                    started,
                    session_lifecycle::ThreadAttachPresentation::SessionLineage,
                    /*initial_user_message*/ None,
                )
                .await;
        };
        let side_thread_id = started.session.thread_id;
        match app_server
            .resume_thread(
                &self.local_settings,
                self.config.clone(),
                parent_thread_id,
                crate::app_server_session::ResumeModelSettings::PreserveExistingThread,
            )
            .await
        {
            Ok(parent) => {
                self.replace_chat_widget_with_app_server_thread(
                    tui,
                    parent,
                    session_lifecycle::ThreadAttachPresentation::SessionLineage,
                    /*initial_user_message*/ None,
                )
                .await?;
                self.ensure_thread_channel(side_thread_id)
                    .store
                    .lock()
                    .await
                    .set_session(started.session, started.turns);
                self.side_threads
                    .insert(side_thread_id, SideThreadState::new(parent_thread_id));
                self.upsert_agent_picker_thread(
                    side_thread_id,
                    /*agent_nickname*/ None,
                    /*agent_role*/ None,
                    /*is_closed*/ false,
                );
                self.select_agent_thread(tui, app_server, side_thread_id)
                    .await
            }
            Err(error) => {
                // A deleted or unavailable parent must not prevent reading the saved side chat.
                self.replace_chat_widget_with_app_server_thread(
                    tui,
                    started,
                    session_lifecycle::ThreadAttachPresentation::SessionLineage,
                    /*initial_user_message*/ None,
                )
                .await?;
                self.side_threads
                    .insert(side_thread_id, SideThreadState::new(parent_thread_id));
                self.sync_side_thread_ui();
                self.chat_widget.add_error_message(format!(
                    "Could not reopen the side conversation's parent: {error}. The side conversation will still be archived when you exit."
                ));
                Ok(())
            }
        }
    }
}
