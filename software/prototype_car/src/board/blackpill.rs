use super::{Direction, EnginePwm};

use cortex_m::prelude::_embedded_hal_blocking_delay_DelayUs;
use defmt::info;
use embassy_stm32::adc::Adc;
use embassy_stm32::adc::VrefInt;
use embassy_stm32::gpio::{Level, Output, OutputType, Speed};
use embassy_stm32::i2c::I2c;
use embassy_stm32::mode::Async;
use embassy_stm32::peripherals::{ADC1, PA0, PA1, PA2, PA3, PA4, PB3, PB4, PB5, PB8, PB9, TIM5};
use embassy_stm32::spi::Config as SpiConfig;
use embassy_stm32::spi::Spi;
use embassy_stm32::time::hz;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::usart;
use embassy_stm32::usart::Config;
use embassy_stm32::usart::Parity;
use embassy_stm32::usart::StopBits;
use embassy_stm32::usart::Uart;
use embassy_stm32::{bind_interrupts, i2c, peripherals};
use embassy_time::Delay;
use libm::fabs;
use motor::Command;

type PwmC1 = PA0;
type PwmC2 = PA1;
type RadioRx = PA3;
type RadioTx = PA2;
type StorageSck = PB3;
type StorageMiso = PB4;
type StorageMosi = PB5;
type DisplaySck = PB8;
type DisplaySda = PB9;
// type ImuSck = PB13;
// type ImuMiso = PB14;
// type ImuMosi = PB15;
type EngineInt1 = Output<'static>; // PB2
type EngineInt2 = Output<'static>; // PC13
type EngineInt3 = Output<'static>; // PC14
type EngineInt4 = Output<'static>; // PC15

pub type DisplayI2c = I2c<'static, Async>;
pub type RadioUart = Uart<'static, Async>; // USART2
pub type StorageCs = Output<'static>;
pub type StorageSpi = Spi<'static, Async>; // SPI3

bind_interrupts!(struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
    USART2 => usart::InterruptHandler<peripherals::USART2>;
});

pub struct Board<M: motor::Type> {
    pub display_i2c: DisplayI2c,
    pub engines: BlackpillEnginePwm<M>,
    pub radio_uart: RadioUart,
    pub battery_monitor: BatteryMonitor,
    pub storage_spi: StorageSpi,
    pub storage_cs: StorageCs,
}

impl<M: motor::Type> Board<M> {
    pub fn init(motor_driver: M) -> Board<M> {
        let p = embassy_stm32::init(Default::default());

        // init display
        let display_sck: DisplaySck = p.PB8;
        let display_sda: DisplaySda = p.PB9;
        let display_i2c = I2c::new(
            p.I2C1,
            display_sck,
            display_sda,
            Irqs,
            p.DMA1_CH1,
            p.DMA1_CH0,
            hz(400_000),
            Default::default(),
        );

        // init battery adc
        let battery_delay = Delay;
        let battery_adc = Adc::new(p.ADC1);
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
            p.DMA1_CH7,
            p.DMA1_CH2,
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
        let engines = BlackpillEnginePwm::init(
            motor_driver,
            pwm,
            Output::new(p.PB2, Level::Low, Speed::Medium),
            Output::new(p.PC13, Level::Low, Speed::Medium),
            Output::new(p.PC14, Level::Low, Speed::Medium),
            Output::new(p.PC15, Level::Low, Speed::Medium),
        );

        // init radio
        let mut radio_uart_config = Config::default();
        radio_uart_config.baudrate = 100000;
        radio_uart_config.parity = Parity::ParityEven;
        radio_uart_config.stop_bits = StopBits::STOP2;
        let radio_rx: RadioRx = p.PA3;
        let radio_tx: RadioTx = p.PA2;
        let radio_uart = Uart::new(
            p.USART2,
            radio_rx,
            radio_tx,
            Irqs,
            p.DMA1_CH6,
            p.DMA1_CH5,
            radio_uart_config,
        )
        .unwrap();

        Board {
            display_i2c,
            engines,
            radio_uart,
            battery_monitor,
            storage_spi,
            storage_cs,
        }
    }
}

pub type EnginePwmType<M> = BlackpillEnginePwm<M>;

const MINIMAL_DUTY: u16 = 150;

pub struct BlackpillEnginePwm<M>
where
    M: motor::Type,
{
    motor_driver: M,
    pwm: SimplePwm<'static, TIM5>,
    int1: EngineInt1,
    int2: EngineInt2,
    int3: EngineInt3,
    int4: EngineInt4,
    last: [Direction; 4],
}

impl<M> BlackpillEnginePwm<M>
where
    M: motor::Type,
{
    pub fn init(
        motor_driver: M,
        mut pwm: SimplePwm<'static, TIM5>,
        int1: EngineInt1,
        int2: EngineInt2,
        int3: EngineInt3,
        int4: EngineInt4,
    ) -> Self {
        pwm.ch1().set_duty_cycle(0);
        pwm.ch2().set_duty_cycle(0);
        pwm.ch1().enable();
        pwm.ch2().enable();
        BlackpillEnginePwm {
            motor_driver,
            pwm,
            int1,
            int2,
            int3,
            int4,
            last: [Direction::Stop; 4],
        }
    }

    fn get_max_duty(&self) -> u16 {
        self.pwm.max_duty_cycle()
    }

    fn scale_duty(&self, duty: f32) -> u16 {
        if duty == 0.0 {
            return 0;
        } else {
            return ((self.get_max_duty() - MINIMAL_DUTY) as f32 * duty) as u16 + MINIMAL_DUTY;
        }
    }

    fn set_duty(&mut self, duty: [f32; 2]) {
        let scaled_duty = duty.map(|d| {
            self.scale_duty(fabs(d as f64) as f32)
                .clamp(0, self.get_max_duty())
        });
        info!(
            "duty={} => scaled_duty={} (max={})",
            duty,
            scaled_duty,
            self.pwm.max_duty_cycle()
        );
        self.pwm.ch1().set_duty_cycle(scaled_duty[0]);
        self.pwm.ch2().set_duty_cycle(scaled_duty[1]);
    }
}

impl<M: motor::Type> EnginePwm for BlackpillEnginePwm<M> {
    fn update(&mut self, cmd: &Command) {
        let directions = self.motor_driver.update(cmd);
        match directions == self.last {
            true => return,
            false => self.last = directions,
        }
        info!(
            "motor_left={:?}, motor_right={:?}",
            directions[0], directions[1]
        );

        let mut duty_left = 0.0;
        let mut duty_right = 0.0;

        match directions[0] {
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
        match directions[1] {
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
        let v = self.adc.blocking_read(&mut self.pin) as f32;
        v * 996.0 / 274.4 / 1000.0
    }
}
