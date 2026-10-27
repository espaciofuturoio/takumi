//! espaciofuturo fork: the 3D fixtures the SVG backend paints flat, joined to `svg_parity`'s
//! `KNOWN_DIVERGENT`.

pub(crate) const KNOWN_DIVERGENT: &[(&str, &str)] = &[
  (
    "style_perspective_cube",
    "svg paints a projective 3D transform flat; only raster resamples the layer \
     through the homography (83.6%, floor is 90%)",
  ),
  (
    "style_transform_3d_card",
    "svg paints a projective 3D transform flat; only raster resamples the layer \
     through the homography (87.6%, floor is 90%)",
  ),
];
