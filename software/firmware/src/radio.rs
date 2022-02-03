#[cfg(not(feature = "flightcontroller"))]
use core::convert::Infallible;
use embedded_nrf24l01::{self, Configuration, CrcMode, DataRate, RxMode, NRF24L01};
use rtt_target::rprintln;
use serde::Serialize;
use serde_cbor::de::from_mut_slice;
use serde_cbor::ser::SliceWrite;
use serde_cbor::Serializer;

pub use crate::board::{RadioCe, RadioCs, RadioInterruptType, RadioIrq, RadioSpi};

pub struct Radio {
    #[cfg(feature = "flightcontroller")]
    rx: RxMode<NRF24L01<(), RadioCe, RadioCs, RadioSpi>>,
    #[cfg(not(feature = "flightcontroller"))]
    rx: RxMode<NRF24L01<Infallible, RadioCe, RadioCs, RadioSpi>>,
}

impl Radio {
    pub fn init(spi: RadioSpi, cs: RadioCs, ce: RadioCe) -> Self {
        let mut nrf = NRF24L01::new(ce, cs, spi).unwrap();

        // configure radio
        // - channel/frequency = 0x32 (50)
        // - data rate = 2Mbps
        // - Checksum length = 1 byte

        nrf.set_frequency(0x32).unwrap();
        nrf.set_auto_retransmit(6, 15).unwrap();
        nrf.set_rf(&DataRate::R2Mbps, 0).unwrap();
        nrf.set_pipes_rx_enable(&[false, true, false, false, false, false])
            .unwrap();
        nrf.set_auto_ack(&[true; 6]).unwrap();
        nrf.set_crc(CrcMode::OneByte).unwrap();
        nrf.set_pipes_rx_lengths(&[None; 6], true).unwrap();
        nrf.set_rx_addr(1, &[0xe7u8, 0xe7u8, 0xe7u8, 0xe7u8, 0xe7u8] as &[u8])
            .unwrap();

        let mut rx = nrf.rx().unwrap();

        let is_empty = rx.is_empty().unwrap();
        if !is_empty {
            rprintln!("RX queue not empty. Truncating ...");
            while let Some(_) = rx.can_read().unwrap() {
                let res = rx.read().unwrap();
                rprintln!("- Got {} bytes: {:02X?}", res.len(), res.as_ref());
            }
        }

        return Radio { rx };
    }

    pub fn poll(&mut self, status: &protocol::Status) -> Option<protocol::Command> {
        self.rx.clear_interrupts().unwrap();
        while let Some(_) = self.rx.can_read().unwrap() {
            // prepare response
            let mut buf = [0u8; 32];
            let writer = SliceWrite::new(&mut buf[..]);
            let mut ser = Serializer::new(writer);
            status.serialize(&mut ser).ok();
            let writer = ser.into_inner();
            let size = writer.bytes_written();

            // read incoming packet
            let payload = self.rx.read().unwrap();
            rprintln!("- Got {} bytes: {:02X?}", payload.len(), payload.as_ref());
            self.rx.send(&buf[..size], Some(1)).unwrap();

            let mut payload_array = [0u8; 32];
            payload_array[0..payload.len()].copy_from_slice(payload.as_ref());

            let cmd: protocol::Command =
                from_mut_slice(&mut payload_array[0..payload.len()]).unwrap();
            return Some(cmd);
        }
        None
    }
}
