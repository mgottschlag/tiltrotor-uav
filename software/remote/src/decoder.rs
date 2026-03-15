use bytes::Buf;
use bytes::Bytes;
use bytes::BytesMut;
use tokio_util::codec::Decoder;

pub struct FrameDecoder {}

impl Decoder for FrameDecoder {
    type Item = Bytes;
    type Error = std::io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> std::io::Result<Option<Self::Item>> {
        if buf.len() < 1 {
            return Ok(None);
        }
        let frame_len = buf[0] as usize;
        if buf.len() < 1 + frame_len {
            return Ok(None);
        }

        buf.advance(1);
        let data = buf.split_to(frame_len);
        Ok(Some(data.freeze()))
    }
}
