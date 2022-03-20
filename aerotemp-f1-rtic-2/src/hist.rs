//! hist
//!
//! This module provides implementation to draw histograms on a Display
//!

use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::prelude::{DrawTarget, PixelColor, Primitive};
use embedded_graphics::primitives::{Line, PrimitiveStyle};
use embedded_graphics::Drawable;
use heapless::spsc::Queue;

use crate::temp::Temp;

/// Represent a histogram with values contained in the `ring` but rescaled to fit in the window
/// defined by the `upper_left` and `lower_right` points
#[derive(Debug)]
pub struct Hist {
    upper_left: Point,
    size: Size,
}

/// A struct containing three points
pub type ThreePoints = [Point; 3];

/// Errors in creating the histogram
#[derive(Debug)]
pub enum Error {
    /// Draw error
    DrawError,
}

impl Hist {
    /// Create an Hist, checking if parameters are valid
    pub fn new(upper_left: Point, size: Size) -> Hist {
        Hist { upper_left, size }
    }

    /// Draw the histogram on a display
    pub fn draw<C: PixelColor, D: DrawTarget<Color = C>, const N: usize>(
        &self,
        queue: &Queue<Temp, N>,
        display: &mut D,
        foreground: C,
        background: C,
    ) -> Result<(), Error> {
        let lines = self.draw_lines(queue).unwrap();
        for points in lines.iter() {
            Line::new(points[0], points[1])
                .into_styled(PrimitiveStyle::with_stroke(foreground, 1))
                .draw(display)
                .map_err(|_| Error::DrawError)?;
            Line::new(points[1], points[2])
                .into_styled(PrimitiveStyle::with_stroke(background, 1))
                .draw(display)
                .map_err(|_| Error::DrawError)?;
        }
        Ok(())
    }

    /// internal testable method, returning N tuples of 3 points (A,B,C)
    /// A->B will be foreground colored while B-C will be background colored
    fn draw_lines<const N: usize>(
        &self,
        array: &Queue<Temp, N>,
    ) -> Result<[ThreePoints; N], Error> {
        let mut result = [ThreePoints::default(); N];

        if array.len() > 0 {
            let (min, max) = min_max(&array);
            let delta = max - min;
            let baseline_y = self.upper_left.y + self.size.height as i32;
            let baseline_x = self.upper_left.x as usize;

            // start from right
            for (i, val) in array.iter().enumerate() {
                let x = (baseline_x + self.size.width as usize - array.len() + i) as i32;

                let mut zero_one = ((val.0 - min) as f32) / delta as f32;
                if zero_one.is_nan() {
                    zero_one = 0.5;
                }
                let rescaled = (zero_one * self.size.height as f32) as i32;

                let a = Point::new(x, baseline_y);
                let b = Point::new(x, baseline_y - rescaled);
                let c = Point::new(x, baseline_y - self.size.height as i32 + 1);
                defmt::trace!(
                    "a: {=i32} {=i32} b: {=i32} {=i32} c: {=i32} {=i32}",
                    a.x,
                    a.y,
                    b.x,
                    b.y,
                    c.x,
                    c.y
                );
                result[i] = [a, b, c];
            }
        }
        Ok(result)
    }
}

fn min_max<const N: usize>(array: &Queue<Temp, N>) -> (i16, i16) {
    let mut min = i16::MAX;
    let mut max = i16::MIN;
    for i in array.iter() {
        min = min.min(i.0);
        max = max.max(i.0);
    }
    (min, max)
}
