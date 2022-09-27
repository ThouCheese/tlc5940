#![no_std]

use gpio::{GpioOut, GpioValue};
use rppal::{
    pwm::{Channel, Polarity, Pwm},
    spi::{Bus, Mode, SlaveSelect, Spi},
};

// private trait, we want to be able to do pulses
trait GpioOutExt: GpioOut {
    fn pulse(&mut self) -> Result<(), Self::Error> {
        self.set_high()?;
        self.set_low()
    }
}

impl<T: GpioOut> GpioOutExt for T {}

pub struct TlcController<Pin, const LEN: usize> {
    _spi: Spi,
    _gsclk: Pwm,
    blank: Pwm,
    xlat: Pin,
    colors: [u16; LEN],
}

impl<Pin, Error, const LEN: usize> TlcController<Pin, LEN>
where
    Pin: GpioOut<Error = Error>,
{
    pub fn new(
        bus: Bus,
        gsclk_channel: Channel,
        blank_channel: Channel,        
        mut xlat: Pin,
    ) -> Result<Self, Error> {
        xlat.set_low()?;
        let colors = [0; LEN];
        let _gsclk = Pwm::with_frequency(
            gsclk_channel, 
            409_600.0, 
            0.50, 
            Polarity::Normal, 
            true
        ).unwrap();

        let _spi = Spi::new(bus, SlaveSelect::Ss0, 100_000, Mode::Mode0).unwrap();

        let blank = Pwm::with_frequency(
            blank_channel, 
            100.0, 
            0.50, 
            Polarity::Normal, 
            true
        ).unwrap(); 

        Ok(Self {
            _spi,
            _gsclk,
            blank,
            xlat,
            colors,
        })
    }

    pub fn set_channel(&mut self, channel: usize, color: u16) {
        self.colors[channel] = color;
    }

    pub fn set_all(&mut self, value: u16) {
        self.colors.iter_mut().for_each(|num| *num = value);
    }

    pub fn clear(&mut self) {
        self.set_all(0);
    }

    pub fn update(&mut self) -> Result<(), Error> {
        self.update_init()?;
        let mut buffer = [0u8; 72];
        
        let mut channel_counter = (self.colors.len() - 1) as isize;
        let mut bit_counter = 0;

        while channel_counter >= 0 {
            for i in (0..12).rev() {
                let buf_index = bit_counter / 8;
                let bit_index = bit_counter % 8;
                let val = (((self.colors[channel_counter as usize] & (1 << i)) >> i) << bit_index) as u8;
                buffer[buf_index] |= val;
                bit_counter += 1;
            }
            channel_counter -= 1;
        }
        // reverse_bits(&mut buffer);
        self._spi.write(&buffer).unwrap();
        self.update_post()
    }

    fn update_init(&mut self) -> Result<(), Error> {
        self.blank.set_duty_cycle(1.0).unwrap();
        Ok(())
    }

    fn update_post(&mut self) -> Result<(), Error> {
        self.blank.set_duty_cycle(0.5).unwrap();
        self.xlat.pulse()?;
        Ok(())
    }

    fn get_pin_value_for_channel(&self, channel: usize, bit: u8) -> GpioValue {
        match (self.colors[channel] & (1 << bit)) >> bit {
            0 => GpioValue::Low,
            1 => GpioValue::High,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        const LEN: usize = 48;
        let xlat = gpio::sysfs::SysFsGpioOutput::open(10).unwrap();
        let bus = rppal::spi::Bus::Spi0;

        let gsclk_channel = rppal::pwm::Channel::Pwm0;
        let blank_channel = rppal::pwm::Channel::Pwm1;

        let mut ctrl = crate::TlcController::<_, LEN>::new(bus, gsclk_channel, blank_channel, xlat).unwrap();

        ctrl.set_channel(3, 2312);
        ctrl.update().unwrap();
    }
}
