use stm32f4xx_hal::gpio::gpioa::{PA10, PA5, PA6, PA7, PA8, PA9};
use stm32f4xx_hal::gpio::{Alternate, Output, PushPull};
pub use stm32f4xx_hal::pac;
use stm32f4xx_hal::pac::SPI1;
use stm32f4xx_hal::prelude::*;
use stm32f4xx_hal::pwm::{PwmChannel, C1, C2, C3, C4};
use stm32f4xx_hal::spi::{Mode, Phase, Polarity, Spi, TransferModeNormal};
use stm32f4xx_hal::timer::Timer;

use super::EnginePwm;

pub type RadioSck = PA5<Alternate<PushPull, 5>>;
pub type RadioMiso = PA6<Alternate<PushPull, 5>>;
pub type RadioMosi = PA7<Alternate<PushPull, 5>>;
pub type RadioCs = PA8<Output<PushPull>>;
pub type RadioCe = PA9<Output<PushPull>>;
pub type RadioIrq = PA10<Output<PushPull>>;
pub type RadioSpi = Spi<SPI1, (RadioSck, RadioMiso, RadioMosi), TransferModeNormal>;

pub struct Board {
    pub engines: BlackpillEnginePwm,
    pub radio_spi: RadioSpi,
    pub radio_cs: RadioCs,
    pub radio_ce: RadioCe,
    pub radio_irq: RadioIrq,
}

impl Board {
    pub fn init(_core: rtic::Peripherals, device: pac::Peripherals) -> Board {
        let rcc = device.RCC.constrain();

        let clocks = rcc
            .cfgr
            .use_hse(25.mhz())
            .sysclk(48.mhz())
            .require_pll48clk()
            .freeze();

        let gpioa = device.GPIOA.split();

        // init pwm
        let c1 = gpioa.pa0.into_alternate();
        let c2 = gpioa.pa1.into_alternate();
        let c3 = gpioa.pa2.into_alternate();
        let c4 = gpioa.pa3.into_alternate();
        let pins = (c1, c2, c3, c4);
        //let pwm = pwm::tim5(device.TIM5, pins, clocks, 50.hz());
        let pwm = Timer::new(device.TIM5, &clocks).pwm(pins, 50.hz());
        let engines = BlackpillEnginePwm::init(pwm);

        // init radio
        let radio_sck = gpioa.pa5.into_alternate();
        let radio_miso = gpioa.pa6.into_alternate();
        let radio_mosi = gpioa.pa7.into_alternate();
        let radio_cs = gpioa.pa8.into_push_pull_output();
        let radio_ce = gpioa.pa9.into_push_pull_output();
        let radio_irq = gpioa.pa10.into_push_pull_output();
        let radio_spi = Spi::new(
            device.SPI1,
            (radio_sck, radio_miso, radio_mosi),
            Mode {
                polarity: Polarity::IdleLow,
                phase: Phase::CaptureOnFirstTransition,
            },
            2_000_000.hz(),
            &clocks,
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

pub type EnginePwmType = BlackpillEnginePwm;

pub struct BlackpillEnginePwm {
    c: (
        PwmChannel<pac::TIM5, C1>,
        PwmChannel<pac::TIM5, C2>,
        PwmChannel<pac::TIM5, C3>,
        PwmChannel<pac::TIM5, C4>,
    ),
}

impl BlackpillEnginePwm {
    pub fn init(
        mut c: (
            PwmChannel<pac::TIM5, C1>,
            PwmChannel<pac::TIM5, C2>,
            PwmChannel<pac::TIM5, C3>,
            PwmChannel<pac::TIM5, C4>,
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
        BlackpillEnginePwm { c }
    }
}

impl EnginePwm for BlackpillEnginePwm {
    fn get_max_duty(&self) -> u16 {
        self.c.0.get_max_duty()
    }
    fn set_duty(&mut self, duty: [u16; 4]) {
        self.c.0.set_duty(duty[0]);
        self.c.1.set_duty(duty[1]);
        self.c.2.set_duty(duty[2]);
        self.c.3.set_duty(duty[3]);
    }
}
