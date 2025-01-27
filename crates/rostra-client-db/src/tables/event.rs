use std::borrow::Cow;

use bincode::{Decode, Encode};
use rostra_core::event::{EventContentUnsized, SignedEvent};
use rostra_core::ShortEventId;

#[derive(Debug, Encode, Decode, Clone)]
pub enum EventContentState<'a> {
    Present(Cow<'a, EventContentUnsized>),
    // Deleted as requested by the author
    Deleted {
        // Notably: we only store one ShortEventId somewhat opportunistically
        // If multiple events pointed at the same parent event to be deleted,
        // we might return any.
        deleted_by: ShortEventId,
    },
    Pruned,
}

pub type EventContentStateOwned = EventContentState<'static>;

#[derive(Debug, Encode, Decode, Clone)]
pub struct EventRecord {
    pub signed: SignedEvent,
}

#[derive(Decode, Encode, Debug)]
pub struct EventsMissingRecord {
    // Notably: we only store one ShortEventId somewhat opportunistically
    // If multiple events pointed at the same parent event to be deleted,
    // we might return any.
    pub deleted_by: Option<ShortEventId>,
}

#[derive(Decode, Encode, Debug)]
pub struct EventsHeadsTableRecord;
