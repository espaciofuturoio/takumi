use std::ops::Range;

use tiny_skia::Pixmap;

use crate::{geometry::Point, scene::SceneBounds, style::Homography};

/// Redraws `layer`, whose top-left pixel sits at device `origin`, through the device-space
/// `projection`, sampling the flat paint bilinearly as Blink composites a 3D-transformed layer.
/// Only the pixels the projected `source` bounds can reach are resampled, when they are known.
pub(crate) fn project_layer(
  layer: &mut Pixmap,
  origin: Point<u32>,
  projection: Homography,
  source: Option<SceneBounds>,
) {
  let Some(inverse) = projection.invert() else {
    layer.fill(tiny_skia::Color::TRANSPARENT);
    return;
  };
  let width = layer.width() as usize;
  let height = layer.height() as usize;
  let (columns, rows) = source
    .and_then(|bounds| projected_extent(bounds, projection, origin, width, height))
    .unwrap_or((0..width, 0..height));
  let source = layer.data().to_vec();
  let target = layer.data_mut();

  target.fill(0);

  for y in rows {
    let device_y = origin.y as f32 + y as f32 + 0.5;

    for x in columns.clone() {
      let device_x = origin.x as f32 + x as f32 + 0.5;
      let pixel = inverse
        .map_point(device_x, device_y)
        .map(|(source_x, source_y)| {
          sample_bilinear(
            &source,
            width,
            height,
            source_x - origin.x as f32 - 0.5,
            source_y - origin.y as f32 - 0.5,
          )
        })
        .unwrap_or([0; 4]);
      let offset = (y * width + x) * 4;

      target[offset..offset + 4].copy_from_slice(&pixel);
    }
  }
}

fn sample_bilinear(source: &[u8], width: usize, height: usize, x: f32, y: f32) -> [u8; 4] {
  let x0 = x.floor();
  let y0 = y.floor();
  let fx = x - x0;
  let fy = y - y0;
  let texel = |tx: f32, ty: f32| -> [f32; 4] {
    if tx < 0.0 || ty < 0.0 || tx >= width as f32 || ty >= height as f32 {
      return [0.0; 4];
    }
    let offset = (ty as usize * width + tx as usize) * 4;

    [
      f32::from(source[offset]),
      f32::from(source[offset + 1]),
      f32::from(source[offset + 2]),
      f32::from(source[offset + 3]),
    ]
  };
  let top_left = texel(x0, y0);
  let top_right = texel(x0 + 1.0, y0);
  let bottom_left = texel(x0, y0 + 1.0);
  let bottom_right = texel(x0 + 1.0, y0 + 1.0);
  let mut out = [0; 4];

  for channel in 0..4 {
    let top = top_left[channel] + (top_right[channel] - top_left[channel]) * fx;
    let bottom = bottom_left[channel] + (bottom_right[channel] - bottom_left[channel]) * fx;

    out[channel] = (top + (bottom - top) * fy).round().clamp(0.0, 255.0) as u8;
  }

  out
}

/// The layer columns and rows the four projected corners of `bounds` span, or `None` when a
/// corner lands behind the viewer.
fn projected_extent(
  bounds: SceneBounds,
  projection: Homography,
  origin: Point<u32>,
  width: usize,
  height: usize,
) -> Option<(Range<usize>, Range<usize>)> {
  let corners = [
    (bounds.left, bounds.top),
    (bounds.right, bounds.top),
    (bounds.left, bounds.bottom),
    (bounds.right, bounds.bottom),
  ]
  .map(|(x, y)| projection.map_point(x as f32, y as f32));
  let (mut left, mut top, mut right, mut bottom) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);

  for corner in corners {
    let (x, y) = corner?;
    left = left.min(x);
    top = top.min(y);
    right = right.max(x);
    bottom = bottom.max(y);
  }
  let clamp =
    |value: f32, offset: u32, limit: usize| ((value - offset as f32).max(0.0) as usize).min(limit);

  Some((
    clamp(left.floor() - 1.0, origin.x, width)..clamp(right.ceil() + 1.0, origin.x, width),
    clamp(top.floor() - 1.0, origin.y, height)..clamp(bottom.ceil() + 1.0, origin.y, height),
  ))
}
