use gpio::{GpioOut, GpioValue};

trait GpioOutExt: GpioOut {
    fn pulse(&mut self) -> Result<(), Self::Error> {
        self.set_high()?;
        self.set_low()
    }
}

impl<T: GpioOut> GpioOutExt for T {}

pub struct TlcController<Pin> {
    sin: Pin,
    sclk: Pin,
    blank: Pin,
    xlat: Pin,
    gsclk: Pin,
    colors: Vec<u16>,
}

impl<Pin, Error> TlcController<Pin>
where 
    Pin: GpioOut<Error = Error>
{
    pub fn new(
        mut sin: Pin,
        mut sclk: Pin,
        mut blank: Pin,
        mut xlat: Pin,
        mut gsclk: Pin,
        len: usize,
    ) -> Result<Self, Error> {
        vec![&mut sin, &mut sclk, &mut xlat, &mut gsclk].into_iter().try_for_each(GpioOut::set_low)?;
        blank.set_high()?;
        let colors = vec![0; len * 16];
        Ok(Self { sin, sclk, blank, xlat, gsclk, colors })
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
        match (self.colors[channel] & (1 << bit)) >> bit == 0 {
            true => GpioValue::Low,
            false => GpioValue::High,
        }
    }
}
