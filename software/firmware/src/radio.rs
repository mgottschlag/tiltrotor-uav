#[cfg(not(feature = "flightcontroller"))]
use core::convert::Infallible;
use defmt::info;
use embedded_nrf24l01::{self, Configuration, CrcMode, DataRate, RxMode, NRF24L01};
use minicbor::encode::write::EndOfSlice;

pub use crate::board::{RadioCe, RadioCs, RadioIrq, RadioSpi};

#[derive(thiserror_no_std::Error, Debug)]
pub enum Error {
    #[error("interface error")]
    Interface(#[from] embedded_nrf24l01::Error<embassy_stm32::spi::Error>),

    #[error("decode error")]
    Decode(#[from] minicbor::decode::Error),

    #[error("encode error")]
    Encode(#[from] minicbor::encode::Error<EndOfSlice>),
}

pub struct Radio {
    #[cfg(feature = "flightcontroller")]
    rx: RxMode<NRF24L01<(), RadioCe, RadioCs, RadioSpi>>,
    #[cfg(not(feature = "flightcontroller"))]
    rx: RxMode<NRF24L01<Infallible, RadioCe, RadioCs, RadioSpi>>,
}

impl Radio {
    pub fn init(spi: RadioSpi, cs: RadioCs, ce: RadioCe) -> Result<Self, Error> {
        let mut nrf = NRF24L01::new(ce, cs, spi)?;

        // configure radio
        // - channel/frequency = 0x32 (50)
        // - data rate = 2Mbps
        // - Checksum length = 1 byte

        nrf.set_frequency(0x32)?;
        nrf.set_auto_retransmit(6, 15)?;
        nrf.set_rf(&DataRate::R2Mbps, 0)?;
        nrf.set_pipes_rx_enable(&[false, true, false, false, false, false])?;
        nrf.set_auto_ack(&[true; 6])?;
        nrf.set_crc(CrcMode::OneByte)?;
        nrf.set_pipes_rx_lengths(&[None; 6], true)?;
        nrf.set_rx_addr(1, &[0x44u8, 0x72u8, 0x6fu8, 0x6eu8, 0x65u8] as &[u8])?;

        let mut rx = match nrf.rx() {
            Ok(rx) => rx,
            Err((_, err)) => return Err(Error::Interface(err)),
        };

        // clear message queue to force radio to disable interrupt
        let is_empty = rx.is_empty()?;
        if !is_empty {
            info!("RX queue not empty. Truncating ...");
            while let Some(_) = rx.can_read()? {
                let res = rx.read()?;
                info!("- Got {} bytes: 0x{:02x}", res.len(), res.as_ref());
            }
            info!("RX queue truncated");
        }

        Ok(Self { rx })
    }

    pub fn poll(&mut self, status: &protocol::Status) -> Result<Option<protocol::Command>, Error> {
        self.rx.clear_interrupts()?;
        while let Some(_) = self.rx.can_read()? {
            // prepare response
            let size = minicbor::len(&status); // TODO: handle size >= 32 bytes
            let mut buf = [0u8; 32];
            minicbor::encode(&status, buf.as_mut())?;

            // read incoming packet and send response
            let payload = self.rx.read()?;
            //info!("- Got {} bytes: {:02x}", payload.len(), payload.as_ref());
            self.rx.send(&buf[..size], Some(1))?;

            let cmd: protocol::Command = minicbor::decode(payload.as_ref())?;
            return Ok(Some(cmd));
        }
        Ok(None)
    }
}
