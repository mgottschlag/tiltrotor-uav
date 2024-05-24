use core::fmt::Write;
use defmt::error;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X12, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text},
};
use heapless::String;
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};

use crate::board::DisplayI2c;

pub type EventChannel = Channel<CriticalSectionRawMutex, Event, 10>;

static TEXT_STYLE: MonoTextStyle<'static, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_6X12)
    .text_color(BinaryColor::On)
    .build();
static CLEAR_STYLE: PrimitiveStyle<BinaryColor> = PrimitiveStyleBuilder::new()
    .stroke_width(10)
    .stroke_color(BinaryColor::Off)
    .build();

pub enum Event {
    Command(protocol::Command),
    Battery(f32),
}

#[derive(thiserror_no_std::Error, Debug, defmt::Format)]
pub enum Error {
    #[error("interface error")]
    Interface(#[from] display_interface::DisplayError),
}

pub struct Display {
    display: Ssd1306<
        I2CInterface<DisplayI2c>,
        DisplaySize128x64,
        BufferedGraphicsMode<DisplaySize128x64>,
    >,
}

impl Display {
    pub fn init(i2c: DisplayI2c) -> Result<Self, Error> {
        let interface = I2CDisplayInterface::new(i2c);
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        display.init()?;

        Ok(Display { display })
    }

    pub fn handle(&mut self, event: Event) -> Result<(), Error> {
        let mut msg: String<16> = String::new();
        let y = match event {
            Event::Command(cmd) => {
                write!(
                    &mut msg,
                    "Pose: [{}, {}]",
                    round(cmd.pose[0]),
                    round(cmd.pose[1])
                )
                .unwrap();
                0
            }
            Event::Battery(voltage) => {
                write!(&mut msg, "Bat:  {} V", round(voltage)).unwrap();
                20
            }
        };

        Rectangle::new(Point::new(0, y), Size::new(128, 10))
            .into_styled(CLEAR_STYLE)
            .draw(&mut self.display)?;
        Text::with_baseline(msg.as_str(), Point::new(0, y), TEXT_STYLE, Baseline::Top)
            .draw(&mut self.display)?;
        self.display.flush()?;

        Ok(())
    }
}

#[embassy_executor::task]
pub async fn run(mut display: Display, event_channel: &'static EventChannel) {
    loop {
        let event = event_channel.receive().await;
        if let Err(err) = display.handle(event) {
            error!("Failed to display event: {:?}", err)
        }
    }
}

fn round(v: f32) -> f32 {
    libm::roundf(v * 100.0) / 100.0
}
