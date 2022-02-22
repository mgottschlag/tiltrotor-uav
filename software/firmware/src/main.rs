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

    use crate::board::{EnginePwm, PidTimer, RadioInterrupt};
    use crate::imu::Rotations;
    use crate::Board;
    use crate::Engines;
    use crate::Imu;
    use crate::Pid;
    use crate::Radio;
    use crate::RadioInterruptType;

    #[shared]
    struct Shared {
        engines: Engines,
        rotations: Rotations,
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
        let pid_timer = board.pid_timer;
        let pid_pitch = Pid::new(1.0, 0.5, 0.25);
        let pid_roll = Pid::new(1.0, 0.5, 0.25);

        let mono: Systick<100> = Systick::new(board.syst, 16_000_000);
        update::spawn_after(100.millis()).unwrap();
        (
            Shared {
                engines: Engines {
                    pwm: engine_pwm,
                    thrust: [0; 4],
                    pose: [0.0; 2],
                    pid_timer: pid_timer,
                    pid_pitch,
                    pid_roll,
                },
                rotations: Rotations {
                    roll: 0.0,
                    pitch: 0.0,
                    yaw: 0.0,
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

    #[task(local = [imu], shared = [engines, rotations])]
    fn update(ctx: update::Context) {
        update::spawn_after(1000.millis()).unwrap();

        let shared = ctx.shared;
        let local = ctx.local;

        (shared.engines, shared.rotations).lock(|engines, rotations| {
            *rotations = local.imu.get_rotations();
            let duration = engines.pid_timer.elapsed_secs();

            let target_error_pitch = engines.pose[0];
            let target_error_roll = engines.pose[1];

            let c_pitch = engines
                .pid_pitch
                .update(target_error_pitch, rotations.pitch, duration);
            let c_roll = engines
                .pid_roll
                .update(target_error_roll, rotations.roll, duration);

            let mut pwm: [u16; 4] = [0; 4];
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
                "update: pitch={}, roll={}, thrust={:?}, actual={:?}, pwm={:?}, duration={:?}",
                rotations.pitch,
                rotations.roll,
                engines.thrust,
                actual_thrust,
                pwm,
                duration,
            );
        });
    }

    #[task(binds = EXTI15_10, local = [interrupts, radio], shared = [engines, rotations])]
    fn radio_irq(mut ctx: radio_irq::Context) {
        let mut status = protocol::Status { r: 0.0, p: 0.0 };

        (ctx.shared.rotations).lock(|rotations| {
            status.r = rotations.roll;
            status.p = rotations.pitch;
        });

        //rprintln!("Radio!");
        ctx.local.interrupts.reset();
        // status is set as ACK payload for the next icoming command
        // TODO: consider two-way protocol to return status as reponse for most recent incoming command
        match ctx.local.radio.poll(&status) {
            None => {}
            Some(cmd) => {
                (ctx.shared.engines).lock(|engines| {
                    engines.thrust = cmd.thrust;
                    engines.pose = cmd.pose;
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
    pose: [f32; 2],
    pid_timer: board::PidTimerType,
    pid_pitch: Pid,
    pid_roll: Pid,
}

pub struct Pid {
    kp: f32,
    ki: f32,
    kd: f32,
    last_error: f32,
    cum_error: f32,
}

impl Pid {
    pub fn new(kp: f32, ki: f32, kd: f32) -> Self {
        Pid {
            kp,
            ki,
            kd,
            last_error: 0.0,
            cum_error: 0.0,
        }
    }

    pub fn update(&mut self, target_error: f32, actual_error: f32, duration: f32) -> i16 {
        let error = target_error - actual_error; // P
        self.cum_error = self.cum_error + error * duration; // I
        let rate_error = (error - self.last_error) / duration; // D
        self.last_error = error;

        (error * self.kp + self.cum_error * self.ki + rate_error * self.kd) as i16
    }
}
