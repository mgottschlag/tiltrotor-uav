use core::fmt::Write;
use defmt::info;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Instant;
use embedded_sdmmc::sdmmc::AcquireOpts;
use embedded_sdmmc::{Mode, TimeSource, Timestamp, VolumeIdx};
use heapless::String;

use crate::board::{StorageCs, StorageSpi};

pub type EventChannel = Channel<CriticalSectionRawMutex, Event, 10>;

pub enum Event {
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

#[embassy_executor::task]
pub async fn run(spi: StorageSpi, cs: StorageCs, event_channel: &'static EventChannel) {
    let mut spi_dev = embedded_sdmmc::SdMmcSpi::new(spi, cs);
    let block = spi_dev
        .acquire_with_opts(AcquireOpts { require_crc: false })
        .unwrap();
    let mut controller = embedded_sdmmc::Controller::new(block, Clock);
    let size = controller.device().card_size_bytes().unwrap();
    info!("Found sd card with size={}", size);
    let mut volume = controller.get_volume(VolumeIdx(0)).unwrap();

    let root_dir = controller.open_root_dir(&volume).unwrap();
    let mut file_count = 0;
    controller
        .iterate_dir(&volume, &root_dir, |dir_entry| {
            let mut name_vec =
                heapless::Vec::<u8, 12>::from_slice(dir_entry.name.base_name()).unwrap();
            name_vec.extend_from_slice(&[b'.']).unwrap();
            name_vec
                .extend_from_slice(dir_entry.name.extension())
                .unwrap();
            let name = String::from_utf8(name_vec).unwrap();
            info!("- {:?}", name.as_str());
            file_count += 1;
        })
        .unwrap();

    let mut file_name: String<12> = String::new();
    write!(&mut file_name, "{}.txt", file_count).unwrap();
    let mut file = controller
        .open_file_in_dir(
            &mut volume,
            &root_dir,
            file_name.as_str(),
            Mode::ReadWriteCreate,
        )
        .unwrap();
    info!("Created new trace file '{}'", file_name.as_str());

    loop {
        let event = event_channel.receive().await;
        let msg = match event {
            Event::Command(cmd) => {
                let mut res: String<64> = String::new();
                write!(&mut res, "{};{:?}\n", Instant::now().as_millis(), cmd).unwrap();
                res
            }
        };
        controller
            .write(&mut volume, &mut file, msg.as_bytes())
            .unwrap();
    }
}
