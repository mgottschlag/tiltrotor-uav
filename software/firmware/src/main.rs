#![no_main]
#![no_std]

use panic_halt as _;

use board::{Board, RadioInterruptType};
use radio::Radio;

mod board;
mod radio;

#[rtic::app(device = crate::board::pac, peripherals = true)]
mod app {

    use rtt_target::{rprintln, rtt_init_print};

    use crate::board::{EnginePwm, RadioInterrupt};
    use crate::Board;
    use crate::Engines;
    use crate::Radio;
    use crate::RadioInterruptType;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        engines: Engines,
        radio: Radio,
        interrupts: RadioInterruptType,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();

        // Initialize (enable) the monotonic timer (CYCCNT)
        ctx.core.DCB.enable_trace();
        ctx.core.DWT.enable_cycle_counter();

        let board = Board::init(ctx.core, ctx.device);

        rprintln!("Setting up interrupts ...");
        let interrupts = board.interrupts;

        rprintln!("Setting up pwm ...");
        let engine_pwm = board.engines;

        rprintln!("Setting up radio ...");
        let radio = Radio::init(board.radio_spi, board.radio_cs, board.radio_ce);

        (
            Shared {},
            Local {
                engines: Engines {
                    engine_pwm,
                    engine_speed: 0,
                    current_engine: 0,
                },
                radio,
                interrupts,
            },
            init::Monotonics(),
        )
    }

    #[task(binds = EXTI15_10, local = [engines, interrupts, radio])]
    fn radio_irq(ctx: radio_irq::Context) {
        let status = protocol::Status { r: 1.0, p: 2.0 };

        rprintln!("Radio!");
        ctx.local.interrupts.reset();
        // status is set as ACK payload for the next icoming command
        // TODO: consider two-way protocol to return status as reponse for most recent incoming command
        match ctx.local.radio.poll(&status) {
            None => {}
            Some(cmd) => {
                rprintln!("Thrust {:?}!", cmd.thrust);
                let max_duty = ctx.local.engines.engine_pwm.get_max_duty() as u32;
                let mut duty = [0; 4];
                for i in 0..4 {
                    duty[i] = (max_duty / 20 + max_duty / 20 * cmd.thrust[i] as u32 / 256) as u16;
                }
                ctx.local.engines.engine_pwm.set_duty(duty);
            }
        }
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }
}

pub struct Engines {
    engine_pwm: board::EnginePwmType,
    engine_speed: u32,
    current_engine: usize,
}
