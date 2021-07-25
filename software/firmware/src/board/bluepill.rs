use embedded_hal::PwmPin;
use pac::TIM2;
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::pwm::{PwmChannel, C1, C2, C3, C4};
pub use stm32f1xx_hal::stm32 as pac;
use stm32f1xx_hal::time::U32Ext;
use stm32f1xx_hal::timer::{Tim2NoRemap, Timer};

use super::EnginePwm;

pub struct Board {
    pub engines: BluepillEnginePwm,
    // TODO
}

impl Board {
    pub fn init(_core: rtic::Peripherals, device: pac::Peripherals) -> Board {
        let mut flash = device.FLASH.constrain();
        let mut rcc = device.RCC.constrain();

        let clocks = rcc
            .cfgr
            .use_hse(8.mhz())
            .sysclk(48.mhz())
            .pclk1(24.mhz())
            .freeze(&mut flash.acr);

        let mut afio = device.AFIO.constrain(&mut rcc.apb2);
        let mut gpioa = device.GPIOA.split(&mut rcc.apb2);

        // Engine PWM:
        let c1 = gpioa.pa0.into_alternate_push_pull(&mut gpioa.crl);
        let c2 = gpioa.pa1.into_alternate_push_pull(&mut gpioa.crl);
        let c3 = gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl);
        let c4 = gpioa.pa3.into_alternate_push_pull(&mut gpioa.crl);
        let pins = (c1, c2, c3, c4);

        let pwm = Timer::tim2(device.TIM2, &clocks, &mut rcc.apb1).pwm::<Tim2NoRemap, _, _, _>(
            pins,
            &mut afio.mapr,
            50.hz(),
        );

        let engines = BluepillEnginePwm::init(pwm.split());

        Board { engines }
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
