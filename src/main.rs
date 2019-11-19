#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m;
use cortex_m_rt::entry;

use cortex_m_semihosting::{debug, hprintln};
use crate::hal::{prelude::*, stm32};
use stm32f4xx_hal as hal;


// LED display pattern, and step size in ms
static PATTERN: [i32; 8] = [1, 1, 1, 0, 1, 0, 1, 0];

/// Treat the array as a ring, i.e. the counter wraps around so
/// that you can repeat the array forever by incrementing counter
fn next_in_ring(an_array: &[i32], counter: usize) -> i32 {
    let slice = counter % an_array.len();
    an_array[slice]
}

#[rtfm::app(device = stm32f4xx_hal::stm32)]
const APP: () = {

    struct Resources {
        led: hal::gpio::gpioa::PA5<hal::gpio::Output<hal::gpio::PushPull>>,
        xled: hal::gpio::gpioa::PA6<hal::gpio::Output<hal::gpio::PushPull>>,
        button: hal::gpio::gpioa::PA1<hal::gpio::Input<hal::gpio::PullDown>>,
        delay: hal::delay::Delay            
    }
    
    #[init]
    fn init(_cx: init::Context) -> init::LateResources {
        // Our device and cortex peripherals
        if let (Some(dp), Some(cp)) = (
            stm32::Peripherals::take(),
            cortex_m::peripheral::Peripherals::take())
        {
            // Set up the LED...
            // First is connected to pin PA5 on the microcontroler
            // The external LED, on the next pin down:
            let gpioa = dp.GPIOA.split();
            let led = gpioa.pa5.into_push_pull_output();
            let xled = gpioa.pa6.into_push_pull_output();

            // Set up a switch as input with interrupt
            let button = gpioa.pa1.into_pull_down_input();
            // TO DO interupt
            
            // Set up the system clock. We want to run at 48MHz
            // because ... ???
            let rcc = dp.RCC.constrain();
            let clocks = rcc.cfgr.sysclk(48.mhz()).freeze();
        
            // Create a delay abstraction based on SysTick
            let delay = hal::delay::Delay::new(cp.SYST, clocks);
            
            init::LateResources{ led, xled, button, delay }
        }
        else {
            panic!("failed to access peripherals");
        }
    }

    #[idle(resources = [led, xled, button, delay])]
    fn idle(cx: idle::Context) -> ! {

        let (led, xled, button, delay) = (
            cx.resources.led,
            cx.resources.xled,
            cx.resources.button,
            cx.resources.delay);
        
        // How quick between LED transitions?
        let ms = 250_u32;    
        let mut counter = 0;
        
        loop {
            if next_in_ring(&PATTERN, counter) == 1 {
                hprintln!("On").unwrap();                    
                led.set_high().unwrap();
                xled.set_high().unwrap();
            }
            else {
                hprintln!("Off").unwrap();
                led.set_low().unwrap();
                xled.set_low().unwrap();                    
            }

            hprintln!("Button {:?}", button.is_low()).unwrap();
            delay.delay_ms(ms);
            counter += 1;
        }
    }

    #[task(binds = EXTI1)]
    fn button_push(_: button_push::Context) {
        hprintln!("Interrupt!").unwrap();
        // https://flowdsp.io/blog/stm32f3-01-interrupts/
    }

};
