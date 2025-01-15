use std::time::{SystemTime, UNIX_EPOCH};

use convi::ExpectInto as _;

use super::{Event, EventContent, EventContentData, EventKind, SignedEvent};
use crate::bincode::STD_BINCODE_CONFIG;
use crate::id::RostraId;
use crate::{ContentHash, EventId, MsgLen, ShortEventId};

impl EventContent {
    pub fn compute_content_hash(&self) -> ContentHash {
        blake3::hash(&self.0).into()
    }
}

#[bon::bon]
impl Event {
    #[builder]
    pub fn new(
        author: RostraId,
        delete: Option<ShortEventId>,
        kind: impl Into<EventKind>,
        parent_prev: Option<ShortEventId>,
        parent_aux: Option<ShortEventId>,
        timestamp: Option<SystemTime>,
        content: EventContent,
    ) -> Self {
        if delete.is_some() && parent_aux.is_some() {
            panic!("Can't set both both delete and parent_aux");
        }

        let replace = delete.map(Into::into);
        let parent_prev = parent_prev.map(Into::into);
        let parent_aux = parent_aux.map(Into::into);

        let timestamp = timestamp
            .unwrap_or_else(SystemTime::now)
            .duration_since(UNIX_EPOCH)
            .expect("Dates before Unix epoch are unsupported")
            .as_secs();

        Self {
            version: 0,
            flags: if replace.is_some() { 1 } else { 0 },
            kind: kind.into(),
            content_len: MsgLen(content.len().expect_into()),
            padding: [0; 16],
            timestamp: timestamp.into(),
            author,
            parent_prev: parent_prev.unwrap_or_default(),
            parent_aux: parent_aux.or(replace).unwrap_or_default(),
            content_hash: content.compute_content_hash(),
        }
    }

    pub fn compute_id(&self) -> EventId {
        let encoded =
            ::bincode::encode_to_vec(self, STD_BINCODE_CONFIG).expect("Can't fail encoding");
        blake3::hash(&encoded).into()
    }

    pub fn compute_short_id(&self) -> ShortEventId {
        self.compute_id().into()
    }
}

impl SignedEvent {
    pub fn compute_id(&self) -> EventId {
        self.event.compute_id()
    }
    pub fn compute_short_id(&self) -> ShortEventId {
        self.event.compute_id().into()
    }
}

impl<'a, 'de: 'a> bincode::BorrowDecode<'de> for &'a EventContentData {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        let bytes_ref: &[u8] = bincode::BorrowDecode::borrow_decode(decoder)?;
        let ptr = bytes_ref as *const [u8] as *const EventContentData;
        Ok(unsafe { &*ptr })
    }
}

impl EventContent {
    pub fn decode<T>(&self) -> Result<T, ::bincode::error::DecodeError>
    where
        T: ::bincode::Decode,
    {
        let res = ::bincode::decode_from_slice(&self.0, STD_BINCODE_CONFIG)?;
        Ok(res.0)
    }
}

#[cfg(test)]
mod tests;
