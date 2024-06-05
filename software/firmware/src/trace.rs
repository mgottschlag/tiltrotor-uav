use core::cell::RefCell;
use core::fmt::Write;
use defmt::info;
use dummy_pin::DummyPin;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::channel::Channel;
use embassy_time::{Delay, Instant};
use embedded_sdmmc::sdcard::AcquireOpts;
use embedded_sdmmc::{Mode, TimeSource, Timestamp, VolumeIdx, VolumeManager};
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
    let size = sdcard.num_bytes().unwrap();
    info!("Found sd card with size={}", size);

    let mut volume_mgr = VolumeManager::new(sdcard, Clock);
    let mut volume = volume_mgr.open_volume(VolumeIdx(0)).unwrap();

    let mut root_dir = volume.open_root_dir().unwrap();
    let mut file_count = 0;
    root_dir
        .iterate_dir(|dir_entry| {
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
    let mut file = root_dir
        .open_file_in_dir(file_name.as_str(), Mode::ReadWriteCreate)
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
        file.write(msg.as_bytes()).unwrap();
    }
}
