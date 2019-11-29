# Rust Embedded LEDs project

A Rust project created by the Brighton Rust User Group for the STM32F411 Nucleo-64 Discovery board.

Our aim is to drive some Christmas tree lights like these: 

* 50pcs / lot DC5V/12V input WS2811 pixel module, 12mm black/green Wire led string; Chrismas tree; waterproof with 3pin JST Connectors. https://www.aliexpress.com/item/32444382333.html?spm=a2g0s.9042311.0.0.44ff4c4dSJelZJ

# To do

* Move util functions into their own module
* Now we have interrupts working for the button, make it do something
* Add more LED patterns
* ...

# Running

- Plug in the hardware
- Launch `openocd`
- `cargo run`

This will start up the Gnu Debugger, so type 'c' to continue, or you could set breakpoints, step through the code etc. See the [wiki](../../wiki/debugging) for more info. 

# Documentation

Check out the [wiki](../../wiki)
