#![no_main]
#![no_std]

use panic_halt as _;

use board::{Board, RadioInterruptType};
use imu::Imu;
use radio::Radio;

mod board;
mod imu;
mod radio;

#[rtic::app(device = crate::board::pac, peripherals = true, dispatchers = [USART1])]
mod app {

    use rtt_target::{rprintln, rtt_init_print};
    use systick_monotonic::*;

    use crate::board::{EnginePwm, RadioInterrupt};
    use crate::Board;
    use crate::Engines;
    use crate::Imu;
    use crate::Radio;
    use crate::RadioInterruptType;

    #[shared]
    struct Shared {
        engines: Engines,
    }

    #[local]
    struct Local {
        imu: Imu,
        radio: Radio,
        interrupts: RadioInterruptType,
    }

    #[monotonic(binds = SysTick, default = true)]
    type MyMono = Systick<100>;

    #[init]
    fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();

        // Initialize (enable) the monotonic timer (CYCCNT)
        ctx.core.DCB.enable_trace();
        ctx.core.DWT.enable_cycle_counter();

        let mut board = Board::init(ctx.core, ctx.device);

        rprintln!("Setting up interrupts ...");
        let interrupts = board.interrupts;

        rprintln!("Setting up radio ...");
        let radio = Radio::init(board.radio_spi, board.radio_cs, board.radio_ce);

        rprintln!("Setting up engines ...");
        let engine_pwm = board.engines;

        rprintln!("Setting up imu ...");
        let imu = Imu::new(
            board.imu_spi,
            board.imu_cs,
            board.imu_irq,
            &mut board.imu_delay,
        );

        let mono: Systick<100> = Systick::new(board.syst, 16_000_000);
        update::spawn_after(100.millis()).unwrap();
        (
            Shared {
                engines: Engines {
                    pwm: engine_pwm,
                    thrust: [0; 4],
                },
            },
            Local {
                imu,
                radio,
                interrupts,
            },
            init::Monotonics(mono),
        )
    }

    #[task(local = [imu], shared = [engines])]
    fn update(mut ctx: update::Context) {
        update::spawn_after(100.millis()).unwrap();

        let data = ctx.local.imu.get_rotations();

        let correction_factor = 1.0;
        let c_pitch = (data.pitch * correction_factor) as i16;
        let c_roll = (data.roll * correction_factor) as i16;

        let mut pwm: [u16; 4] = [0; 4];
        (ctx.shared.engines).lock(|engines| {
            let mut actual_thrust: [i16; 4] = [0; 4];

            actual_thrust[0] = clamp(engines.thrust[0] as i16 + c_pitch - c_roll);
            actual_thrust[1] = clamp(engines.thrust[1] as i16 + c_pitch + c_roll);
            actual_thrust[2] = clamp(engines.thrust[2] as i16 - c_pitch + c_roll);
            actual_thrust[3] = clamp(engines.thrust[3] as i16 - c_pitch - c_roll);

            let max_duty = engines.pwm.get_max_duty() as u32;
            for i in 0..4 {
                pwm[i] = (max_duty / 20 + max_duty / 20 * actual_thrust[i] as u32 / 255) as u16;
            }
            engines.pwm.set_duty(pwm);

            rprintln!(
                "update: pitch={}, roll={}, thrust={:?}, actual={:?}, pwm={:?}",
                data.pitch,
                data.roll,
                engines.thrust,
                actual_thrust,
                pwm
            );
        });
    }

    #[task(binds = EXTI15_10, local = [interrupts, radio], shared = [engines])]
    fn radio_irq(mut ctx: radio_irq::Context) {
        let status = protocol::Status { r: 1.0, p: 2.0 };

        //rprintln!("Radio!");
        ctx.local.interrupts.reset();
        // status is set as ACK payload for the next icoming command
        // TODO: consider two-way protocol to return status as reponse for most recent incoming command
        match ctx.local.radio.poll(&status) {
            None => {}
            Some(cmd) => {
                (ctx.shared.engines).lock(|engines| {
                    engines.thrust = cmd.thrust;
                });
            }
        }
    }

    fn clamp(v: i16) -> i16 {
        if v > 80 {
            return 80;
        }
        if v < 0 {
            return 0;
        }
        return v;
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }
}

pub struct Engines {
    pwm: board::EnginePwmType,
    thrust: [u8; 4],
}
