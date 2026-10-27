//! espaciofuturo fork: the CSS 3D transform functions, composed as a 4x4 and flattened to the
//! `z = 0` plane. Upstream's `Transform` carries them as one variant, [`Transform::ThreeD`].

use std::{
  fmt,
  ops::{Mul, MulAssign},
};

use cssparser::Parser;

use crate::style::{
  Affine, Angle, Animatable, Color, ComputedStyle, FromCss, Length, MakeComputed, ParseResult,
  SizingContext, ToCss, Transform, discrete,
};

/// A CSS 3D transform function.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Transform3d {
  /// Projects the element through a perspective of the given distance
  Perspective(Length),
  /// Rotates an element around the X-axis
  RotateX(Angle),
  /// Rotates an element around the Y-axis
  RotateY(Angle),
  /// Rotates an element around the `(x, y, z)` axis
  Rotate3d(f32, f32, f32, Angle),
  /// Translates an element along the Z-axis
  TranslateZ(Length),
  /// Applies raw 4x4 matrix values
  Matrix3d(Matrix3d),
}

/// Parses the arguments of one transform function into a [`Transform`].
pub(crate) type ParseFunction<'i> = fn(&mut Parser<'i>) -> ParseResult<'i, Transform>;

impl Transform3d {
  /// The parser of the 3D function `name`, or `None` when it is not one. `rotateZ` is the 2D
  /// rotation.
  pub(crate) fn parser<'i>(name: &str) -> Option<ParseFunction<'i>> {
    let parse: ParseFunction<'i> = match name.to_ascii_lowercase().as_str() {
      "rotatez" => |input| Ok(Transform::Rotate(Angle::from_css(input)?)),
      "perspective" => |input| Ok(Self::Perspective(Length::from_css(input)?).into()),
      "rotatex" => |input| Ok(Self::RotateX(Angle::from_css(input)?).into()),
      "rotatey" => |input| Ok(Self::RotateY(Angle::from_css(input)?).into()),
      "rotate3d" => |input| {
        let x = input.expect_number()?;
        input.expect_comma()?;
        let y = input.expect_number()?;
        input.expect_comma()?;
        let z = input.expect_number()?;
        input.expect_comma()?;
        Ok(Self::Rotate3d(x, y, z, Angle::from_css(input)?).into())
      },
      "translatez" => |input| Ok(Self::TranslateZ(Length::from_css(input)?).into()),
      "matrix3d" => |input| {
        let mut values = [0.0; 16];
        for (index, value) in values.iter_mut().enumerate() {
          if index > 0 {
            input.expect_comma()?;
          }
          *value = input.expect_number()?;
        }
        Ok(Self::Matrix3d(Matrix3d(values)).into())
      },
      _ => return None,
    };

    Some(parse)
  }

  /// The function's 4x4 matrix.
  pub fn to_matrix(self, sizing: &SizingContext) -> Matrix3d {
    match self {
      Self::Perspective(distance) => Matrix3d::perspective(distance.to_px(sizing, 0.0)),
      Self::RotateX(angle) => Matrix3d::rotation(1.0, 0.0, 0.0, angle),
      Self::RotateY(angle) => Matrix3d::rotation(0.0, 1.0, 0.0, angle),
      Self::Rotate3d(x, y, z, angle) => Matrix3d::rotation(x, y, z, angle),
      Self::TranslateZ(z) => Matrix3d::translation_z(z.to_px(sizing, 0.0)),
      Self::Matrix3d(matrix) => matrix,
    }
  }

  /// The function alone flattened to the plane, as upstream's 2D path composes it; identity when
  /// it is projective. A list holding a 3D function composes through [`Matrix3d`] instead.
  pub fn to_affine(self, sizing: &SizingContext) -> Affine {
    self
      .to_matrix(sizing)
      .flatten()
      .to_affine()
      .unwrap_or(Affine::IDENTITY)
  }
}

impl From<Transform3d> for Transform {
  fn from(transform: Transform3d) -> Self {
    Transform::ThreeD(transform)
  }
}

impl MakeComputed for Transform3d {
  fn make_computed(&mut self, sizing: &SizingContext) {
    if let Self::Perspective(length) | Self::TranslateZ(length) = self {
      length.make_computed(sizing);
    }
  }
}

impl Animatable for Transform3d {
  fn neutral_value_like(other: &Self) -> Option<Self> {
    Some(match *other {
      Self::RotateX(_) => Self::RotateX(Angle::zero()),
      Self::RotateY(_) => Self::RotateY(Angle::zero()),
      Self::Rotate3d(x, y, z, _) => Self::Rotate3d(x, y, z, Angle::zero()),
      Self::TranslateZ(_) => Self::TranslateZ(Length::zero()),
      Self::Matrix3d(_) => Self::Matrix3d(Matrix3d::IDENTITY),
      Self::Perspective(_) => return None,
    })
  }

  fn interpolate(
    &mut self,
    from: &Self,
    to: &Self,
    progress: f32,
    sizing: &SizingContext,
    current_color: Color,
  ) {
    let angle =
      |from: &Angle, to: &Angle| Angle::interpolated(from, to, progress, sizing, current_color);
    let length = |from: &Length, to: &Length| {
      Animatable::interpolated(from, to, progress, sizing, current_color)
    };

    *self = match (from, to) {
      (Self::RotateX(from), Self::RotateX(to)) => Self::RotateX(angle(from, to)),
      (Self::RotateY(from), Self::RotateY(to)) => Self::RotateY(angle(from, to)),
      (Self::Rotate3d(x, y, z, from), Self::Rotate3d(to_x, to_y, to_z, to))
        if (x, y, z) == (to_x, to_y, to_z) =>
      {
        Self::Rotate3d(*x, *y, *z, angle(from, to))
      }
      (Self::TranslateZ(from), Self::TranslateZ(to)) => Self::TranslateZ(length(from, to)),
      (Self::Perspective(from), Self::Perspective(to)) => Self::Perspective(length(from, to)),
      _ => discrete(from, to, progress),
    };
  }
}

impl ToCss for Transform3d {
  fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
    match self {
      Self::Perspective(distance) => {
        dest.write_str("perspective(")?;
        distance.to_css(dest)?;
      }
      Self::RotateX(angle) => {
        dest.write_str("rotateX(")?;
        angle.to_css(dest)?;
      }
      Self::RotateY(angle) => {
        dest.write_str("rotateY(")?;
        angle.to_css(dest)?;
      }
      Self::Rotate3d(x, y, z, angle) => {
        write!(dest, "rotate3d({x}, {y}, {z}, ")?;
        angle.to_css(dest)?;
      }
      Self::TranslateZ(z) => {
        dest.write_str("translateZ(")?;
        z.to_css(dest)?;
      }
      Self::Matrix3d(Matrix3d(values)) => {
        dest.write_str("matrix3d(")?;
        for (index, value) in values.iter().enumerate() {
          if index > 0 {
            dest.write_str(", ")?;
          }
          write!(dest, "{value}")?;
        }
      }
    }
    dest.write_char(')')
  }
}

impl ComputedStyle {
  /// Whether `transform` lists a 3D function.
  pub fn has_3d_transform(&self) -> bool {
    self.svg_data.transform.as_ref().is_some_and(|transforms| {
      transforms
        .iter()
        .any(|transform| matches!(transform, Transform::ThreeD(_)))
    })
  }

  /// The element's local 4x4 transform when its own 3D functions take it out of the plane, with
  /// `perspective` (its containing block's, in this box's space) applied first.
  pub fn local_matrix3d(
    &self,
    width: f32,
    height: f32,
    sizing: &SizingContext,
    perspective: Option<Matrix3d>,
  ) -> Option<Matrix3d> {
    if !self.has_3d_transform() {
      return None;
    }
    let transforms = self.svg_data.transform.as_ref()?;
    let (origin_x, origin_y) = self
      .svg_data
      .transform_origin
      .to_point(sizing, width, height);
    // Upstream's local transform without `transform`: `T(origin) * translate * rotate * scale *
    // offset * T(-origin)`. A 2D matrix commutes with flattening, so it composes as one.
    let mut flat = self.clone();
    flat.svg_data_mut().transform = None;
    let mut matrix = perspective.unwrap_or(Matrix3d::IDENTITY);

    matrix *= Matrix3d::from(
      flat.local_transform(width, height, sizing) * Affine::translation(origin_x, origin_y),
    );
    matrix *= Matrix3d::from_transforms(transforms.iter(), sizing, width, height);
    matrix *= Affine::translation(-origin_x, -origin_y).into();

    Some(matrix)
  }

  /// [`ComputedStyle::local_transform`] for a list with a 3D function: the 4x4 flattened to the
  /// plane, or identity when the result is projective (SVG, PDF and `measure()` paint it flat).
  pub(crate) fn local_transform_3d(
    &self,
    width: f32,
    height: f32,
    sizing: &SizingContext,
  ) -> Option<Affine> {
    let matrix = self.local_matrix3d(width, height, sizing, None)?;

    Some(matrix.flatten().to_affine().unwrap_or(Affine::IDENTITY))
  }
}

// Column-major, as `matrix3d()` lists its arguments.
/// A 4x4 transform matrix, for the CSS 3D transform functions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix3d(pub [f32; 16]);

impl Default for Matrix3d {
  fn default() -> Self {
    Self::IDENTITY
  }
}

impl Mul<Matrix3d> for Matrix3d {
  type Output = Matrix3d;

  fn mul(self, rhs: Matrix3d) -> Self::Output {
    let mut out = [0.0; 16];

    for column in 0..4 {
      for row in 0..4 {
        out[column * 4 + row] = (0..4).map(|k| self.at(row, k) * rhs.at(k, column)).sum();
      }
    }

    Matrix3d(out)
  }
}

impl MulAssign<Matrix3d> for Matrix3d {
  fn mul_assign(&mut self, rhs: Matrix3d) {
    *self = *self * rhs;
  }
}

impl From<Affine> for Matrix3d {
  fn from(affine: Affine) -> Self {
    let mut matrix = Self::IDENTITY;
    matrix.set(0, 0, affine.a);
    matrix.set(1, 0, affine.b);
    matrix.set(0, 1, affine.c);
    matrix.set(1, 1, affine.d);
    matrix.set(0, 3, affine.x);
    matrix.set(1, 3, affine.y);
    matrix
  }
}

impl Matrix3d {
  /// The identity matrix.
  pub const IDENTITY: Self = Self([
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
  ]);

  /// The element at `row` and `column`.
  #[inline(always)]
  pub fn at(&self, row: usize, column: usize) -> f32 {
    self.0[column * 4 + row]
  }

  fn set(&mut self, row: usize, column: usize, value: f32) {
    self.0[column * 4 + row] = value;
  }

  /// <https://drafts.csswg.org/css-transforms-2/#funcdef-perspective>
  pub fn perspective(distance: f32) -> Self {
    let mut matrix = Self::IDENTITY;
    matrix.set(3, 2, -1.0 / distance.max(1.0));
    matrix
  }

  /// <https://drafts.csswg.org/css-transforms-2/#funcdef-rotate3d>
  pub fn rotation(x: f32, y: f32, z: f32, angle: Angle) -> Self {
    let length = (x * x + y * y + z * z).sqrt();
    if length <= f32::EPSILON {
      return Self::IDENTITY;
    }
    let (x, y, z) = (x / length, y / length, z / length);
    let (sin, cos) = angle.to_radians().sin_cos();
    let t = 1.0 - cos;
    let mut matrix = Self::IDENTITY;

    matrix.set(0, 0, 1.0 - t * (y * y + z * z));
    matrix.set(0, 1, t * x * y - sin * z);
    matrix.set(0, 2, t * x * z + sin * y);
    matrix.set(1, 0, t * x * y + sin * z);
    matrix.set(1, 1, 1.0 - t * (x * x + z * z));
    matrix.set(1, 2, t * y * z - sin * x);
    matrix.set(2, 0, t * x * z - sin * y);
    matrix.set(2, 1, t * y * z + sin * x);
    matrix.set(2, 2, 1.0 - t * (x * x + y * y));
    matrix
  }

  /// A translation along the Z-axis.
  pub fn translation_z(z: f32) -> Self {
    let mut matrix = Self::IDENTITY;
    matrix.set(2, 3, z);
    matrix
  }

  /// The determinant.
  pub fn determinant(&self) -> f32 {
    let m = |row: usize, column: usize| self.at(row, column);
    let s0 = m(0, 0) * m(1, 1) - m(1, 0) * m(0, 1);
    let s1 = m(0, 0) * m(1, 2) - m(1, 0) * m(0, 2);
    let s2 = m(0, 0) * m(1, 3) - m(1, 0) * m(0, 3);
    let s3 = m(0, 1) * m(1, 2) - m(1, 1) * m(0, 2);
    let s4 = m(0, 1) * m(1, 3) - m(1, 1) * m(0, 3);
    let s5 = m(0, 2) * m(1, 3) - m(1, 2) * m(0, 3);
    let c5 = m(2, 2) * m(3, 3) - m(3, 2) * m(2, 3);
    let c4 = m(2, 1) * m(3, 3) - m(3, 1) * m(2, 3);
    let c3 = m(2, 1) * m(3, 2) - m(3, 1) * m(2, 2);
    let c2 = m(2, 0) * m(3, 3) - m(3, 0) * m(2, 3);
    let c1 = m(2, 0) * m(3, 2) - m(3, 0) * m(2, 2);
    let c0 = m(2, 0) * m(3, 1) - m(3, 0) * m(2, 1);

    s0 * c5 - s1 * c4 + s2 * c3 + s3 * c2 - s4 * c1 + s5 * c0
  }

  /// Whether the element's back faces the viewer, as Blink's `IsBackFaceVisible` decides: the
  /// inverse maps the screen normal to a negative z.
  pub fn is_back_facing(&self) -> bool {
    let determinant = self.determinant();
    if determinant.abs() <= f32::EPSILON {
      return false;
    }

    self.flatten().determinant() / determinant < 0.0
  }

  /// The 3x3 homography this matrix applies to the `z = 0` plane, row-major.
  pub fn flatten(&self) -> Homography {
    Homography([
      self.at(0, 0),
      self.at(0, 1),
      self.at(0, 3),
      self.at(1, 0),
      self.at(1, 1),
      self.at(1, 3),
      self.at(3, 0),
      self.at(3, 1),
      self.at(3, 3),
    ])
  }

  /// Composes the CSS transform functions, 2D and 3D, left to right.
  pub(crate) fn from_transforms<'a, I: Iterator<Item = &'a Transform>>(
    transforms: I,
    sizing: &SizingContext,
    width: f32,
    height: f32,
  ) -> Matrix3d {
    let mut instance = Matrix3d::IDENTITY;

    for transform in transforms {
      instance *= match *transform {
        Transform::ThreeD(transform) => transform.to_matrix(sizing),
        two_d => Affine::from_transforms([two_d].iter(), sizing, width, height).into(),
      };
    }

    instance
  }
}

/// A row-major 3x3 projective map of the plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Homography(pub [f32; 9]);

impl From<Affine> for Homography {
  fn from(affine: Affine) -> Self {
    Homography([
      affine.a, affine.c, affine.x, affine.b, affine.d, affine.y, 0.0, 0.0, 1.0,
    ])
  }
}

impl Mul<Homography> for Homography {
  type Output = Homography;

  fn mul(self, rhs: Homography) -> Self::Output {
    let (a, b) = (self.0, rhs.0);
    let mut out = [0.0; 9];

    for row in 0..3 {
      for column in 0..3 {
        out[row * 3 + column] = (0..3).map(|k| a[row * 3 + k] * b[k * 3 + column]).sum();
      }
    }

    Homography(out)
  }
}

impl Homography {
  /// The affine map this homography equals, or `None` when it is projective.
  pub fn to_affine(self) -> Option<Affine> {
    let [a, c, x, b, d, y, p, q, w] = self.0;
    if p.abs() > 1e-7 || q.abs() > 1e-7 || w.abs() <= f32::EPSILON {
      return None;
    }

    Some(Affine {
      a: a / w,
      b: b / w,
      c: c / w,
      d: d / w,
      x: x / w,
      y: y / w,
    })
  }

  /// Maps `(x, y)`, or `None` when the point lands behind the viewer.
  #[inline(always)]
  pub fn map_point(&self, x: f32, y: f32) -> Option<(f32, f32)> {
    let m = &self.0;
    let w = m[6] * x + m[7] * y + m[8];
    if w <= f32::EPSILON {
      return None;
    }

    Some((
      (m[0] * x + m[1] * y + m[2]) / w,
      (m[3] * x + m[4] * y + m[5]) / w,
    ))
  }

  /// The determinant.
  pub fn determinant(&self) -> f32 {
    let [a, b, c, d, e, f, g, h, i] = self.0;

    a * (e * i - f * h) + b * (f * g - d * i) + c * (d * h - e * g)
  }

  /// The axis-aligned bounds of the rectangle `(left, top, right, bottom)` mapped through this
  /// homography, or `None` when a corner lands behind the viewer.
  pub fn map_bounds(&self, left: f32, top: f32, right: f32, bottom: f32) -> Option<[f32; 4]> {
    let mut out = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];

    for (x, y) in [(left, top), (right, top), (left, bottom), (right, bottom)] {
      let (x, y) = self.map_point(x, y)?;
      out = [out[0].min(x), out[1].min(y), out[2].max(x), out[3].max(y)];
    }

    Some(out)
  }

  /// The inverse map, or `None` when it is singular.
  pub fn invert(self) -> Option<Self> {
    let [a, b, c, d, e, f, g, h, i] = self.0;
    let co_a = e * i - f * h;
    let co_b = f * g - d * i;
    let co_c = d * h - e * g;
    let det = a * co_a + b * co_b + c * co_c;
    if det.abs() <= f32::EPSILON * f32::EPSILON {
      return None;
    }
    let inv = 1.0 / det;

    Some(Homography([
      co_a * inv,
      (c * h - b * i) * inv,
      (b * f - c * e) * inv,
      co_b * inv,
      (a * i - c * g) * inv,
      (c * d - a * f) * inv,
      co_c * inv,
      (b * g - a * h) * inv,
      (a * e - b * d) * inv,
    ]))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::style::FromCssStr;

  #[test]
  fn transform_3d_functions_parse() {
    for (css, expected) in [
      (
        "perspective(800px)",
        Transform3d::Perspective(Length::Px(800.0)).into(),
      ),
      (
        "rotateX(30deg)",
        Transform3d::RotateX(Angle::new(30.0)).into(),
      ),
      (
        "rotateY(30deg)",
        Transform3d::RotateY(Angle::new(30.0)).into(),
      ),
      ("rotateZ(30deg)", Transform::Rotate(Angle::new(30.0))),
      (
        "rotate3d(0, 1, 0, 30deg)",
        Transform3d::Rotate3d(0.0, 1.0, 0.0, Angle::new(30.0)).into(),
      ),
      (
        "translateZ(10px)",
        Transform3d::TranslateZ(Length::Px(10.0)).into(),
      ),
      (
        "matrix3d(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)",
        Transform3d::Matrix3d(Matrix3d::IDENTITY).into(),
      ),
    ] {
      assert_eq!(Transform::from_css_str(css), Ok(expected), "{css}");
    }
  }

  #[test]
  fn transform_3d_functions_serialize() {
    for css in [
      "perspective(800px)",
      "rotateX(30deg)",
      "rotate3d(0, 1, 0, 30deg)",
      "translateZ(10px)",
    ] {
      let transform = Transform::from_css_str(css).unwrap();

      assert_eq!(transform.to_css_string(), css);
    }
  }

  #[test]
  fn rotate_x_without_perspective_flattens_to_an_affine_squash() {
    let affine = Matrix3d::rotation(1.0, 0.0, 0.0, Angle::new(60.0))
      .flatten()
      .to_affine();

    assert!(
      affine.is_some_and(|affine| (affine.d - 0.5).abs() < 1e-5 && (affine.a - 1.0).abs() < 1e-5)
    );
  }

  #[test]
  fn perspective_rotate_y_is_projective_and_shrinks_the_far_edge() {
    let homography = (Matrix3d::perspective(800.0)
      * Matrix3d::rotation(0.0, 1.0, 0.0, Angle::new(30.0)))
    .flatten();

    assert!(homography.to_affine().is_none());
    let near = homography.map_point(-150.0, 150.0).unwrap().1
      - homography.map_point(-150.0, -150.0).unwrap().1;
    let far = homography.map_point(150.0, 150.0).unwrap().1
      - homography.map_point(150.0, -150.0).unwrap().1;
    assert!(far < near, "{far} < {near}");
    let inverse = homography.invert().unwrap();
    let (projected_x, projected_y) = homography.map_point(40.0, -70.0).unwrap();
    let (x, y) = inverse.map_point(projected_x, projected_y).unwrap();
    assert!((x - 40.0).abs() < 1e-3 && (y + 70.0).abs() < 1e-3);
  }

  #[test]
  fn back_facing_follows_the_rotation_past_ninety_degrees() {
    let rotate_y = |degrees| {
      Matrix3d::perspective(600.0) * Matrix3d::rotation(0.0, 1.0, 0.0, Angle::new(degrees))
    };

    assert!(!rotate_y(60.0).is_back_facing());
    assert!(rotate_y(120.0).is_back_facing());
    assert!(rotate_y(240.0).is_back_facing());
    assert!(!Matrix3d::from(Affine::scale(-1.0, 1.0)).is_back_facing());
    assert!((Matrix3d::from(Affine::scale(2.0, 3.0)).determinant() - 6.0).abs() < 1e-5);
  }
}
