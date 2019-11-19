#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m;
use cortex_m_rt::entry;

use cortex_m_semihosting::{debug, hprintln};
use crate::hal::{prelude::*, stm32};
use stm32f4xx_hal as hal;

/// Treat the array as a ring, i.e. the counter wraps around so
/// that you can repeat the array forever by incrementing counter
fn next_in_ring(an_array: &[i32], counter: usize) -> i32 {
    let slice = counter % an_array.len();
    an_array[slice]
}

#[rtfm::app(device = stm32f4xx_hal)]
const APP: () = {
    #[init]
    fn init(_: init::Context) {
        // Access the device peripherals (dp) and cortex peripherals (cp):
        if let (Some(dp), Some(cp)) = (
            stm32::Peripherals::take(),
            cortex_m::peripheral::Peripherals::take(),
        ) {
            // Set up the LED: it's connected to pin PA5 on the microcontroler
            let gpioa = dp.GPIOA.split();
            let mut led = gpioa.pa5.into_push_pull_output();
            
            // The external LED, on the next pin down:
            let mut xled = gpioa.pa6.into_push_pull_output();
            
            // Set up the system clock. We want to run at 48MHz for this one.
            let rcc = dp.RCC.constrain();
            let clocks = rcc.cfgr.sysclk(48.mhz()).freeze();
            
            // Create a delay abstraction based on SysTick
            let mut delay = hal::delay::Delay::new(cp.SYST, clocks);
            
            // LED display pattern, and step size in ms
            let pattern = [1, 1, 1, 0, 1, 0, 1, 0];
            let ms = 250_u32;    
            let mut counter = 0;
            
            loop {
                if next_in_ring(&pattern, counter) == 1 {
                    hprintln!("On").unwrap();                    
                    led.set_high().unwrap();
                    xled.set_high().unwrap();
                }
                else {
                    hprintln!("Off").unwrap();
                    led.set_low().unwrap();
                    xled.set_low().unwrap();                    
                }
                
                delay.delay_ms(ms);
                counter += 1;
            }
        } else {
            panic!("failed to access peripherals");
        }
    }

};
