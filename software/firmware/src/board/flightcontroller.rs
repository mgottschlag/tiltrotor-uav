use core::marker::PhantomData;

use super::Direction;
use super::EnginePwm;

use defmt::info;
use embassy_stm32::gpio::Level;
use embassy_stm32::gpio::Output;
use embassy_stm32::gpio::Speed;
use embassy_stm32::mode::Async;
use embassy_stm32::peripherals::{PA0, PA1, PA10, PA5, PA6, PA7, PA9, TIM5};
use embassy_stm32::spi::Config as SpiConfig;
use embassy_stm32::spi::Spi;
use embassy_stm32::time::hz;
use embassy_stm32::timer::simple_pwm::SimplePwm;
use embassy_stm32::usart;
use embassy_stm32::usart::Config;
use embassy_stm32::usart::HalfDuplexReadback;
use embassy_stm32::usart::Parity;
use embassy_stm32::usart::StopBits;
use embassy_stm32::usart::Uart;
use embassy_stm32::Peri;
use embassy_stm32::{bind_interrupts, i2c, peripherals};
use libm::fabs;
use motor::Command;

// see https://github.com/betaflight/unified-targets/blob/master/configs/default/OPEN-REVO.config for pin map
type PwmC1 = PA0;
type PwmC2 = PA1;
type RadioRx = Peri<'static, PA10>;
type RadioTx = Peri<'static, PA9>;
type ImuSck = Peri<'static, PA5>;
type ImuMiso = Peri<'static, PA6>;
type ImuMosi = Peri<'static, PA7>;
type EngineInt1 = Output<'static>; // PB2
type EngineInt2 = Output<'static>; // PC13
type EngineInt3 = Output<'static>; // PC14
type EngineInt4 = Output<'static>; // PC15

pub type ImuSpi = Spi<'static, Async>; // SPI1
pub type ImuCs = Output<'static>; // PA4
pub type RadioUart = Uart<'static, Async>; // USART1

bind_interrupts!(struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
    USART1 => usart::InterruptHandler<peripherals::USART1>;
});

pub struct Board<M: motor::Type> {
    phantom: PhantomData<M>,
    pub radio_uart: RadioUart,
    pub imu_spi: ImuSpi,
    pub imu_cs: ImuCs,
}

impl<M: motor::Type> Board<M> {
    pub fn init(_motor_driver: M) -> Board<M> {
        let p = embassy_stm32::init(Default::default());

        // init radio
        let mut radio_uart_config = Config::default();
        radio_uart_config.baudrate = 100000;
        radio_uart_config.parity = Parity::ParityEven;
        radio_uart_config.stop_bits = StopBits::STOP2;
        let _radio_rx: RadioRx = p.PA10;
        let radio_tx: RadioTx = p.PA9;
        let radio_uart = Uart::new_half_duplex(
            p.USART1,
            radio_tx,
            Irqs,
            p.DMA2_CH7,
            p.DMA2_CH2,
            radio_uart_config,
            HalfDuplexReadback::Readback,
        )
        .unwrap();

        // init imu
        let imu_cs = Output::new(p.PA4, Level::High, Speed::VeryHigh);
        let imu_sck: ImuSck = p.PA5;
        let imu_miso: ImuMiso = p.PA6;
        let imu_mosi: ImuMosi = p.PA7;
        let mut imu_spi_config = SpiConfig::default();
        imu_spi_config.frequency = hz(1_000_000);
        let imu_spi = Spi::new(
            p.SPI1,
            imu_sck,
            imu_mosi,
            imu_miso,
            p.DMA2_CH3,
            p.DMA2_CH0,
            imu_spi_config,
        );

        Board {
            phantom: PhantomData,
            radio_uart,
            imu_spi,
            imu_cs,
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
