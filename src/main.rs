#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m;
use cortex_m_rt::entry;
use cortex_m::iprint;
use cortex_m_semihosting::{hprintln};

use crate::hal::{prelude::*, stm32};
use hal::spi::*;
use stm32f4xx_hal as hal;
use stm32f4xx_hal::gpio::{ExtiPin, Edge};

use ws2812_spi as ws2812;
use crate::ws2812::Ws2812;

use smart_leds_trait::RGB8;
use smart_leds::SmartLedsWrite;

/// Treat the array as a ring, i.e. the counter wraps around so
/// that you can repeat the array forever by incrementing counter
fn next_in_ring(an_array: &[i32], counter: usize) -> i32 {
    let slice = counter % an_array.len();
    an_array[slice]
}

fn rotate_array(data: &mut [RGB8]) {
    let temp: RGB8 = data[0];
    for i in 0..data.len()-1 {
        data[i] = data[i+1];
    }
    data[data.len()-1] = temp;
}

#[rtfm::app(device = stm32f4xx_hal::stm32)]
const APP: () = {

    struct Resources {
        led: hal::gpio::gpioa::PA5<hal::gpio::Output<hal::gpio::PushPull>>,
        xled: hal::gpio::gpioa::PA6<hal::gpio::Output<hal::gpio::PushPull>>,
        button: hal::gpio::gpioc::PC13<hal::gpio::Input<hal::gpio::Floating>>,
        exti: stm32::EXTI,
        delay: hal::delay::Delay,
        itm: cortex_m::peripheral::ITM,
        
        led_controller: ws2812_spi::Ws2812<stm32f4xx_hal::spi::Spi<stm32f4xx_hal::stm32::SPI1, (stm32f4xx_hal::gpio::gpiob::PB3<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::AF5>>, stm32f4xx_hal::spi::NoMiso, stm32f4xx_hal::gpio::gpiob::PB5<stm32f4xx_hal::gpio::Alternate<stm32f4xx_hal::gpio::AF5>>)>>
    }
    
    #[init]
    fn init(cx: init::Context) -> init::LateResources {
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

            // ITM for logging
            let mut itm = cx.core.ITM;
            let log = &mut itm.stim[0];
            iprint!(log, "Init");

            // Configure pins for SPI
            // We don't connect sck, but I think the SPI traits require it?
            let gpiob = dp.GPIOB.split();
            let sck = gpiob.pb3.into_alternate_af5();
            // Master Out Slave In - pb5, Nucleo 64 pin d4
            let mosi = gpiob.pb5.into_alternate_af5();
            
            let spi = Spi::spi1(
                dp.SPI1,
                (sck, NoMiso, mosi),
                Mode {
                    polarity: Polarity::IdleLow,
                    phase: Phase::CaptureOnFirstTransition,
                },
                stm32f4xx_hal::time::KiloHertz(3000).into(),
                clocks,
            );
            let mut led_controller = Ws2812::new(spi);

            init::LateResources{ led, xled, button, exti, delay, itm, led_controller }
        }
        else {
            panic!("failed to access peripherals");
        }
    }

    #[idle(resources = [led, xled, delay, itm, led_controller])]
    fn idle(cx: idle::Context) -> ! {

        let (led, xled, delay, led_controller) = (
            cx.resources.led,
            cx.resources.xled,
            cx.resources.delay,
            cx.resources.led_controller);

        // Logging setup
        let log = &mut cx.resources.itm.stim[0];
        
        // LED patterns
        let mut data: [RGB8; 50] = [RGB8::default(); 50];

        let palette = [ RGB8{ r: 0x60, g: 0,    b: 0    }, 
                        RGB8{ r: 0x60, g: 0x60, b: 0    }, 
                        RGB8{ r: 0,    g: 0x60, b: 0    }, 
                        RGB8{ r: 0,    g: 0x60, b: 0x60 }, 
                        RGB8{ r: 0,    g: 0,    b: 0x60 } ];

        // Possible that r and g are transposed with our
        // LED hardware
        for block in 0..5 {
            for i in 0..3 {                
                let led = (block*10+i) as usize;
                let colour = palette[block];
                let scale = (i+1) as u8;
                data[led] = RGB8{ r: colour.r / scale,
                                  g: colour.g / scale,
                                  b: colour.b / scale };
            }
        }
        
        // How quick between LED transitions?
        let ms = 250_u32;    

        loop {
            led_controller.write(data.iter().cloned()).unwrap();
            delay.delay_ms(ms);

            rotate_array(&mut data);
        }
    }

    #[task(binds = EXTI15_10, priority = 2, resources = [button, exti])]
    fn press(cx: press::Context) {

        // Logging setup
        //let log = &mut cx.resources.itm.stim[0];
        //iprint!(log, "Interrupt!");
        // ...doesn't work here, use hprintln!
        
        hprintln!("Interrupt!").unwrap();
        cx.resources.button.clear_interrupt_pending_bit(cx.resources.exti);
    }

};
