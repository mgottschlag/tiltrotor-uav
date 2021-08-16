use core::convert::Infallible;
use cortex_m_semihosting::hprintln;
use embedded_nrf24l01::{self, Configuration, CrcMode, DataRate, TxMode, NRF24L01};
use serde::Serialize;
use serde_cbor::de::from_mut_slice;
use serde_cbor::ser::SliceWrite;
use serde_cbor::Serializer;

pub use crate::board::{RadioCe, RadioCs, RadioIrq, RadioSpi};
use crate::protocol;

pub struct Radio {
    tx: TxMode<NRF24L01<Infallible, RadioCe, RadioCs, RadioSpi>>,
}

impl Radio {
    pub fn init(spi: RadioSpi, cs: RadioCs, ce: RadioCe, _irq: RadioIrq) -> Self {
        let mut nrf = NRF24L01::new(ce, cs, spi).unwrap();
        nrf.set_frequency(13).unwrap();
        nrf.set_auto_retransmit(6, 15).unwrap();
        nrf.set_rf(&DataRate::R2Mbps, 0).unwrap();
        nrf.set_pipes_rx_enable(&[true, true, false, false, false, false])
            .unwrap();
        nrf.set_auto_ack(&[true; 6]).unwrap();
        nrf.set_crc(CrcMode::OneByte).unwrap();
        nrf.set_pipes_rx_lengths(&[None; 6], true).unwrap();
        // TODO: set proper addresses
        nrf.set_rx_addr(0, &b"aaaaa"[..]).unwrap();
        nrf.set_rx_addr(1, &b"aaaab"[..]).unwrap();
        nrf.set_tx_addr(&b"aaaaa"[..]).unwrap();

        let tx = nrf.tx().unwrap();
        return Radio { tx };
    }

    pub fn send_status(&mut self, data: &protocol::Status) -> Option<protocol::Command> {
        let mut buf = [0u8; 32];
        let writer = SliceWrite::new(&mut buf[..]);
        let mut ser = Serializer::new(writer);
        data.serialize(&mut ser).ok();
        let writer = ser.into_inner();
        let size = writer.bytes_written();

        let mut ack_payload = heapless::Vec::<u8, 32>::new();
        self.tx.send(&buf[..size], Some(0)).unwrap();
        loop {
            match self.tx.poll_send(&mut ack_payload) {
                Err(nb::Error::WouldBlock) => {}
                Err(nb::Error::Other(_)) => {
                    hprintln!("break").ok();
                    break;
                }
                Ok(success) => {
                    if !success {
                        hprintln!("Failed to send").ok();
                    } else {
                        //hprintln!("AP: {:?}", ack_payload.len()).ok();
                        //hprintln!("AP: {:?}", ack_payload).ok();
                        if ack_payload.len() > 1 {
                            let cmd: protocol::Command =
                                from_mut_slice(&mut ack_payload[..]).unwrap();
                            //hprintln!("AP: {:?}", cmd.e).ok();
                            return Some(cmd);
                        }
                    }
                    break;
                }
            }
        }

        return None;
    }
}
