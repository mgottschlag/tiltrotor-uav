use bytes::Buf;
use bytes::Bytes;
use bytes::BytesMut;
use tokio_util::codec::Decoder;

pub struct FrameDecoder {}

impl Decoder for FrameDecoder {
    type Item = Bytes;
    type Error = std::io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> std::io::Result<Option<Self::Item>> {
        println!("buf len: {}", buf.len());
        if buf.len() < 1 {
            return Ok(None);
        }
        let frame_len = buf[0] as usize;
        println!("packet_len={frame_len}");
        if buf.len() < 1 + frame_len {
            return Ok(None);
        }

        buf.advance(1);
        let data = buf.split_to(frame_len);
        println!(
            "Splitted: first={}, last={}",
            data.first().unwrap(),
            data.last().unwrap()
        );
        Ok(Some(data.freeze()))
    }
}
