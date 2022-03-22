use core::str::FromStr;
use defmt::Format;
use embedded_graphics::mono_font::iso_8859_13::FONT_6X10;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{Point, RgbColor};
use embedded_graphics::text::renderer::{CharacterStyle, TextRenderer};
use embedded_graphics::text::{Baseline, Text};
use embedded_graphics::Drawable;
use heapless::spsc::Queue;
use heapless::String;
use profont::{PROFONT_18_POINT, PROFONT_24_POINT};

use crate::temp::Temp;
use crate::types::{Display, Temps, SCREEN_WIDTH, SCREEN_WIDTH_PLUS_1, TITLES};

use crate::unit::Unit;

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

pub struct Model {
    pub last: Option<Temps>,
    pub mins: Temps,
    pub maxs: Temps,
    pub history: [Queue<Temp, SCREEN_WIDTH_PLUS_1>; 2],
    pub unit: Unit,
    pub screen_type: ScreenType,
    pub changed: bool,
    pub clear: bool,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            last: Default::default(),
            mins: [i16::MAX.into(), i16::MAX.into()],
            maxs: [i16::MIN.into(), i16::MIN.into()],
            history: Default::default(),
            unit: Default::default(),
            screen_type: Default::default(),
            changed: Default::default(),
            clear: Default::default(),
        }
    }
}

impl Model {
    fn update_min_max(&mut self, temps: Temps) {
        for i in 0..2 {
            self.mins[i] = self.mins[i].min(*temps[i]).into();
            self.maxs[i] = self.maxs[i].max(*temps[i]).into();
        }
    }
    pub fn apply(&mut self, changes: ModelChange) {
        match changes {
            ModelChange::Last(last) => {
                self.clear = false;
                if Some(last) != self.last {
                    self.changed = true;
                    self.last = Some(last);
                    self.update_min_max(last);
                } else {
                    self.changed = false;
                }
            }
            ModelChange::LastAndAverage(last, average) => {
                self.changed = true;
                self.clear = false;
                self.last = Some(last);
                self.update_min_max(last);
                for i in 0..2 {
                    if self.history[i].len() == SCREEN_WIDTH {
                        self.history[i].dequeue();
                    }
                    self.history[i].enqueue(average[i]).unwrap();
                }
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

    pub fn min_or_max(&self, max: bool, index: usize) -> Temp {
        if max {
            self.maxs[index]
        } else {
            self.mins[index]
        }
    }
}

/// Draw the texts that needs only to be re-drawn only on reset
pub fn draw_titles(display: &mut Display, screen_type: ScreenType) {
    match screen_type {
        ScreenType::Both => {
            for i in 0..2usize {
                let mut title = String::<10>::from_str(TITLES[i]).unwrap();
                text_titles(display, &mut title, 0, i as i32 * 64);
            }
        }
        ScreenType::Single(i) => {
            let mut title = String::<10>::from_str(TITLES[i as usize]).unwrap();
            text_titles(display, &mut title, 0, 0);
        }
    }
}

pub fn text<const N: usize, S: TextRenderer<Color = Rgb565>>(
    display: &mut Display,
    buffer: &mut String<N>,
    x: i32,
    y: i32,
    style: S,
) {
    let p = Point::new(x, y);
    Text::with_baseline(buffer.as_str(), p, style, Baseline::Top)
        .draw(display)
        .unwrap();
    buffer.clear()
}

pub fn text_titles<const N: usize>(display: &mut Display, buffer: &mut String<N>, x: i32, y: i32) {
    let mut style = MonoTextStyle::new(&PROFONT_18_POINT, Rgb565::WHITE);
    style.set_background_color(Some(Rgb565::BLACK));
    text(display, buffer, x, y, style)
}

pub fn text_temperature<const N: usize>(
    display: &mut Display,
    buffer: &mut String<N>,
    x: i32,
    y: i32,
    single: bool,
    temp: Temp,
    unit: Unit,
) {
    let color = if temp < 0.into() {
        RgbColor::RED
    } else if temp < 1500.into() {
        RgbColor::YELLOW
    } else {
        RgbColor::GREEN
    };
    let font = if single {
        PROFONT_24_POINT
    } else {
        PROFONT_18_POINT
    };

    temp.write_buffer(unit, true, buffer);

    let mut style = MonoTextStyle::new(&font, color);
    style.set_background_color(Some(Rgb565::BLACK));
    text(display, buffer, x, y, style)
}

pub fn text_small_white<const N: usize>(
    display: &mut Display,
    buffer: &mut String<N>,
    x: i32,
    y: i32,
) {
    let mut style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);
    style.set_background_color(Some(Rgb565::BLACK));
    text(display, buffer, x, y, style)
}
