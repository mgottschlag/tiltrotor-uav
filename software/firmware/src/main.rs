#![no_main]
#![no_std]

use panic_semihosting as _;

#[rtic::app(device = stm32f4xx_hal::stm32, peripherals = true)]
const APP: () = {
    struct Resources {
        _placeholder: u32,
    }

    #[init]
    fn init(_ctx: init::Context) -> init::LateResources {
        init::LateResources { _placeholder: 0 }
    }
};
