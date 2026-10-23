use rppal::gpio::Gpio;
use std::{error::Error, thread, time::Duration};

// Número BCM do pino (GPIO17 = pino físico 11).
const LED_PIN: u8 = 17;

fn main() -> Result<(), Box<dyn Error>> {
    let gpio = Gpio::new()?;
    let mut led = gpio.get(LED_PIN)?.into_output_low();

    println!("Piscando LED no GPIO{LED_PIN} (Ctrl+C para sair)");

    loop {
        led.toggle();
        thread::sleep(Duration::from_millis(500));
    }
}
