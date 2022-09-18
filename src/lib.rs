#![no_std]

use gpio::{GpioOut, GpioValue};

// private trait, we want to be able to do pulses
trait GpioOutExt: GpioOut {
    fn pulse(&mut self) -> Result<(), Self::Error> {
        self.set_high()?;
        self.set_low()
    }
}

impl<T: GpioOut> GpioOutExt for T {}

pub struct TlcController<Pin, const LEN: usize> {
    sin: Pin,
    sclk: Pin,
    blank: Pin,
    xlat: Pin,
    gsclk: Pin,
    colors: [u16; LEN],
}

impl<Pin, Error, const LEN: usize> TlcController<Pin, LEN>
where
    Pin: GpioOut<Error = Error>,
{
    pub fn new(
        mut sin: Pin,
        mut sclk: Pin,
        mut blank: Pin,
        mut xlat: Pin,
        mut gsclk: Pin,
    ) -> Result<Self, Error> {
        [&mut sin, &mut sclk, &mut xlat, &mut gsclk]
            .iter_mut()
            .try_for_each(|p| p.set_low())?;
        blank.set_high()?;
        let colors = [0; LEN];
        Ok(Self {
            sin,
            sclk,
            blank,
            xlat,
            gsclk,
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
        let mut channel_counter = (self.colors.len() - 1) as isize;
        let mut gsclk_counter = 0;
        while gsclk_counter < 4096 {
            if channel_counter >= 0 {
                for i in (0..12).rev() {
                    let val = self.get_pin_value_for_channel(channel_counter as usize, i);
                    self.sin.set_value(val)?;
                    self.sclk.pulse()?;
                    self.gsclk.pulse()?;
                    gsclk_counter += 1;
                }
                channel_counter -= 1;
            } else {
                self.sin.set_low()?;
                self.gsclk.pulse()?;
                gsclk_counter += 1
            }
        }
        self.update_post()
    }

    fn update_init(&mut self) -> Result<(), Error> {
        self.blank.set_low()
    }

    fn update_post(&mut self) -> Result<(), Error> {
        self.blank.set_high()?;
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
        let sin = gpio::sysfs::SysFsGpioOutput::open(1).unwrap();
        let sclk = gpio::sysfs::SysFsGpioOutput::open(14).unwrap();
        let blank = gpio::sysfs::SysFsGpioOutput::open(4).unwrap();
        let xlat = gpio::sysfs::SysFsGpioOutput::open(10).unwrap();
        let gsclk = gpio::sysfs::SysFsGpioOutput::open(11).unwrap();

        let mut ctrl = crate::TlcController::<_, LEN>::new(sin, sclk, blank, xlat, gsclk).unwrap();
        ctrl.set_channel(3, 12312);
        ctrl.update().unwrap();
    }
}
