//use super::EnginePwm;
use super::Direction;

use defmt::info;
use embassy_stm32::dma::NoDma;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Input, Level, Output, OutputType, Pull, Speed};
use embassy_stm32::peripherals::{PA10, PA2, PA3, PA8, PA9, PC13, PC14, SPI1, TIM5};
use embassy_stm32::spi::{Config as SpiConfig, Spi};
use embassy_stm32::time::hz;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::timer::Channel;

// pub type RadioSck = PA5;
// pub type RadioMiso = PA6;
// pub type RadioMosi = PA7;
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
        let pwm = SimplePwm::new(
            p.TIM5,
            Some(c1),
            Some(c2),
            None,
            None,
            hz(50_000),
            Default::default(),
        );
        let engines = EnginePwm::init(
            pwm,
            Output::new(p.PA2, Level::Low, Speed::Medium),
            Output::new(p.PA3, Level::Low, Speed::Medium),
            Output::new(p.PC13, Level::Low, Speed::Medium),
            Output::new(p.PC14, Level::Low, Speed::Medium),
        );

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

const MINIMAL_DUTY: u16 = 150;

pub struct EnginePwm {
    pwm: SimplePwm<'static, TIM5>,
    int1: Output<'static, PA2>,
    int2: Output<'static, PA3>,
    int3: Output<'static, PC13>,
    int4: Output<'static, PC14>,
}

impl EnginePwm {
    pub fn init(
        mut pwm: SimplePwm<'static, TIM5>,
        int1: Output<'static, PA2>,
        int2: Output<'static, PA3>,
        int3: Output<'static, PC13>,
        int4: Output<'static, PC14>,
    ) -> Self {
        pwm.set_duty(Channel::Ch1, 0);
        pwm.set_duty(Channel::Ch2, 0);
        pwm.enable(Channel::Ch1);
        pwm.enable(Channel::Ch2);
        EnginePwm {
            pwm,
            int1,
            int2,
            int3,
            int4,
        }
    }

    fn get_max_duty(&self) -> u16 {
        self.pwm.get_max_duty()
    }

    fn scale_duty(&self, duty: f32) -> u16 {
        if duty == 0.0 {
            return 0;
        } else {
            return ((self.get_max_duty() - MINIMAL_DUTY) as f32 * duty) as u16 + MINIMAL_DUTY;
        }
    }

    fn set_duty(&mut self, duty: [f32; 2]) {
        let scaled_duty = duty.map(|d| self.scale_duty(d));
        info!("duty={} => scaled_duty={}", duty, scaled_duty);
        self.pwm.set_duty(Channel::Ch1, scaled_duty[0]);
        self.pwm.set_duty(Channel::Ch2, scaled_duty[1]);
    }

    pub fn update(&mut self, motor_left: Direction, motor_right: Direction) {
        info!("motor_left={:?}, motor_right={:?}", motor_left, motor_right);

        let mut duty_left = 0.0;
        let mut duty_right = 0.0;

        match motor_left {
            Direction::Forward(duty) => {
                self.int1.set_high();
                self.int2.set_low();
                duty_left = duty;
            }
            Direction::Backward(duty) => {
                self.int1.set_low();
                self.int2.set_high();
                duty_left = duty;
            }
            Direction::Stop => {
                self.int1.set_low();
                self.int2.set_low();
            }
        }
        match motor_right {
            Direction::Forward(duty) => {
                self.int3.set_low();
                self.int4.set_high();
                duty_right = duty;
            }
            Direction::Backward(duty) => {
                self.int3.set_high();
                self.int4.set_low();
                duty_right = duty;
            }
            Direction::Stop => {
                self.int3.set_low();
                self.int4.set_low();
            }
        }

        self.set_duty([duty_left, duty_right]);
    }
}
