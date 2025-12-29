use bytes::Buf;
use bytes::BytesMut;
use tokio_util::codec::Decoder;

pub struct FrameDecoder {}

impl Decoder for FrameDecoder {
    type Item = BytesMut;
    type Error = std::io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> std::io::Result<Option<Self::Item>> {
        if buf.len() < 1 {
            return Ok(None);
        }
        let packet_len = buf[0] as usize;
        println!("packet_len={packet_len}");
        if buf.len() < packet_len as usize + 1 {
            return Ok(None);
        }

        buf.advance(1);
        Ok(Some(buf.split_to(packet_len as usize)))
    }
}
