use core::fmt;

use defmt::Format;
use embedded_graphics::{pixelcolor::Rgb565, prelude::RgbColor};

#[derive(Copy, Clone, Format, Debug)]
pub enum Unit {
    Celsius,
    Fahrenheit,
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Unit::Celsius => write!(f, "°C"),
            Unit::Fahrenheit => write!(f, "°F"),
        }
    }
}

impl Default for Unit {
    fn default() -> Self {
        Unit::Celsius
    }
}

impl Unit {
    pub fn next(&mut self) -> Self {
        *self = match self {
            Unit::Celsius => Unit::Fahrenheit,
            Unit::Fahrenheit => Unit::Celsius,
        };
        *self
    }
}

/// `degrees` is degrees multiplied by 100, eg 3.31 °C is 331
/// returned value is fahrenheit multiplied by 100, eg 22.41 °F is 2241
pub fn fahrenheit(degrees: i16) -> i16 {
    let f = degrees as i32;
    (f * 9 / 5 + 3200) as i16
}

/// color of the text printing degrees
pub fn _color(degrees: i16) -> Rgb565 {
    if degrees < 0 {
        RgbColor::RED
    } else if degrees < 1500 {
        RgbColor::YELLOW
    } else {
        RgbColor::GREEN
    }
}

/// format a value multiplied by 100 into a decimal number with 1 digit after the dot
pub fn format_100<T: fmt::Write>(val: i16, buf: &mut T) {
    let sign = if val >= 0 { "" } else { "-" };
    let abs_val = val.abs();
    let before_comma = abs_val / 100;
    let after_comma = (abs_val % 100) / 10;
    write!(buf, "{}{}.{}", sign, before_comma, after_comma).unwrap();
}
