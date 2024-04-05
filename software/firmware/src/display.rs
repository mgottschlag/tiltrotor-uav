use core::fmt::Write;
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
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

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
}

#[embassy_executor::task]
pub async fn handle(i2c: DisplayI2c, event_channel: &'static EventChannel) {
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();

    loop {
        let event = event_channel.receive().await;
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
        };

        Rectangle::new(Point::new(0, y), Size::new(128, 10))
            .into_styled(CLEAR_STYLE)
            .draw(&mut display)
            .unwrap();
        Text::with_baseline(msg.as_str(), Point::new(0, y), TEXT_STYLE, Baseline::Top)
            .draw(&mut display)
            .unwrap();
        display.flush().unwrap();
    }
}

fn round(v: f32) -> f32 {
    libm::roundf(v * 100.0) / 100.0
}
