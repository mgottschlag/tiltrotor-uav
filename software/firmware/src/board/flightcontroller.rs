use super::EnginePwm;

use defmt::info;
use embassy_stm32::Peri;
use embassy_stm32::gpio::Level;
use embassy_stm32::gpio::Output;
use embassy_stm32::gpio::Speed;
use embassy_stm32::mode::Async;
use embassy_stm32::peripherals::USB_OTG_FS;
use embassy_stm32::peripherals::{PA0, PA1, PA5, PA6, PA7, PA9, PA10, TIM5};
use embassy_stm32::spi::Config as SpiConfig;
use embassy_stm32::spi::Spi;
use embassy_stm32::time::Hertz;
use embassy_stm32::time::hz;
use embassy_stm32::timer::simple_pwm::SimplePwm;
use embassy_stm32::usart;
use embassy_stm32::usart::Config as UsartConfig;
use embassy_stm32::usart::HalfDuplexReadback;
use embassy_stm32::usart::Parity;
use embassy_stm32::usart::StopBits;
use embassy_stm32::usart::Uart;
use embassy_stm32::usb;
use embassy_stm32::{bind_interrupts, i2c, peripherals};
use embassy_usb::Builder;
use embassy_usb::class::cdc_acm::CdcAcmClass;
use embassy_usb::class::cdc_acm::State;
use libm::fabs;
use motor::Command;
use static_cell::StaticCell;

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
pub type UsbClass = CdcAcmClass<'static, usb::Driver<'static, USB_OTG_FS>>;
pub type UsbDevice = embassy_usb::UsbDevice<'static, usb::Driver<'static, USB_OTG_FS>>;

bind_interrupts!(struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
    OTG_FS => usb::InterruptHandler<peripherals::USB_OTG_FS>;
    USART1 => usart::InterruptHandler<peripherals::USART1>;
});

static EP_OUT_BUFFER: StaticCell<[u8; 256]> = StaticCell::new();
static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
static STATE: StaticCell<State> = StaticCell::new();

pub struct Board {
    pub radio_uart: RadioUart,
    pub imu_spi: ImuSpi,
    pub imu_cs: ImuCs,
    pub usb_class: UsbClass,
    pub usb_device: UsbDevice,
}

impl Board {
    pub fn init() -> Board {
        let mut config = embassy_stm32::Config::default();
        {
            use embassy_stm32::rcc::*;
            config.rcc.hse = Some(Hse {
                freq: Hertz(8_000_000),
                mode: HseMode::Oscillator,
            });
            config.rcc.pll_src = PllSource::HSE;
            config.rcc.pll = Some(Pll {
                prediv: PllPreDiv::DIV8,
                mul: PllMul::MUL336,
                divp: Some(PllPDiv::DIV2), // 8mhz / 4 * 168 / 2 = 168Mhz.
                divq: Some(PllQDiv::DIV7), // 8mhz / 4 * 168 / 7 = 48Mhz.
                divr: None,
            });
            config.rcc.ahb_pre = AHBPrescaler::DIV1;
            config.rcc.apb1_pre = APBPrescaler::DIV4;
            config.rcc.apb2_pre = APBPrescaler::DIV2;
            config.rcc.sys = Sysclk::PLL1_P;
            config.rcc.mux.clk48sel = mux::Clk48sel::PLL1_Q;
        }
        let p = embassy_stm32::init(config);

        // init usb connection
        let ep_out_buffer = EP_OUT_BUFFER.init([0; 256]);
        let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);
        let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);
        let control_buf = CONTROL_BUF.init([0; 64]);
        let state = STATE.init(State::new());

        let mut config = usb::Config::default();
        config.vbus_detection = false;
        let driver = usb::Driver::new_fs(p.USB_OTG_FS, Irqs, p.PA12, p.PA11, ep_out_buffer, config);

        let mut config = embassy_usb::Config::new(0xdead, 0xcafe);
        config.manufacturer = Some("tilt-rotor");
        config.product = Some("uav");
        config.serial_number = Some("42");

        let mut usb_builder = Builder::new(
            driver,
            config,
            config_descriptor,
            bos_descriptor,
            &mut [],
            control_buf,
        );

        let usb_class = CdcAcmClass::new(&mut usb_builder, state, 64);
        let usb_device = usb_builder.build();

        // init radio
        let mut radio_uart_config = UsartConfig::default();
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
            radio_uart,
            imu_spi,
            imu_cs,
            usb_class,
            usb_device,
        }
    }
}

pub type EnginePwmType = BlackpillEnginePwm;

const MINIMAL_DUTY: u16 = 150;

pub struct BlackpillEnginePwm {
    pwm: SimplePwm<'static, TIM5>,
    int1: EngineInt1,
    int2: EngineInt2,
    int3: EngineInt3,
    int4: EngineInt4,
}

impl BlackpillEnginePwm {
    pub fn init(
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
            pwm,
            int1,
            int2,
            int3,
            int4,
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

impl EnginePwm for BlackpillEnginePwm {
    fn update(&mut self, _cmd: &Command) {}
}
