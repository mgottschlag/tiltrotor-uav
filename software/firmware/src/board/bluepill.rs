use embedded_hal::PwmPin;
use pac::{SPI2, TIM2};
use stm32f1xx_hal::gpio::gpioa::{PA8, PA9};
use stm32f1xx_hal::gpio::gpiob::{PB12, PB13, PB14, PB15};
use stm32f1xx_hal::gpio::{Alternate, Floating, IOPinSpeed, Input, Output, OutputSpeed, PushPull};
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::pwm::{PwmChannel, C1, C2, C3, C4};
use stm32f1xx_hal::spi::{self, Phase, Polarity, Spi, Spi2NoRemap};
pub use stm32f1xx_hal::stm32 as pac;
use stm32f1xx_hal::time::U32Ext;
use stm32f1xx_hal::timer::{Tim2NoRemap, Timer};

use super::EnginePwm;

pub type RadioSck = PB13<Alternate<PushPull>>;
pub type RadioMiso = PB14<Input<Floating>>;
pub type RadioMosi = PB15<Alternate<PushPull>>;
pub type RadioCs = PA8<Output<PushPull>>;
pub type RadioCe = PA9<Output<PushPull>>;
pub type RadioIrq = PB12<Output<PushPull>>;
pub type RadioSpi = Spi<SPI2, Spi2NoRemap, (RadioSck, RadioMiso, RadioMosi), u8>;

pub struct Board {
    pub engines: BluepillEnginePwm,
    pub radio_spi: RadioSpi,
    pub radio_cs: RadioCs,
    pub radio_ce: RadioCe,
    pub radio_irq: RadioIrq,
}

impl Board {
    pub fn init(_core: rtic::Peripherals, device: pac::Peripherals) -> Board {
        let mut flash = device.FLASH.constrain();
        let rcc = device.RCC.constrain();
        let mut afio = device.AFIO.constrain();

        let clocks = rcc
            .cfgr
            .use_hse(8.mhz())
            .sysclk(48.mhz())
            .pclk1(24.mhz())
            .freeze(&mut flash.acr);

        let mut gpioa = device.GPIOA.split();
        let mut gpiob = device.GPIOB.split();

        // Engine PWM:
        let c1 = gpioa.pa0.into_alternate_push_pull(&mut gpioa.crl);
        let c2 = gpioa.pa1.into_alternate_push_pull(&mut gpioa.crl);
        let c3 = gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl);
        let c4 = gpioa.pa3.into_alternate_push_pull(&mut gpioa.crl);
        let pins = (c1, c2, c3, c4);
        let pwm = Timer::tim2(device.TIM2, &clocks).pwm::<Tim2NoRemap, _, _, _>(
            pins,
            &mut afio.mapr,
            50.hz(),
        );

        let engines = BluepillEnginePwm::init(pwm.split());

        // radio
        let mut radio_cs = gpioa.pa8.into_push_pull_output(&mut gpioa.crh);
        let mut radio_ce = gpioa.pa9.into_push_pull_output(&mut gpioa.crh);
        let mut radio_irq = gpiob.pb12.into_push_pull_output(&mut gpiob.crh);
        let mut radio_sck = gpiob.pb13.into_alternate_push_pull(&mut gpiob.crh);
        let radio_miso = gpiob.pb14.into_floating_input(&mut gpiob.crh);
        let mut radio_mosi = gpiob.pb15.into_alternate_push_pull(&mut gpiob.crh);

        radio_cs.set_speed(&mut gpioa.crh, IOPinSpeed::Mhz50);
        radio_ce.set_speed(&mut gpioa.crh, IOPinSpeed::Mhz50);
        radio_irq.set_speed(&mut gpiob.crh, IOPinSpeed::Mhz50);
        radio_sck.set_speed(&mut gpiob.crh, IOPinSpeed::Mhz50);
        radio_mosi.set_speed(&mut gpiob.crh, IOPinSpeed::Mhz50);

        let radio_spi = Spi::spi2(
            device.SPI2,
            (radio_sck, radio_miso, radio_mosi),
            spi::Mode {
                polarity: Polarity::IdleLow,
                phase: Phase::CaptureOnFirstTransition,
            },
            2.mhz(),
            clocks,
        );

        Board {
            engines,
            radio_spi,
            radio_cs,
            radio_ce,
            radio_irq,
        }
    }
}

pub type EnginePwmType = BluepillEnginePwm;

pub struct BluepillEnginePwm {
    c: (
        PwmChannel<TIM2, C1>,
        PwmChannel<TIM2, C2>,
        PwmChannel<TIM2, C3>,
        PwmChannel<TIM2, C4>,
    ),
}

impl BluepillEnginePwm {
    pub fn init(
        mut c: (
            PwmChannel<TIM2, C1>,
            PwmChannel<TIM2, C2>,
            PwmChannel<TIM2, C3>,
            PwmChannel<TIM2, C4>,
        ),
    ) -> Self {
        c.0.set_duty(0);
        c.1.set_duty(0);
        c.2.set_duty(0);
        c.3.set_duty(0);
        c.0.enable();
        c.1.enable();
        c.2.enable();
        c.3.enable();
        BluepillEnginePwm { c }
    }
}

impl EnginePwm for BluepillEnginePwm {
    fn get_max_duty(&self) -> u16 {
        // We assume that all channels have the same maximum duty.
        self.c.0.get_max_duty()
    }
    fn set_duty(&mut self, duty: [u16; 4]) {
        self.c.0.set_duty(duty[0]);
        self.c.1.set_duty(duty[1]);
        self.c.2.set_duty(duty[2]);
        self.c.3.set_duty(duty[3]);
    }
}
