use defmt::Format;
use embedded_graphics::mono_font::ascii::FONT_8X13;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{Point, RgbColor};
use embedded_graphics::text::{Baseline, Text};
use embedded_graphics::Drawable;

use crate::types::{Display, Queue, Temp, Temps, PERIOD, TITLES};

use crate::unit::{fahrenheit, Unit};

#[derive(Copy, Clone, Format, Debug)]
pub enum ScreenType {
    Both,
    Single(bool),
}

impl ScreenType {
    pub fn next(&mut self) -> Self {
        *self = match self {
            ScreenType::Both => ScreenType::Single(false),
            ScreenType::Single(false) => ScreenType::Single(true),
            ScreenType::Single(true) => ScreenType::Both,
        };
        *self
    }
}

impl Default for ScreenType {
    fn default() -> Self {
        ScreenType::Both
    }
}

#[derive(Debug, Format)]
pub enum ModelChange {
    Last(Temps),
    LastAndAverage(Temps, Temps),
    Unit(Unit),
    ScreenType(ScreenType),
    Clear,
}

#[derive(Default)]
pub struct Model {
    pub last: Temps,
    pub mins: Temps,
    pub maxs: Temps,
    pub history: Queue<Temps>,
    pub unit: Unit,
    pub screen_type: ScreenType,
    pub changed: bool,
    pub clear: bool,
}

impl Model {
    fn update_min_max(&mut self, temps: Temps) {
        for i in 0..2 {
            self.mins[i] = self.mins[i].min(temps[i]);
            self.maxs[i] = self.maxs[i].max(temps[i]);
        }
    }
    pub fn apply(&mut self, changes: ModelChange) {
        match changes {
            ModelChange::Last(last) => {
                if last != self.last {
                    self.changed = false;
                    self.clear = false;
                    self.last = last;
                    self.update_min_max(last);
                }
            }
            ModelChange::LastAndAverage(last, average) => {
                self.changed = true;
                self.clear = false;
                self.last = last;
                self.update_min_max(last);
                if self.history.len() == PERIOD {
                    self.history.dequeue();
                }
                self.history.enqueue(average).unwrap();
            }
            ModelChange::Unit(unit) => {
                self.changed = true;
                self.clear = true;
                self.unit = unit;
            }
            ModelChange::ScreenType(screen_type) => {
                self.changed = true;
                self.clear = true;
                self.screen_type = screen_type;
            }
            ModelChange::Clear => {
                self.changed = true;
                self.clear = true;
            }
        }
    }

    pub fn last_converted(&self) -> Temps {
        match self.unit {
            Unit::Celsius => self.last,
            Unit::Fahrenheit => [fahrenheit(self.last[0]), fahrenheit(self.last[1])],
        }
    }

    pub fn min_or_max_converted(&self, max: bool, index: usize) -> Temp {
        if max {
            match self.unit {
                Unit::Celsius => self.maxs[index],
                Unit::Fahrenheit => fahrenheit(self.maxs[index]),
            }
        } else {
            match self.unit {
                Unit::Celsius => self.mins[index],
                Unit::Fahrenheit => fahrenheit(self.mins[index]),
            }
        }
    }
}

/// Draw the texts that needs only to be re-drawn only on reset
pub fn draw_titles(display: &mut Display, screen_type: ScreenType) {
    let text_style_small = MonoTextStyle::new(&FONT_8X13, Rgb565::WHITE);

    match screen_type {
        ScreenType::Both => {
            for i in 0..2usize {
                Text::with_baseline(
                    TITLES[i],
                    Point::new(0, i as i32 * 64),
                    text_style_small,
                    Baseline::Top,
                )
                .draw(display)
                .unwrap();
            }
        }
        ScreenType::Single(i) => {
            let i = i as usize;
            Text::with_baseline(TITLES[i], Point::new(0, 0), text_style_small, Baseline::Top)
                .draw(display)
                .unwrap();
        }
    }
}
