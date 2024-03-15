//use super::EnginePwm;

use embassy_stm32::dma::NoDma;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Input, Level, Output, OutputType, Pull, Speed};
use embassy_stm32::peripherals::{PA10, PA5, PA6, PA7, PA8, PA9, SPI1, TIM5};
use embassy_stm32::spi::{Config as SpiConfig, Spi};
use embassy_stm32::time::hz;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::timer::Channel;

pub type RadioSck = PA5;
pub type RadioMiso = PA6;
pub type RadioMosi = PA7;
pub type RadioCs = Output<'static, PA8>;
pub type RadioCe = Output<'static, PA9>;
pub type RadioIrq = ExtiInput<'static, PA10>;
pub type RadioSpi = Spi<'static, SPI1, NoDma, NoDma>;

pub struct Board {
    pub engines: EnginePwm,
    pub radio_spi: RadioSpi,
    pub radio_cs: RadioCs,
    pub radio_ce: RadioCe,
    pub radio_irq: RadioIrq,
}

impl Board {
    pub fn init() -> Board {
        let p = embassy_stm32::init(Default::default());

        // pwm
        let c1 = PwmPin::new_ch1(p.PA0, OutputType::PushPull);
        let c2 = PwmPin::new_ch2(p.PA1, OutputType::PushPull);
        let c3 = PwmPin::new_ch3(p.PA2, OutputType::PushPull);
        let c4 = PwmPin::new_ch4(p.PA3, OutputType::PushPull);
        let pwm = SimplePwm::new(
            p.TIM5,
            Some(c1),
            Some(c2),
            Some(c3),
            Some(c4),
            hz(50),
            Default::default(),
        );
        let engines = EnginePwm::init(pwm);

        // init radio
        let mut radio_spi_config = SpiConfig::default();
        radio_spi_config.frequency = hz(2_000_000);
        let radio_sck = p.PA5;
        let radio_miso = p.PA6;
        let radio_mosi = p.PA7;
        let radio_cs = Output::new(p.PA8, Level::High, Speed::VeryHigh);
        let radio_ce = Output::new(p.PA9, Level::High, Speed::VeryHigh);
        let radio_irq = ExtiInput::new(Input::new(p.PA10, Pull::Up), p.EXTI10);

        let radio_spi = Spi::new(
            p.SPI1,
            radio_sck,
            radio_mosi,
            radio_miso,
            NoDma,
            NoDma,
            radio_spi_config,
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

//pub type EnginePwmType = BlackpillEnginePwm;

pub struct EnginePwm {
    pwm: SimplePwm<'static, TIM5>,
}

impl EnginePwm {
    pub fn init(mut pwm: SimplePwm<'static, TIM5>) -> Self {
        pwm.set_duty(Channel::Ch1, 0);
        pwm.set_duty(Channel::Ch2, 0);
        pwm.set_duty(Channel::Ch3, 0);
        pwm.set_duty(Channel::Ch4, 0);
        pwm.enable(Channel::Ch1);
        pwm.enable(Channel::Ch2);
        pwm.enable(Channel::Ch3);
        pwm.enable(Channel::Ch4);
        EnginePwm { pwm }
    }

    fn get_max_duty(&self) -> u16 {
        self.pwm.get_max_duty()
    }
    fn set_duty(&mut self, duty: [u16; 4]) {
        self.pwm.set_duty(Channel::Ch1, duty[0]);
        self.pwm.set_duty(Channel::Ch2, duty[1]);
        self.pwm.set_duty(Channel::Ch3, duty[2]);
        self.pwm.set_duty(Channel::Ch4, duty[3]);
    }
}
