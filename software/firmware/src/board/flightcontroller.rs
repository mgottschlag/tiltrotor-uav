use pac::SPI3;
use stm32g4xx_hal::gpio::gpiob::PB5;
use stm32g4xx_hal::gpio::gpioc::{PC10, PC11, PC13, PC14, PC6};
use stm32g4xx_hal::gpio::{Alternate, Floating, Input, Output, PushPull, AF6};
use stm32g4xx_hal::gpio::{ExtiPin, SignalEdge};
use stm32g4xx_hal::prelude::*;
use stm32g4xx_hal::pwm::{ActiveHigh, ComplementaryImpossible, Pwm, C1, C2, C3, C4};
use stm32g4xx_hal::rcc::Config;
use stm32g4xx_hal::spi::{Mode, Phase, Polarity, Spi};
pub use stm32g4xx_hal::stm32 as pac;
use stm32g4xx_hal::syscfg::SysCfgExt;

use super::{EnginePwm, Interrupts};

pub type RadioSck = PC10<Alternate<AF6>>;
pub type RadioMiso = PC11<Alternate<AF6>>;
pub type RadioMosi = PB5<Alternate<AF6>>;
pub type RadioCs = PC6<Output<PushPull>>;
pub type RadioCe = PC13<Output<PushPull>>;
pub type RadioIrq = PC14<Input<Floating>>;
pub type RadioSpi = Spi<SPI3, (RadioSck, RadioMiso, RadioMosi)>;

pub struct Board {
    pub engines: FlightControllerEnginePwm,
    pub radio_spi: RadioSpi,
    pub radio_cs: RadioCs,
    pub radio_ce: RadioCe,
    pub interrupts: FlightControllerInterrupts,
}

impl Board {
    pub fn init(_core: rtic::Peripherals, device: pac::Peripherals) -> Board {
        let mut rcc = device.RCC.constrain();
        let mut syscfg = device.SYSCFG.constrain();

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
            2.mhz(),
            &mut clocks,
        );

        // init interrupts and interrupt handler
        let mut exti = device.EXTI;
        radio_irq.make_interrupt_source(&mut syscfg);
        radio_irq.trigger_on_edge(&mut exti, SignalEdge::Falling);

        let mut interrupts = FlightControllerInterrupts::init(exti, radio_irq);
        interrupts.activate_radio_irq();

        Board {
            engines,
            radio_spi,
            radio_cs,
            radio_ce,
            interrupts,
        }
    }
}

pub type InterruptsType = FlightControllerInterrupts;

pub struct FlightControllerInterrupts {
    exti: pac::EXTI,
    radio_irq: RadioIrq,
}

impl FlightControllerInterrupts {
    pub fn init(exti: pac::EXTI, radio_irq: RadioIrq) -> Self {
        FlightControllerInterrupts { exti, radio_irq }
    }
}

impl Interrupts for FlightControllerInterrupts {
    fn activate_radio_irq(&mut self) {
        self.radio_irq.enable_interrupt(&mut self.exti);
    }

    fn reset_radio_irq(&mut self) {
        self.radio_irq.clear_interrupt_pending_bit();
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
        self.c.0.set_duty(duty[0] as u32);
        self.c.1.set_duty(duty[1] as u32);
        self.c.2.set_duty(duty[2] as u32);
        self.c.3.set_duty(duty[3] as u32);
    }
}
