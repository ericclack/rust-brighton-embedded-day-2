#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m;
use cortex_m_rt::entry;

use cortex_m_semihosting::{debug, hprintln};
use crate::hal::{prelude::*, stm32};
use stm32f4xx_hal as hal;
use stm32f4xx_hal::gpio::{ExtiPin, Edge};

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
        button: hal::gpio::gpioc::PC13<hal::gpio::Input<hal::gpio::Floating>>,
        exti: stm32::EXTI,
        delay: hal::delay::Delay            
    }
    
    #[init]
    fn init(_cx: init::Context) -> init::LateResources {
        // Our device and cortex peripherals
        if let (Some(dp), Some(cp)) = (
            stm32::Peripherals::take(),
            cortex_m::peripheral::Peripherals::take())
        {
            // Write the SYSCFGGEN bit to this register in order to enable
            // the system configuration controller (so that changes to
            // dp.SYSCFG take effect).
            //
            // This will be needed for GPIO interrupts to work.  See e.g.
            // "RM0090 Reference manual STM32F405/415, STM32F407/417,
            // STM32F427/437 and STM32F429/439 advanced Arm®-based 32-bit MCUs"
            // sections:
            //
            // 7.3.14 RCC APB2 peripheral clock enable register (RCC_APB2ENR)
            //
            // 9 System configuration controller (SYSCFG)
            //
            dp.RCC.apb2enr.write(|w| w.syscfgen().enabled());
            // Set up the system clock to run at 48MHz
            let rcc = dp.RCC.constrain();
            let clocks = rcc.cfgr.sysclk(48.mhz()).freeze();

            // Set up the LED...
            // First is connected to pin PA5 on the microcontroler
            // The external LED, on the next pin down:
            let gpioa = dp.GPIOA.split();
            let led = gpioa.pa5.into_push_pull_output();
            let xled = gpioa.pa6.into_push_pull_output();

            // Set up a switch as input with interrupt
            let gpioc = dp.GPIOC.split();
            // The microcontroller doesn't try to "pull" a floating input
            // either high or low.  The Nucleo-64 uses an external pull-up
            // resistor on this pin, and also connects a normally-open push
            // switch between it and ground, so that:
            // not pressed = high, pressed = low
            let mut button = gpioc.pc13.into_floating_input();
            // Enable interrupt on falling-edge for this input
            let mut exti = dp.EXTI;
            let mut syscfg = dp.SYSCFG;
            button.make_interrupt_source(&mut syscfg);
            button.enable_interrupt(&mut exti);
            button.trigger_on_edge(&mut exti, Edge::FALLING);

            // Create a delay abstraction based on SysTick
            let delay = hal::delay::Delay::new(cp.SYST, clocks);
            
            init::LateResources{ led, xled, button, exti, delay }
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

            //let pressed = button.is_low();
            //hprintln!("Button {:?}", pressed).unwrap();
            delay.delay_ms(ms);
            counter += 1;
        }
    }

    #[task(binds = EXTI15_10, priority = 2, resources = [button, exti])]
    fn press(cx: press::Context) {
        hprintln!("Interrupt!").unwrap();        
        cx.resources.button.clear_interrupt_pending_bit(cx.resources.exti);
    }

};
