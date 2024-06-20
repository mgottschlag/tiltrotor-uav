use core::cell::RefCell;
use core::fmt::Write;
use defmt::{error, info};
use dummy_pin::DummyPin;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::channel::Channel;
use embassy_time::{Delay, Instant};
use embedded_sdmmc::sdcard::AcquireOpts;
use embedded_sdmmc::{Mode, SdCardError, TimeSource, Timestamp, VolumeIdx, VolumeManager};
use heapless::String;

use crate::board::{StorageCs, StorageSpi};

pub type EventChannel = Channel<CriticalSectionRawMutex, Event, 10>;
pub type ErrorString = String<256>;

#[derive(defmt::Format)]
pub enum Event {
    Error(ErrorString),
    Command(protocol::Command),
}

struct Clock;

impl TimeSource for Clock {
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

#[derive(thiserror_no_std::Error, Debug, defmt::Format)]
pub enum Error {
    #[error("interface error")]
    Interface(#[from] embedded_sdmmc::Error<SdCardError>),
}

async fn handle(
    spi: StorageSpi,
    cs: StorageCs,
    event_channel: &'static EventChannel,
) -> Result<(), Error> {
    let bus: Mutex<CriticalSectionRawMutex, RefCell<StorageSpi>> = Mutex::new(RefCell::new(spi));
    let spidevice = SpiDevice::new(&bus, cs);

    let sdcard = embedded_sdmmc::SdCard::new_with_options(
        spidevice,
        DummyPin::new_low(),
        Delay,
        AcquireOpts {
            use_crc: false,
            acquire_retries: 0,
        },
    );

    match sdcard.num_bytes() {
        Ok(size) => {
            info!("Found sdcard with size={}", size);

            let mut volume_mgr = VolumeManager::new(sdcard, Clock);
            let mut volume = volume_mgr.open_volume(VolumeIdx(0))?;

            let mut root_dir = volume.open_root_dir()?;
            let mut file_count = 0;
            root_dir.iterate_dir(|_| {
                file_count += 1;
            })?;

            let mut file_name: String<12> = String::new();
            write!(&mut file_name, "{}.txt", file_count).ok();
            let mut file = root_dir.open_file_in_dir(file_name.as_str(), Mode::ReadWriteCreate)?;
            info!("Created new trace file '{}'", file_name.as_str());

            loop {
                let event = event_channel.receive().await;
                match event {
                    /*Event::Command(cmd) => {
                        let mut res: String<64> = String::new();
                        write!(&mut res, "{};{:?}\n", Instant::now().as_millis(), cmd).ok();
                        res
                    }*/
                    Event::Error(msg) => {
                        let mut res: String<16> = String::new();
                        write!(&mut res, "{};E;", Instant::now().as_millis()).ok();
                        file.write(res.as_bytes())?;
                        file.write(msg.as_bytes())?;
                    }
                    _ => {}
                };
            }
        }
        Err(e) => {
            info!("No sdcard available: {}", e);
            info!("Printing events to stdout");
            loop {
                let event = event_channel.receive().await;
                match event {
                    Event::Error(msg) => {
                        error!("Got error for sd card: {}", msg)
                    }
                    _ => {}
                };
            }
        }
    };
}

#[embassy_executor::task]
pub async fn run(spi: StorageSpi, cs: StorageCs, event_channel: &'static EventChannel) {
    if let Err(e) = handle(spi, cs, event_channel).await {
        error!("Failed to handle trace: {}", e)
    }
}
