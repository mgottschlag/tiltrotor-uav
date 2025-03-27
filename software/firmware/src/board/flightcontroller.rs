use pac::{SPI1, SPI3, TIM1};
use stm32g4xx_hal::delay::DelayFromCountDownTimer;
use stm32g4xx_hal::gpio::gpioa::{PA4, PA5, PA6, PA7};
use stm32g4xx_hal::gpio::gpiob::PB5;
use stm32g4xx_hal::gpio::gpioc::{PC10, PC11, PC13, PC14, PC4, PC6};
use stm32g4xx_hal::gpio::{Alternate, Floating, Input, Output, PushPull, AF5, AF6};
use stm32g4xx_hal::gpio::{ExtiPin, SignalEdge};
use stm32g4xx_hal::prelude::*;
use stm32g4xx_hal::pwm::{ActiveHigh, ComplementaryImpossible, Pwm, C1, C2, C3, C4};
use stm32g4xx_hal::rcc::{Clocks, Config};
use stm32g4xx_hal::spi::{Mode, Phase, Polarity, Spi};
pub use stm32g4xx_hal::stm32 as pac;
pub use stm32g4xx_hal::stm32::{DCB, DWT};
use stm32g4xx_hal::syscfg::SysCfgExt;
use stm32g4xx_hal::timer::{CountDownTimer, Timer};
use stm32g4xx_hal::timer::{Instant, MonoTimer};

use super::{EnginePwm, PidTimer, RadioInterrupt};

pub type RadioSck = PC10<Alternate<AF6>>;
pub type RadioMiso = PC11<Alternate<AF6>>;
pub type RadioMosi = PB5<Alternate<AF6>>;
pub type RadioCs = PC6<Output<PushPull>>;
pub type RadioCe = PC13<Output<PushPull>>;
pub type RadioIrq = PC14<Input<Floating>>;
pub type RadioSpi = Spi<SPI3, (RadioSck, RadioMiso, RadioMosi)>;

pub type ImuSck = PA5<Alternate<AF5>>;
pub type ImuMiso = PA6<Alternate<AF5>>;
pub type ImuMosi = PA7<Alternate<AF5>>;
pub type ImuCs = PC4<Output<PushPull>>;
pub type ImuIrq = PA4<Output<PushPull>>;
pub type ImuSpi = Spi<SPI1, (ImuSck, ImuMiso, ImuMosi)>;
pub type ImuDelay = DelayFromCountDownTimer<CountDownTimer<TIM1>>;

pub type Syst = pac::SYST;

pub struct Board {
    pub syst: Syst,
    pub engines: FlightControllerEnginePwm,
    pub imu_spi: ImuSpi,
    pub imu_cs: ImuCs,
    pub imu_irq: ImuIrq,
    pub imu_delay: DelayFromCountDownTimer<CountDownTimer<TIM1>>,
    pub radio_spi: RadioSpi,
    pub radio_cs: RadioCs,
    pub radio_ce: RadioCe,
    pub interrupts: FlightControllerRadioInterrupt,
    pub pid_timer: FlightControllerPidTimer,
}

impl Board {
    pub fn init(core: rtic::export::Peripherals, device: pac::Peripherals) -> Board {
        let mut rcc = device.RCC.constrain();
        let rcc_clocks = rcc.clocks;
        let mut syscfg = device.SYSCFG.constrain();
        let syst = core.SYST;

        let gpioa = device.GPIOA.split(&mut rcc);
        let gpiob = device.GPIOB.split(&mut rcc);
        let gpioc = device.GPIOC.split(&mut rcc);

        let mut clocks = rcc.freeze(Config::hsi());

        // init pwm
        let c1 = gpioa.pa0.into_alternate();
        let c2 = gpioa.pa1.into_alternate();
        let c3 = gpioa.pa2.into_alternate();
        let c4 = gpioa.pa3.into_alternate();
        let pins = (c1, c2, c3, c4);
        let pwm = device.TIM2.pwm(pins, 50.hz(), &mut clocks);
        let engines = FlightControllerEnginePwm::init(pwm);

        // init radio
        let radio_sck = gpioc.pc10.into_alternate();
        let radio_miso = gpioc.pc11.into_alternate();
        let radio_mosi = gpiob.pb5.into_alternate();
        let radio_cs = gpioc.pc6.into_push_pull_output();
        let radio_ce = gpioc.pc13.into_push_pull_output();
        let mut radio_irq = gpioc.pc14.into_floating_input();
        let radio_spi = device.SPI3.spi(
            (radio_sck, radio_miso, radio_mosi),
            Mode {
                polarity: Polarity::IdleLow,
                phase: Phase::CaptureOnFirstTransition,
            },
            2_000_000.hz(),
            &mut clocks,
        );

        // init imu
        let imu_sck = gpioa.pa5.into_alternate();
        let imu_miso = gpioa.pa6.into_alternate();
        let imu_mosi = gpioa.pa7.into_alternate();
        let imu_cs = gpioc.pc4.into_push_pull_output();
        let imu_irq = gpioa.pa4.into_push_pull_output();
        let imu_spi = device.SPI1.spi(
            (imu_sck, imu_miso, imu_mosi),
            Mode {
                polarity: Polarity::IdleLow,
                phase: Phase::CaptureOnFirstTransition,
            },
            2_000_000.hz(),
            &mut clocks,
        );
        let imu_timer = Timer::new(device.TIM1, &rcc_clocks);
        let imu_delay = DelayFromCountDownTimer::new(imu_timer.start_count_down(1.ms()));

        // init interrupts and interrupt handler
        let mut exti = device.EXTI;
        radio_irq.make_interrupt_source(&mut syscfg);
        radio_irq.trigger_on_edge(&mut exti, SignalEdge::Falling);

        let mut interrupts = FlightControllerRadioInterrupt::init(exti, radio_irq);
        interrupts.activate();

        let pid_timer = FlightControllerPidTimer::new(core.DWT, core.DCB, &rcc_clocks);

        Board {
            syst,
            engines,
            imu_spi,
            imu_cs,
            imu_irq,
            imu_delay,
            radio_spi,
            radio_cs,
            radio_ce,
            interrupts,
            pid_timer,
        }
    }
}

pub type RadioInterruptType = FlightControllerRadioInterrupt;

pub struct FlightControllerRadioInterrupt {
    exti: pac::EXTI,
    irq: RadioIrq,
}

impl FlightControllerRadioInterrupt {
    pub fn init(exti: pac::EXTI, irq: RadioIrq) -> Self {
        FlightControllerRadioInterrupt { exti, irq }
    }
}

impl RadioInterrupt for FlightControllerRadioInterrupt {
    fn activate(&mut self) {
        self.irq.enable_interrupt(&mut self.exti);
    }

    fn reset(&mut self) {
        self.irq.clear_interrupt_pending_bit();
    }
}

pub type EnginePwmType = FlightControllerEnginePwm;

pub struct FlightControllerEnginePwm {
    c: (
        Pwm<pac::TIM2, C1, ComplementaryImpossible, ActiveHigh, ActiveHigh>,
        Pwm<pac::TIM2, C2, ComplementaryImpossible, ActiveHigh, ActiveHigh>,
        Pwm<pac::TIM2, C3, ComplementaryImpossible, ActiveHigh, ActiveHigh>,
        Pwm<pac::TIM2, C4, ComplementaryImpossible, ActiveHigh, ActiveHigh>,
    ),
}

impl FlightControllerEnginePwm {
    pub fn init(
        mut c: (
            Pwm<pac::TIM2, C1, ComplementaryImpossible, ActiveHigh, ActiveHigh>,
            Pwm<pac::TIM2, C2, ComplementaryImpossible, ActiveHigh, ActiveHigh>,
            Pwm<pac::TIM2, C3, ComplementaryImpossible, ActiveHigh, ActiveHigh>,
            Pwm<pac::TIM2, C4, ComplementaryImpossible, ActiveHigh, ActiveHigh>,
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
        FlightControllerEnginePwm { c }
    }
}

impl EnginePwm for FlightControllerEnginePwm {
    fn get_max_duty(&self) -> u16 {
        self.c.0.get_max_duty() as u16
    }
    fn set_duty(&mut self, duty: [u16; 4]) {
        self.c.0.set_duty((duty[1] as u32) * 5);
        self.c.1.set_duty((duty[0] as u32) * 5);
        self.c.2.set_duty((duty[3] as u32) * 5);
        self.c.3.set_duty((duty[2] as u32) * 5);
    }
}

pub type PidTimerType = FlightControllerPidTimer;

pub struct FlightControllerPidTimer {
    mono_timer: MonoTimer,
    instant: Instant,
}

impl FlightControllerPidTimer {
    pub fn new(dwt: DWT, dcb: DCB, clocks: &Clocks) -> Self {
        let mono_timer = MonoTimer::new(dwt, dcb, clocks);
        let instant = mono_timer.now();
        FlightControllerPidTimer {
            mono_timer,
            instant,
        }
    }
}

impl PidTimer for FlightControllerPidTimer {
    fn elapsed_secs(&mut self) -> f32 {
        let res = self.instant.elapsed();
        self.instant = self.mono_timer.now();
        res as f32 / 16_000_000 as f32 // TODO get frequency programatically
    }
}
