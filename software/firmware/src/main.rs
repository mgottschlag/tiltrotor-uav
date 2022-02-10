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
        //update::spawn_after(1.secs()).unwrap();
        (
            Shared {
                engines: Engines {
                    engine_pwm,
                    engine_speed: [0; 4],
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

    /*#[task(local = [imu], shared = [engines])]
    fn update(mut ctx: update::Context) {
        update::spawn_after(1.secs()).unwrap();

        let data = ctx.local.imu.get_rotations();
        rprintln!("{} {}", data.pitch, data.roll);

        let correction_factor = 1.0;
        let c_pitch = ((data.pitch - 90.0) / 90.0 * correction_factor) as u16;
        let c_roll = ((data.roll - 90.0) / 90.0 * correction_factor) as u16;

        (ctx.shared.engines).lock(|engines| {
            engines.engine_speed[0] = clamp(engines.engine_speed[0] + c_pitch + c_roll);
            engines.engine_speed[1] = clamp(engines.engine_speed[1] + c_pitch - c_roll);
            engines.engine_speed[2] = clamp(engines.engine_speed[2] - c_pitch + c_roll);
            engines.engine_speed[3] = clamp(engines.engine_speed[3] - c_pitch + c_roll);

            rprintln!("Engines!: {:?}", engines.engine_speed);
            engines.engine_pwm.set_duty(engines.engine_speed);
        });
    }*/

    #[task(binds = EXTI15_10, local = [interrupts, radio], shared = [engines])]
    fn radio_irq(mut ctx: radio_irq::Context) {
        let status = protocol::Status { r: 1.0, p: 2.0 };

        rprintln!("Radio!");
        ctx.local.interrupts.reset();
        // status is set as ACK payload for the next icoming command
        // TODO: consider two-way protocol to return status as reponse for most recent incoming command
        match ctx.local.radio.poll(&status) {
            None => {}
            Some(cmd) => {
                rprintln!("Thrust {:?}!", cmd.thrust);
                (ctx.shared.engines).lock(|engines| {
                    let max_duty = engines.engine_pwm.get_max_duty() as u32;
                    for i in 0..4 {
                        engines.engine_speed[i] =
                            (max_duty / 20 + max_duty / 20 * cmd.thrust[i] as u32 / 256) as u16;
                    }

                    rprintln!("Engines?: {:?}", engines.engine_speed);
                    engines.engine_pwm.set_duty(engines.engine_speed);
                });
            }
        }
    }

    fn clamp(v: u16) -> u16 {
        if v > 255 {
            return 255;
        }
        return v;
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }
}

pub struct Engines {
    engine_pwm: board::EnginePwmType,
    engine_speed: [u16; 4],
}
