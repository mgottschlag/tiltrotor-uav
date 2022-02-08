#![no_main]
#![no_std]

use cortex_m::peripheral::DWT;
use cortex_m_semihosting::hprintln;
use panic_semihosting as _;
use rtic::cyccnt::U32Ext as _;
use rtt_target::{rprintln, rtt_init_print};

use board::{Board, EnginePwm, RadioInterrupt, RadioInterruptType};
use imu::Imu;
use radio::Radio;

mod board;
mod imu;
mod radio;

#[rtic::app(device = crate::board::pac, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        engines: Engines,
        imu: Imu,
        radio: Radio,
        interrupts: RadioInterruptType,
    }

    #[init]
    fn init(mut ctx: init::Context) -> init::LateResources {
        rtt_init_print!();

        // Initialize (enable) the monotonic timer (CYCCNT)
        ctx.core.DCB.enable_trace();
        // required on Cortex-M7 devices that software lock the DWT (e.g. STM32F7)
        DWT::unlock();
        ctx.core.DWT.enable_cycle_counter();

        let mut board = Board::init(ctx.core, ctx.device);

        rprintln!("Setting up interrupts ...");
        let interrupts = board.interrupts;

        rprintln!("Setting up radio ...");
        let radio = Radio::init(board.radio_spi, board.radio_cs, board.radio_ce);

        rprintln!("Setting up imu ...");
        let imu = Imu::new(
            board.imu_spi,
            board.imu_cs,
            board.imu_irq,
            &mut board.imu_delay,
        );

        rprintln!("Setting up pwm ...");
        let engine_pwm = board.engines;

        init::LateResources {
            engines: Engines {
                engine_pwm,
                engine_speed: 0,
                current_engine: 0,
            },
            imu,
            radio,
            interrupts,
        }
    }

    #[task(schedule = [calibration2], resources = [engines])]
    fn calibration1(mut ctx: calibration1::Context) {
        let engines = &mut ctx.resources.engines;
        let max_duty = engines.engine_pwm.get_max_duty();
        engines.engine_pwm.set_duty([max_duty / 20; 4]);
        ctx.schedule
            .calibration2(ctx.scheduled + (48_000_000 * 4).cycles())
            .unwrap();
    }

    #[task(schedule = [engine_test], resources = [engines])]
    fn calibration2(mut ctx: calibration2::Context) {
        let engines = &mut ctx.resources.engines;
        let max_duty = engines.engine_pwm.get_max_duty();
        engines.engine_pwm.set_duty([max_duty / 10; 4]);
        ctx.schedule
            .engine_test(ctx.scheduled + (48_000_000 * 4).cycles())
            .unwrap();
    }

    #[task(schedule = [engine_test], resources = [engines])]
    fn engine_test(mut ctx: engine_test::Context) {
        let engines = &mut ctx.resources.engines;
        if engines.engine_speed == 100 {
            engines.engine_speed = 0;
            engines.current_engine = (engines.current_engine + 1) & 3;
        } else {
            engines.engine_speed += 10;
        }

        hprintln!(
            "Engine: #{} | Speed: {}",
            engines.current_engine,
            engines.engine_speed
        )
        .ok();

        let max_duty = engines.engine_pwm.get_max_duty() as u32;
        let mut duty = [0; 4];
        // We want between 1-2ms of each 20ms PWM period.
        duty[engines.current_engine] =
            (max_duty / 20 + max_duty * engines.engine_speed / 2000) as u16;
        engines.engine_pwm.set_duty(duty);
        ctx.schedule
            .engine_test(ctx.scheduled + 48_000_000.cycles())
            .unwrap();
    }

    #[task(binds = EXTI15_10, resources = [engines, interrupts, radio])]
    fn radio_irq(ctx: radio_irq::Context) {
        let status = protocol::Status { r: 1.0, p: 2.0 };

        rprintln!("Radio!");
        ctx.resources.interrupts.reset();
        // status is set as ACK payload for the next icoming command
        // TODO: consider two-way protocol to return status as reponse for most recent incoming command
        match ctx.resources.radio.poll(&status) {
            None => {}
            Some(cmd) => {
                rprintln!("Thrust {:?}!", cmd.thrust);
                let max_duty = ctx.resources.engines.engine_pwm.get_max_duty() as u32;
                let mut duty = [0; 4];
                for i in 0..4 {
                    duty[i] = (max_duty / 20 + max_duty / 20 * cmd.thrust[i] as u32 / 256) as u16;
                }
                ctx.resources.engines.engine_pwm.set_duty(duty);
            }
        }
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::nop()
        }
    }

    extern "C" {
        fn EXTI0();
    }
};

pub struct Engines {
    engine_pwm: board::EnginePwmType,
    engine_speed: u32,
    current_engine: usize,
}
