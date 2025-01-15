use redb_bincode::WriteTransaction;
use rostra_core::event::{
    ContentFollow, ContentUnfollow, EventContent, EventKind, VerifiedEvent, VerifiedEventContent,
};
use rostra_core::id::RostraId;
use rostra_core::ShortEventId;
use rostra_util_error::FmtCompact as _;
use tokio::sync::watch;
use tracing::debug;

use crate::db::{Database, DbResult, InsertEventOutcome, TABLE_EVENTS_HEADS, TABLE_EVENTS_MISSING};

pub struct Storage {
    db: Database,
    self_followee_list_updated: watch::Sender<Vec<RostraId>>,
}

pub const LOG_TARGET: &str = "rostra::storage";

impl Storage {
    const MAX_CONTENT_LEN: u32 = 1_000_000u32;

    pub async fn new(db: Database, self_id: RostraId) -> DbResult<Self> {
        let self_followees = db
            .read_followees(self_id)
            .await?
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        let (self_followee_list_updated, _) = watch::channel(self_followees);
        Ok(Self {
            db,
            self_followee_list_updated,
        })
    }

    pub fn self_followees_list_subscribe(&self) -> watch::Receiver<Vec<RostraId>> {
        self.self_followee_list_updated.subscribe()
    }

    pub async fn has_event(&self, event_id: impl Into<ShortEventId>) -> bool {
        let event_id = event_id.into();
        self.db
            .read_with(|tx| {
                let events_table = tx
                    .open_table(&crate::db::TABLE_EVENTS)
                    .expect("Storage error");
                Database::has_event_tx(event_id, &events_table)
            })
            .await
            .expect("Database panic")
    }

    pub async fn get_event(
        &self,
        event_id: impl Into<ShortEventId>,
    ) -> Option<crate::db::events::EventRecord> {
        let event_id = event_id.into();
        self.db
            .read_with(|tx| {
                let events_table = tx.open_table(&crate::db::TABLE_EVENTS)?;
                Database::get_event_tx(event_id, &events_table)
            })
            .await
            .expect("Database panic")
    }

    pub async fn get_event_content(
        &self,
        event_id: impl Into<ShortEventId>,
    ) -> Option<EventContent> {
        let event_id = event_id.into();
        self.db
            .read_with(|tx| {
                let events_content_table = tx.open_table(&crate::db::TABLE_EVENTS_CONTENT)?;
                Ok(
                    Database::get_event_content_tx(event_id, &events_content_table)?.and_then(
                        |content_state| match content_state {
                            crate::db::events::ContentStateRef::Present(b) => Some(b.into_owned()),
                            crate::db::events::ContentStateRef::Deleted { .. }
                            | crate::db::events::ContentStateRef::Pruned => None,
                        },
                    ),
                )
            })
            .await
            .expect("Database panic")
    }
    pub async fn process_event(
        &self,
        event: &VerifiedEvent,
    ) -> (InsertEventOutcome, ProcessEventState) {
        self.db
            .write_with(|tx| {
                let mut events_table = tx.open_table(&crate::db::TABLE_EVENTS)?;
                let mut events_content_table = tx.open_table(&crate::db::TABLE_EVENTS_CONTENT)?;
                let mut events_missing_table = tx.open_table(&TABLE_EVENTS_MISSING)?;
                let mut events_heads_table = tx.open_table(&TABLE_EVENTS_HEADS)?;

                let insert_event_outcome = Database::insert_event_tx(
                    event,
                    &mut events_table,
                    &mut events_content_table,
                    &mut events_missing_table,
                    &mut events_heads_table,
                )?;

                let process_event_content_state = if Self::MAX_CONTENT_LEN
                    < u32::from(event.event.content_len)
                {
                    Database::prune_event_content_tx(event.event_id, &mut events_content_table)?;

                    ProcessEventState::Pruned
                } else {
                    match insert_event_outcome {
                        InsertEventOutcome::AlreadyPresent => ProcessEventState::Existing,
                        InsertEventOutcome::Inserted { is_deleted, .. } => {
                            if is_deleted {
                                ProcessEventState::Deleted
                            } else {
                                // If the event was not there, and it wasn't deleted
                                // it definitely does not have content yet.
                                ProcessEventState::New
                            }
                        }
                    }
                };
                Ok((insert_event_outcome, process_event_content_state))
            })
            .await
            .expect("Storage error")
    }

    /// Process event content
    ///
    /// Note: Must only be called for an event that was already processed
    pub async fn process_event_content(&self, event_content: &VerifiedEventContent) {
        self.db
            .write_with(|tx| {
                let events_table = tx.open_table(&crate::db::TABLE_EVENTS)?;
                let mut events_content_table = tx.open_table(&crate::db::TABLE_EVENTS_CONTENT)?;

                debug_assert!(Database::has_event_tx(
                    event_content.event_id,
                    &events_table
                )?);

                let content_added =
                    if u32::from(event_content.event.content_len) > Self::MAX_CONTENT_LEN {
                        Database::insert_event_content_tx(event_content, &mut events_content_table)?
                    } else {
                        false
                    };

                if content_added {
                    self.process_event_content_inserted_tx(event_content, tx)?;
                }
                Ok(())
            })
            .await
            .expect("Storage error")
    }

    /// After an event content was inserted process special kinds of event
    /// content, like follows/unfollows
    pub fn process_event_content_inserted_tx(
        &self,
        event_content: &VerifiedEventContent,
        tx: &WriteTransaction,
    ) -> DbResult<()> {
        match event_content.event.kind {
            EventKind::FOLLOW | EventKind::UNFOLLOW => {
                let mut id_followees_table = tx.open_table(&crate::db::TABLE_ID_FOLLOWEES)?;
                let mut id_followers_table = tx.open_table(&crate::db::TABLE_ID_FOLLOWERS)?;
                let mut id_unfollowed_table = tx.open_table(&crate::db::TABLE_ID_UNFOLLOWED)?;

                match event_content.event.kind {
                    EventKind::FOLLOW => match event_content.content.decode::<ContentFollow>() {
                        Ok(follow_content) => {
                            Database::insert_follow_tx(
                                event_content.event.author,
                                event_content.event.timestamp.into(),
                                follow_content,
                                &mut id_followees_table,
                                &mut id_followers_table,
                                &mut id_unfollowed_table,
                            )?;
                        }
                        Err(err) => {
                            debug!(target: LOG_TARGET, err = %err.fmt_compact(), "Ignoring malformed ContentFollow payload");
                        }
                    },
                    EventKind::UNFOLLOW => {
                        match event_content.content.decode::<ContentUnfollow>() {
                            Ok(unfollow_content) => {
                                Database::insert_unfollow_tx(
                                    event_content.event.author,
                                    event_content.event.timestamp.into(),
                                    unfollow_content,
                                    &mut id_followees_table,
                                    &mut id_followers_table,
                                    &mut id_unfollowed_table,
                                )?;
                            }
                            Err(err) => {
                                debug!(target: LOG_TARGET, err = %err.fmt_compact(), "Ignoring malformed ContentUnfollow payload");
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
            _ => {}
        };
        Ok(())
    }

    pub async fn wants_content(
        &self,
        event_id: impl Into<ShortEventId>,
        process_state: ProcessEventState,
    ) -> bool {
        match process_state.wants_content() {
            ContentWantState::DoesNotWant => {
                return false;
            }
            ContentWantState::Wants => {
                return true;
            }
            ContentWantState::MaybeWants => {}
        }

        self.db
            .read_with(|tx| {
                let events_content_table = tx.open_table(&crate::db::TABLE_EVENTS_CONTENT)?;

                Database::has_event_content_tx(event_id.into(), &events_content_table)
            })
            .await
            .expect("Storage error")
    }
}

pub enum ProcessEventState {
    New,
    Existing,
    Pruned,
    Deleted,
}

pub enum ContentWantState {
    Wants,
    MaybeWants,
    DoesNotWant,
}

impl ProcessEventState {
    pub fn wants_content(self) -> ContentWantState {
        match self {
            ProcessEventState::New => ContentWantState::Wants,
            ProcessEventState::Existing => ContentWantState::MaybeWants,
            ProcessEventState::Pruned => ContentWantState::DoesNotWant,
            ProcessEventState::Deleted => ContentWantState::DoesNotWant,
        }
    }
}
