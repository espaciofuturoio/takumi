use std::ops::Range;

use tiny_skia::Pixmap;

use crate::{geometry::Point, style::Homography};

/// Redraws `layer`, whose top-left pixel sits at device `origin`, through the device-space
/// `projection`, sampling the flat paint bilinearly as Blink composites a 3D-transformed layer.
/// Only the pixels the projected layer can reach are resampled.
pub(crate) fn project_layer(layer: &mut Pixmap, origin: Point<u32>, projection: Homography) {
  let Some(inverse) = projection.invert() else {
    layer.fill(tiny_skia::Color::TRANSPARENT);
    return;
  };
  let width = layer.width() as usize;
  let height = layer.height() as usize;
  let (columns, rows) =
    projected_extent(projection, origin, width, height).unwrap_or((0..width, 0..height));
  let source = layer.data().to_vec();
  let target = layer.data_mut();
  let [a, b, c, d, e, f, g, h, i] = inverse.0;
  let (offset_x, offset_y) = (origin.x as f32 + 0.5, origin.y as f32 + 0.5);

  target.fill(0);

  for y in rows {
    let device_y = offset_y + y as f32;
    let device_x = offset_x + columns.start as f32;
    let mut numerator_x = a * device_x + b * device_y + c;
    let mut numerator_y = d * device_x + e * device_y + f;
    let mut denominator = g * device_x + h * device_y + i;
    let row = y * width * 4;

    for x in columns.clone() {
      if denominator > f32::EPSILON {
        let source_x = numerator_x / denominator - offset_x;
        let source_y = numerator_y / denominator - offset_y;
        let offset = row + x * 4;

        target[offset..offset + 4]
          .copy_from_slice(&sample_bilinear(&source, width, height, source_x, source_y));
      }
      numerator_x += a;
      numerator_y += d;
      denominator += g;
    }
  }
}

fn sample_bilinear(source: &[u8], width: usize, height: usize, x: f32, y: f32) -> [u8; 4] {
  if x <= -1.0 || y <= -1.0 || x >= width as f32 || y >= height as f32 {
    return [0; 4];
  }
  let x0 = x.floor();
  let y0 = y.floor();
  let fx = x - x0;
  let fy = y - y0;
  let (x0, y0) = (x0 as isize, y0 as isize);
  let texel = |tx: isize, ty: isize| -> [f32; 4] {
    if tx < 0 || ty < 0 || tx as usize >= width || ty as usize >= height {
      return [0.0; 4];
    }
    let offset = (ty as usize * width + tx as usize) * 4;
    let pixel = &source[offset..offset + 4];

    [
      f32::from(pixel[0]),
      f32::from(pixel[1]),
      f32::from(pixel[2]),
      f32::from(pixel[3]),
    ]
  };
  let top_left = texel(x0, y0);
  let top_right = texel(x0 + 1, y0);
  let bottom_left = texel(x0, y0 + 1);
  let bottom_right = texel(x0 + 1, y0 + 1);
  let mut out = [0; 4];

  for channel in 0..4 {
    let top = top_left[channel] + (top_right[channel] - top_left[channel]) * fx;
    let bottom = bottom_left[channel] + (bottom_right[channel] - bottom_left[channel]) * fx;

    out[channel] = (top + (bottom - top) * fy).round().clamp(0.0, 255.0) as u8;
  }

  out
}

/// The layer columns and rows the projected layer spans, or `None` when a corner lands behind
/// the viewer.
fn projected_extent(
  projection: Homography,
  origin: Point<u32>,
  width: usize,
  height: usize,
) -> Option<(Range<usize>, Range<usize>)> {
  let (left, top) = (origin.x as f32, origin.y as f32);
  let [min_x, min_y, max_x, max_y] =
    projection.map_bounds(left, top, left + width as f32, top + height as f32)?;
  let clamp =
    |value: f32, offset: f32, limit: usize| ((value - offset).max(0.0) as usize).min(limit);

  Some((
    clamp(min_x.floor() - 1.0, left, width)..clamp(max_x.ceil() + 1.0, left, width),
    clamp(min_y.floor() - 1.0, top, height)..clamp(max_y.ceil() + 1.0, top, height),
  ))
}
