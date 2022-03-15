use defmt::Format;
use embedded_graphics::mono_font::ascii::FONT_8X13;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{Point, RgbColor};
use embedded_graphics::text::{Baseline, Text};
use embedded_graphics::Drawable;

use crate::types::{Display, Queue, Temps, PERIOD};

use crate::unit::Unit;

#[derive(Copy, Clone, Format, Debug)]
pub enum ScreenType {
    Both,
    Single(bool),
}

impl ScreenType {
    pub fn next(&mut self) -> Self {
        *self = match self {
            ScreenType::Both => ScreenType::Single(true),
            ScreenType::Single(true) => ScreenType::Single(false),
            ScreenType::Single(false) => ScreenType::Both,
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
}

#[derive(Default)]
pub struct Model {
    pub last: Temps,
    pub history: Queue<Temps>,
    pub unit: Unit,
    pub screen_type: ScreenType,
    pub changed: bool,
    pub clear: bool,
}

impl Model {
    pub fn apply(&mut self, changes: ModelChange) {
        match changes {
            ModelChange::Last(last) => {
                if last != self.last {
                    self.changed = false;
                    self.last = last
                }
            }
            ModelChange::LastAndAverage(last, average) => {
                self.changed = true;
                self.last = last;
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
        }
    }
}

/// Draw the texts that needs only to be re-drawn only on reset
pub fn _draw_titles<const N: usize>(display: &mut Display, _screen_type: ScreenType) {
    let text_style_small = MonoTextStyle::new(&FONT_8X13, Rgb565::WHITE);

    Text::with_baseline("OAT", Point::new(0, 0), text_style_small, Baseline::Top)
        .draw(display)
        .unwrap();

    Text::with_baseline("CAT", Point::new(0, 60), text_style_small, Baseline::Top)
        .draw(display)
        .unwrap();
}
