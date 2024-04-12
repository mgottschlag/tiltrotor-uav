//use super::EnginePwm;
use super::Direction;

use cortex_m::prelude::_embedded_hal_blocking_delay_DelayUs;
use defmt::info;
use embassy_stm32::adc::{Adc, VrefInt};
use embassy_stm32::dma::NoDma;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Input, Level, Output, OutputType, Pull, Speed};
use embassy_stm32::i2c::I2c;
use embassy_stm32::peripherals::{
    ADC1, I2C1, PA0, PA1, PA10, PA15, PA4, PA5, PA6, PA7, PA8, PA9, PB13, PB14, PB15, PB2, PB3,
    PB4, PB5, PB8, PB9, PC13, PC14, PC15, SPI1, SPI2, SPI3, TIM5,
};
use embassy_stm32::spi::{Config as SpiConfig, Spi};
use embassy_stm32::time::hz;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::timer::Channel;
use embassy_stm32::{bind_interrupts, i2c, peripherals};
use embassy_time::Delay;

type PwmC1 = PA0;
type PwmC2 = PA1;
type RadioSck = PA5;
type RadioMiso = PA6;
type RadioMosi = PA7;
pub type RadioCs = Output<'static, PA8>;
pub type RadioCe = Output<'static, PA9>;
pub type RadioIrq = ExtiInput<'static, PA10>;
pub type StorageCs = Output<'static, PA15>;
type EngineInt1 = Output<'static, PB2>;
type StorageSck = PB3;
type StorageMiso = PB4;
type StorageMosi = PB5;
type DisplaySck = PB8;
type DisplaySda = PB9;
// type ImuSck = PB13;
// type ImuMiso = PB14;
// type ImuMosi = PB15;
type EngineInt2 = Output<'static, PC13>;
type EngineInt3 = Output<'static, PC14>;
type EngineInt4 = Output<'static, PC15>;

pub type DisplayI2c = I2c<'static, I2C1>;
pub type RadioSpi = Spi<'static, SPI1, NoDma, NoDma>;
// pub type ImuSpi = Spi<'static, SPI2, NoDma, NoDma>;
pub type StorageSpi = Spi<'static, SPI3, NoDma, NoDma>;

bind_interrupts!(struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
});

pub struct Board {
    pub display_i2c: DisplayI2c,
    pub engines: EnginePwm,
    pub radio_spi: RadioSpi,
    pub radio_cs: RadioCs,
    pub radio_ce: RadioCe,
    pub radio_irq: RadioIrq,
    pub battery_monitor: BatteryMonitor,
    pub storage_spi: StorageSpi,
    pub storage_cs: StorageCs,
}

impl Board {
    pub fn init() -> Board {
        let p = embassy_stm32::init(Default::default());

        // init display
        let display_sck: DisplaySck = p.PB8;
        let display_sda: DisplaySda = p.PB9;
        let display_i2c = I2c::new(
            p.I2C1,
            display_sck,
            display_sda,
            Irqs,
            NoDma,
            NoDma,
            hz(400_000),
            Default::default(),
        );

        // init battery adc
        let mut battery_delay = Delay;
        let battery_adc = Adc::new(p.ADC1, &mut battery_delay);
        let battery_monitor = BatteryMonitor::init(battery_adc, battery_delay, p.PA4);

        // init storage
        let mut storage_spi_config = SpiConfig::default();
        storage_spi_config.frequency = hz(250_000); // TODO: 1_000_000 or more?

        let storage_sck: StorageSck = p.PB3;
        let storage_miso: StorageMiso = p.PB4;
        let storage_mosi: StorageMosi = p.PB5;
        let storage_cs: StorageCs = Output::new(p.PA15, Level::High, Speed::VeryHigh);
        let storage_spi = Spi::new(
            p.SPI3,
            storage_sck,
            storage_mosi,
            storage_miso,
            NoDma,
            NoDma,
            storage_spi_config,
        );

        // init pwm
        let pwm_c1: PwmC1 = p.PA0;
        let pwm_c2: PwmC2 = p.PA1;
        let c1 = PwmPin::new_ch1(pwm_c1, OutputType::PushPull);
        let c2 = PwmPin::new_ch2(pwm_c2, OutputType::PushPull);
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
            Output::new(p.PB2, Level::Low, Speed::Medium),
            Output::new(p.PC13, Level::Low, Speed::Medium),
            Output::new(p.PC14, Level::Low, Speed::Medium),
            Output::new(p.PC15, Level::Low, Speed::Medium),
        );

        // init radio
        let mut radio_spi_config = SpiConfig::default();
        radio_spi_config.frequency = hz(2_000_000);
        let radio_sck: RadioSck = p.PA5;
        let radio_miso: RadioMiso = p.PA6;
        let radio_mosi: RadioMosi = p.PA7;
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
            display_i2c,
            engines,
            radio_spi,
            radio_cs,
            radio_ce,
            radio_irq,
            battery_monitor,
            storage_spi,
            storage_cs,
        }
    }
}

//pub type EnginePwmType = BlackpillEnginePwm;

const MINIMAL_DUTY: u16 = 150;

pub struct EnginePwm {
    pwm: SimplePwm<'static, TIM5>,
    int1: EngineInt1,
    int2: EngineInt2,
    int3: EngineInt3,
    int4: EngineInt4,
}

impl EnginePwm {
    pub fn init(
        mut pwm: SimplePwm<'static, TIM5>,
        int1: EngineInt1,
        int2: EngineInt2,
        int3: EngineInt3,
        int4: EngineInt4,
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

pub struct BatteryMonitor {
    adc: Adc<'static, ADC1>,
    pin: PA4,
}

impl BatteryMonitor {
    pub fn init(adc: Adc<'static, ADC1>, mut delay: Delay, pin: PA4) -> Self {
        adc.enable_vrefint();
        delay.delay_us(VrefInt::start_time_us());
        Self { adc, pin }
    }

    pub fn read(&mut self) -> f32 {
        let v = self.adc.read(&mut self.pin) as f32;
        v * 996.0 / 274.4 / 1000.0
    }
}
