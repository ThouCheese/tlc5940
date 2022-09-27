use gpio::{GpioOut, GpioValue};
use rppal::{
    pwm::{Channel, Polarity, Pwm},
    spi::{Bus, Mode, SlaveSelect, Spi, reverse_bits},
};

/// paars: sin
/// grijs: blank
/// blauw: sclk
/// wit: xlat
/// zwart: gsclk

fn main() {
    const LEN: usize = 48;
    let blank = gpio::sysfs::SysFsGpioOutput::open(27).unwrap();
    let xlat = gpio::sysfs::SysFsGpioOutput::open(25).unwrap();
    let channel = Channel::Pwm0;
    let bus = Bus::Spi0;
    
    let mut ctrl = crate::TlcController::<_, LEN>::new(blank, xlat, channel, bus).unwrap();
    
    let mut index: i32 = 0;
    let mut direction: i32 = 1;
    for _ in 0..200 {
        ctrl.clear();
        ctrl.set_channel(index as usize, 512);
        ctrl.update().unwrap();

        if index == 47 {
            direction = -1;
        }
        if index == 0 {
            direction = 1;
        }
        index += direction;
        
        for _ in 0..10 {
            ctrl.pulse_blank().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

// private trait, we want to be able to do pulses
trait GpioOutExt: GpioOut {
    fn pulse(&mut self) -> Result<(), Self::Error> {
        self.set_high()?;
        self.set_low()
    }
}

impl<T: GpioOut> GpioOutExt for T {}

pub struct TlcController<Pin, const LEN: usize> {
    blank: Pin,
    xlat: Pin,
    _gsclk: Pwm,
    _spi: Spi,
    colors: [u16; LEN],
}

impl<Pin, Error, const LEN: usize> TlcController<Pin, LEN>
where
    Pin: GpioOut<Error = Error>,
{
    pub fn new(
        mut blank: Pin,
        mut xlat: Pin,
        channel: Channel,
        bus: Bus,
    ) -> Result<Self, Error> {
        xlat.set_low()?;
        blank.set_high()?;
        let colors = [0; LEN];
        let _gsclk = Pwm::with_frequency(
            channel, 
            409_600.0, 
            0.50, 
            Polarity::Normal, 
            true
        ).unwrap();

        let _spi = Spi::new(bus, SlaveSelect::Ss0, 100_000, Mode::Mode0).unwrap();

        Ok(Self {
            blank,
            xlat,
            _gsclk,
            _spi,
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
        std::thread::sleep(std::time::Duration::from_millis(10));
        self.update_post()
    }

    pub fn pulse_blank(&mut self) -> Result<(), Error> {
        self.blank.pulse()
    }

    fn update_init(&mut self) -> Result<(), Error> {
        self.blank.set_high()
    }

    fn update_post(&mut self) -> Result<(), Error> {
        self.blank.set_low()?;
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
