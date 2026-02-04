use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use crate::message::TunnelMessage;

const MAX_FRAME_SIZE: usize = 10 * 1024 * 1024; // 10MB

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("frame too large: {0} bytes")]
    FrameTooLarge(usize),
    #[error("deserialization failed: {0}")]
    Deserialize(String),
    #[error("serialization failed: {0}")]
    Serialize(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct TunnelCodec;

impl Decoder for TunnelCodec {
    type Item = TunnelMessage;
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            return Ok(None);
        }

        let len = u32::from_be_bytes(src[..4].try_into().unwrap()) as usize;

        if len > MAX_FRAME_SIZE {
            return Err(ProtocolError::FrameTooLarge(len));
        }

        if src.len() < 4 + len {
            src.reserve(4 + len - src.len());
            return Ok(None);
        }

        src.advance(4);
        let payload = src.split_to(len);

        let msg: TunnelMessage = bincode::deserialize(&payload)
            .map_err(|e| ProtocolError::Deserialize(e.to_string()))?;

        Ok(Some(msg))
    }
}

impl Encoder<TunnelMessage> for TunnelCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: TunnelMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let payload =
            bincode::serialize(&item).map_err(|e| ProtocolError::Serialize(e.to_string()))?;

        if payload.len() > MAX_FRAME_SIZE {
            return Err(ProtocolError::FrameTooLarge(payload.len()));
        }

        dst.reserve(4 + payload.len());
        dst.put_u32(payload.len() as u32);
        dst.put_slice(&payload);
        Ok(())
    }
}
