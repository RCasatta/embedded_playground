use core::fmt::Write;
use core::ops::Deref;

use defmt::Format;
use heapless::String;

use crate::unit::Unit;

/// A temperature stored in celsius multiplied by 100 (eg. `Temp(100i16) = 1.0°C` )
#[derive(Debug, Format, Default, Clone, Copy, PartialEq, Eq, PartialOrd)]

pub struct Temp(pub i16);

impl From<i16> for Temp {
    fn from(t: i16) -> Self {
        Temp(t)
    }
}

impl Deref for Temp {
    type Target = i16;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Temp {
    /// format into `buffer` this temperature in the given unit for example: `-12.3°C`
    /// 7 characters are always used with unit and 5 character without
    pub fn write_buffer<const N: usize>(&self, unit: Unit, show_unit: bool, buf: &mut String<N>) {
        let val = match unit {
            Unit::Fahrenheit => fahrenheit(self.0),
            Unit::Celsius => self.0,
        };
        let abs_val = val.abs();
        let before_comma = val / 100;
        let after_comma = (abs_val % 100) / 10;
        let need_comma = abs_val < 10_000;

        defmt::debug!("before_comma:{=i16}", before_comma);
        match (need_comma, show_unit) {
            (true, true) => write!(buf, "{}.{}{}", before_comma, after_comma, unit).unwrap(),
            (true, false) => write!(buf, "{}.{}", before_comma, after_comma).unwrap(),
            (false, true) => write!(buf, "{}{}", before_comma, unit).unwrap(),
            (false, false) => write!(buf, "{}", before_comma).unwrap(),
        }
    }
}

/// `degrees` is degrees multiplied by 100, eg 3.31 °C is 331
/// returned value is fahrenheit multiplied by 100, eg 22.41 °F is 2241
pub fn fahrenheit(degrees: i16) -> i16 {
    let f = degrees as i32;
    (f * 9 / 5 + 3200) as i16
}
