//! Regression coverage for metadata reads without SQLite.

use std::fs;
use std::fs::FileTimes;

use chrono::DateTime;
use chrono::Utc;
use codex_protocol::ThreadId;
use codex_protocol::protocol::SessionMeta;
use codex_protocol::protocol::SessionMetaLine;
use codex_protocol::protocol::ThreadSource;
use codex_rollout::RolloutItem;
use codex_rollout::RolloutLine;
use pretty_assertions::assert_eq;
use tempfile::TempDir;

use super::LocalThreadStore;
use super::test_support::test_config;
use crate::ReadThreadParams;
use crate::ThreadStore;

#[tokio::test]
async fn archived_side_sources_survive_nonempty_legacy_reads_and_listing()
-> Result<(), Box<dyn std::error::Error>> {
    for with_sqlite in [false, true] {
        let home = TempDir::new()?;
        let config = test_config(home.path());
        let state_db = if with_sqlite {
            Some(
                codex_state::StateRuntime::init(
                    config.sqlite.clone(),
                    config.default_model_provider_id.clone(),
                )
                .await?,
            )
        } else {
            None
        };
        let store = LocalThreadStore::new(config, state_db);
        let uuid = uuid::Uuid::new_v4();
        let thread_id = ThreadId::from_string(&uuid.to_string())?;
        let path = super::test_support::write_archived_session_file(
            home.path(),
            "2025-01-03T12-00-00",
            uuid,
        )?;
        let contents = fs::read_to_string(&path)?;
        let (header, history) = contents.split_once('\n').expect("rollout header");
        let mut header: serde_json::Value = serde_json::from_str(header)?;
        header["payload"]["thread_source"] = serde_json::json!("side_conversation");
        fs::write(&path, format!("{header}\n{history}"))?;
        let expected_source = Some(ThreadSource::Feature("side_conversation".into()));

        let read = store
            .read_thread(ReadThreadParams {
                thread_id,
                include_archived: true,
                include_history: false,
            })
            .await?;
        assert_eq!(read.thread_source, expected_source);

        let params = crate::ListThreadsParams {
            page_size: 10,
            cursor: None,
            sort_key: crate::ThreadSortKey::CreatedAt,
            sort_direction: crate::SortDirection::Desc,
            allowed_sources: Vec::new(),
            model_providers: None,
            cwd_filters: None,
            section: None,
            project_id: None,
            archived: true,
            search_term: None,
            relation_filter: None,
            use_state_db_only: false,
        };
        let listed = store.list_threads(params.clone()).await?;
        assert_eq!(listed.items.len(), 1);
        assert_eq!(listed.items[0].thread_source, expected_source);
        if with_sqlite {
            let listed = store
                .list_threads(crate::ListThreadsParams {
                    use_state_db_only: true,
                    ..params
                })
                .await?;
            assert_eq!(listed.items[0].thread_source, expected_source);
        }
        let searched = store
            .search_threads(crate::SearchThreadsParams {
                page_size: 10,
                cursor: None,
                sort_key: crate::ThreadSortKey::CreatedAt,
                sort_direction: crate::SortDirection::Desc,
                allowed_sources: Vec::new(),
                archived: true,
                search_term: "Archived user message".into(),
            })
            .await?;
        assert_eq!(searched.items.len(), 1);
        assert_eq!(searched.items[0].thread.thread_source, expected_source);
    }
    Ok(())
}

#[tokio::test]
async fn empty_archived_reads_without_sqlite_preserve_source_and_file_time()
-> Result<(), Box<dyn std::error::Error>> {
    let home = TempDir::new()?;
    let store = LocalThreadStore::new(test_config(home.path()), /*state_db*/ None);
    let thread_id = ThreadId::new();
    let timestamp = "2026-07-09T00:00:00Z";
    let created_at = DateTime::parse_from_rfc3339(timestamp)?.with_timezone(&Utc);
    let updated_at = DateTime::parse_from_rfc3339("2026-07-10T00:00:00Z")?.with_timezone(&Utc);
    let directory = home.path().join(codex_rollout::ARCHIVED_SESSIONS_SUBDIR);
    fs::create_dir_all(&directory)?;
    let path = directory.join(format!("rollout-2026-07-09T00-00-00-{thread_id}.jsonl"));
    let line = RolloutLine {
        timestamp: timestamp.to_string(),
        ordinal: None,
        item: RolloutItem::SessionMeta(SessionMetaLine {
            meta: SessionMeta {
                id: thread_id,
                session_id: thread_id.into(),
                timestamp: timestamp.to_string(),
                thread_source: Some(ThreadSource::User),
                ..SessionMeta::default()
            },
            git: None,
        }),
    };
    fs::write(&path, format!("{}\n", serde_json::to_string(&line)?))?;
    fs::OpenOptions::new()
        .write(true)
        .open(&path)?
        .set_times(FileTimes::new().set_modified(updated_at.into()))?;

    let by_id = store
        .read_thread(ReadThreadParams {
            thread_id,
            include_archived: true,
            include_history: false,
        })
        .await?;
    let by_path = store
        .read_thread_by_rollout_path(
            path, /*include_archived*/ true, /*include_history*/ false,
        )
        .await?;
    for thread in [by_id, by_path] {
        assert_eq!(
            (
                thread.thread_id,
                thread.thread_source,
                thread.preview,
                thread.created_at,
                thread.updated_at,
                thread.recency_at,
                thread.archived_at
            ),
            (
                thread_id,
                Some(ThreadSource::User),
                String::new(),
                created_at,
                updated_at,
                updated_at,
                Some(updated_at)
            ),
        );
    }
    Ok(())
}
