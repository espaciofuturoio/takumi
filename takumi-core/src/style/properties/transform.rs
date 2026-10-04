use std::{
  fmt,
  ops::{Deref, DerefMut, Mul, MulAssign},
};

use cssparser::{Parser, Token, match_ignore_ascii_case};
use tiny_skia::Transform as TinyTransform;

use crate::style::{
  Angle, Animatable, Color, FromCss, Length, ListInterpolationStrategy, MakeComputed, ParseResult,
  PercentageNumber, SizingContext, ToCss, discrete, lerp, unexpected_token,
};

const DEFAULT_SCALE: f32 = 1.0;

/// Represents a single CSS transform operation
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Transform {
  /// Translates an element along the X-axis and Y-axis by the specified lengths
  Translate(Length, Length),
  /// Scales an element by the specified factors
  Scale(f32, f32),
  /// Rotates an element (2D rotation) by angle in degrees
  Rotate(Angle),
  /// Skews an element by the specified angles
  Skew(Angle, Angle),
  /// Applies raw affine matrix values
  Matrix(Affine),
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

impl Transform {
  /// Whether this function moves the element out of the 2D plane.
  pub fn is_3d(self) -> bool {
    matches!(
      self,
      Transform::Perspective(_)
        | Transform::RotateX(_)
        | Transform::RotateY(_)
        | Transform::Rotate3d(..)
        | Transform::TranslateZ(_)
        | Transform::Matrix3d(_)
    )
  }
}

impl MakeComputed for Transform {
  fn make_computed(&mut self, sizing: &SizingContext) {
    match self {
      Transform::Translate(x, y) => {
        x.make_computed(sizing);
        y.make_computed(sizing);
      }
      Transform::Perspective(length) | Transform::TranslateZ(length) => {
        length.make_computed(sizing);
      }
      _ => {}
    }
  }
}

impl Animatable for Transform {
  fn list_interpolation_strategy() -> ListInterpolationStrategy {
    ListInterpolationStrategy::PadToLongestWithNeutral
  }

  fn neutral_value_like(other: &Self) -> Option<Self> {
    Some(match *other {
      Transform::Translate(_, _) => Transform::Translate(Length::zero(), Length::zero()),
      Transform::Scale(_, _) => Transform::Scale(1.0, 1.0),
      Transform::Rotate(_) => Transform::Rotate(Angle::zero()),
      Transform::Skew(_, _) => Transform::Skew(Angle::zero(), Angle::zero()),
      Transform::Matrix(_) => Transform::Matrix(Affine::IDENTITY),
      Transform::RotateX(_) => Transform::RotateX(Angle::zero()),
      Transform::RotateY(_) => Transform::RotateY(Angle::zero()),
      Transform::Rotate3d(x, y, z, _) => Transform::Rotate3d(x, y, z, Angle::zero()),
      Transform::TranslateZ(_) => Transform::TranslateZ(Length::zero()),
      Transform::Matrix3d(_) => Transform::Matrix3d(Matrix3d::IDENTITY),
      Transform::Perspective(_) => return None,
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
    *self = match (*from, *to) {
      (Transform::Translate(from_x, from_y), Transform::Translate(to_x, to_y)) => {
        Transform::Translate(
          Animatable::interpolated(&from_x, &to_x, progress, sizing, current_color),
          Animatable::interpolated(&from_y, &to_y, progress, sizing, current_color),
        )
      }
      (Transform::Scale(from_x, from_y), Transform::Scale(to_x, to_y)) => {
        Transform::Scale(lerp(from_x, to_x, progress), lerp(from_y, to_y, progress))
      }
      (Transform::Rotate(from_angle), Transform::Rotate(to_angle)) => Transform::Rotate(
        Angle::interpolated(&from_angle, &to_angle, progress, sizing, current_color),
      ),
      (Transform::Skew(from_x, from_y), Transform::Skew(to_x, to_y)) => Transform::Skew(
        Animatable::interpolated(&from_x, &to_x, progress, sizing, current_color),
        Animatable::interpolated(&from_y, &to_y, progress, sizing, current_color),
      ),
      (Transform::Matrix(from_affine), Transform::Matrix(to_affine)) => Transform::Matrix(Affine {
        a: lerp(from_affine.a, to_affine.a, progress),
        b: lerp(from_affine.b, to_affine.b, progress),
        c: lerp(from_affine.c, to_affine.c, progress),
        d: lerp(from_affine.d, to_affine.d, progress),
        x: lerp(from_affine.x, to_affine.x, progress),
        y: lerp(from_affine.y, to_affine.y, progress),
      }),
      (Transform::RotateX(from_angle), Transform::RotateX(to_angle)) => Transform::RotateX(
        Angle::interpolated(&from_angle, &to_angle, progress, sizing, current_color),
      ),
      (Transform::RotateY(from_angle), Transform::RotateY(to_angle)) => Transform::RotateY(
        Angle::interpolated(&from_angle, &to_angle, progress, sizing, current_color),
      ),
      (
        Transform::Rotate3d(x, y, z, from_angle),
        Transform::Rotate3d(to_x, to_y, to_z, to_angle),
      ) if (x, y, z) == (to_x, to_y, to_z) => Transform::Rotate3d(
        x,
        y,
        z,
        Angle::interpolated(&from_angle, &to_angle, progress, sizing, current_color),
      ),
      (Transform::TranslateZ(from_z), Transform::TranslateZ(to_z)) => Transform::TranslateZ(
        Animatable::interpolated(&from_z, &to_z, progress, sizing, current_color),
      ),
      (Transform::Perspective(from_d), Transform::Perspective(to_d)) => Transform::Perspective(
        Animatable::interpolated(&from_d, &to_d, progress, sizing, current_color),
      ),
      _ => discrete(from, to, progress),
    };
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
        Transform::Perspective(distance) => Matrix3d::perspective(distance.to_px(sizing, 0.0)),
        Transform::RotateX(angle) => Matrix3d::rotation(1.0, 0.0, 0.0, angle),
        Transform::RotateY(angle) => Matrix3d::rotation(0.0, 1.0, 0.0, angle),
        Transform::Rotate3d(x, y, z, angle) => Matrix3d::rotation(x, y, z, angle),
        Transform::TranslateZ(z) => Matrix3d::translation_z(z.to_px(sizing, 0.0)),
        Transform::Matrix3d(matrix) => matrix,
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

// | a c x |
// | b d y |
// | 0 0 1 |
/// An affine transform matrix, representing a combination of translation, rotation, scaling, and skewing.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Affine {
  /// Horizontal scaling / cosine of rotation
  pub a: f32,
  /// Horizontal shear / sine of rotation
  pub b: f32,
  /// Vertical shear / negative sine of rotation
  pub c: f32,
  /// Vertical scaling / cosine of rotation
  pub d: f32,
  /// Horizontal translation (always orthogonal regardless of rotation)
  pub x: f32,
  /// Vertical translation (always orthogonal regardless of rotation)
  pub y: f32,
}

impl From<Affine> for TinyTransform {
  fn from(transform: Affine) -> Self {
    TinyTransform::from_row(
      transform.a,
      transform.b,
      transform.c,
      transform.d,
      transform.x,
      transform.y,
    )
  }
}

impl Mul<Affine> for Affine {
  type Output = Affine;

  fn mul(self, rhs: Affine) -> Self::Output {
    if self.is_identity() {
      return rhs;
    }

    if rhs.is_identity() {
      return self;
    }

    Affine {
      a: self.a * rhs.a + self.c * rhs.b,
      b: self.b * rhs.a + self.d * rhs.b,
      c: self.a * rhs.c + self.c * rhs.d,
      d: self.b * rhs.c + self.d * rhs.d,
      x: self.a * rhs.x + self.c * rhs.y + self.x,
      y: self.b * rhs.x + self.d * rhs.y + self.y,
    }
  }
}

impl MulAssign<Affine> for Affine {
  fn mul_assign(&mut self, rhs: Affine) {
    *self = *self * rhs;
  }
}

impl Affine {
  /// Converts the affine transform to a column-major array.
  pub fn to_cols_array(&self) -> [f32; 6] {
    [self.a, self.b, self.c, self.d, self.x, self.y]
  }

  /// Returns the identity transform
  pub const IDENTITY: Self = Self {
    a: 1.0,
    b: 0.0,
    c: 0.0,
    d: 1.0,
    x: 0.0,
    y: 0.0,
  };

  /// Returns true if the transform is the identity transform
  pub fn is_identity(self) -> bool {
    (self.a - 1.0).abs() < 1e-6
      && self.b.abs() < 1e-6
      && self.c.abs() < 1e-6
      && (self.d - 1.0).abs() < 1e-6
      && self.x.abs() < 1e-6
      && self.y.abs() < 1e-6
  }

  /// Geometric mean of the X and Y axis scales.
  pub fn uniform_scale(self) -> f32 {
    let sx = (self.a * self.a + self.b * self.b).sqrt();
    let sy = (self.c * self.c + self.d * self.d).sqrt();
    (sx * sy).sqrt()
  }

  /// Returns true if the transform is only a translation
  pub fn only_translation(self) -> bool {
    (self.a - 1.0).abs() < 1e-8
      && self.b.abs() < 1e-8
      && self.c.abs() < 1e-8
      && (self.d - 1.0).abs() < 1e-8
  }

  /// Creates a new rotation transform
  pub fn rotation(angle: Angle) -> Self {
    Self::rotation_radians(angle.to_radians())
  }

  /// Creates a new rotation transform from an angle in radians
  pub(crate) fn rotation_radians(radians: f32) -> Self {
    let (sin, cos) = radians.sin_cos();

    Self {
      a: cos,
      b: sin,
      c: -sin,
      d: cos,
      x: 0.0,
      y: 0.0,
    }
  }

  /// Creates a new translation transform
  pub const fn translation(x: f32, y: f32) -> Self {
    Self {
      x,
      y,
      ..Self::IDENTITY
    }
  }

  /// Creates a new scale transform
  pub const fn scale(x: f32, y: f32) -> Self {
    Self {
      a: x,
      b: 0.0,
      c: 0.0,
      d: y,
      x: 0.0,
      y: 0.0,
    }
  }

  /// Transforms a point by the transform
  #[inline(always)]
  pub fn transform_point(self, x: f32, y: f32) -> (f32, f32) {
    // Fast path: If the transform is only a translation, we can just add the translation to the point
    if self.only_translation() {
      return (x + self.x, y + self.y);
    }

    (
      self.a * x + self.c * y + self.x,
      self.b * x + self.d * y + self.y,
    )
  }

  /// Creates a new skew transform
  pub(crate) fn skew(x: Angle, y: Angle) -> Self {
    let tanx = x.to_radians().tan();
    let tany = y.to_radians().tan();

    Self {
      a: 1.0,
      b: tany,
      c: tanx,
      d: 1.0,
      x: 0.0,
      y: 0.0,
    }
  }

  /// Calculates the determinant of the transform
  #[inline(always)]
  pub(crate) fn determinant(self) -> f32 {
    self.a * self.d - self.b * self.c
  }

  /// Returns true if the transform is invertible
  #[inline(always)]
  pub fn is_invertible(self) -> bool {
    self.determinant().abs() > f32::EPSILON
  }

  /// Inverts the transform, returns `None` if the transform is not invertible
  pub fn invert(self) -> Option<Self> {
    let det = self.determinant();
    if det.abs() < f32::EPSILON {
      return None;
    }

    let inv_det = 1.0 / det;

    Some(Self {
      a: self.d * inv_det,
      b: self.b * -inv_det,
      c: self.c * -inv_det,
      d: self.a * inv_det,
      x: (self.d * self.x - self.c * self.y) * -inv_det,
      y: (self.b * self.x - self.a * self.y) * inv_det,
    })
  }

  /// Converts the transforms to a [`Affine`] instance
  ///
  /// CSS transform property applies transformations from left to right.
  /// For `transform: translate() rotate()`, the resulting matrix is translate * rotate.
  /// When applied to point p: translate * rotate * p, rotate is applied first.
  pub(crate) fn from_transforms<'a, I: Iterator<Item = &'a Transform>>(
    transforms: I,
    sizing: &SizingContext,
    width: f32,
    height: f32,
  ) -> Affine {
    let mut instance = Affine::IDENTITY;

    for transform in transforms {
      instance *= match *transform {
        Transform::Translate(x_length, y_length) => Affine::translation(
          x_length.to_px(sizing, width),
          y_length.to_px(sizing, height),
        ),
        Transform::Scale(x_scale, y_scale) => Affine::scale(x_scale, y_scale),
        Transform::Rotate(angle) => Affine::rotation(angle),
        Transform::Skew(x_angle, y_angle) => Affine::skew(x_angle, y_angle),
        Transform::Matrix(affine) => affine,
        Transform::Perspective(_)
        | Transform::RotateX(_)
        | Transform::RotateY(_)
        | Transform::Rotate3d(..)
        | Transform::TranslateZ(_)
        | Transform::Matrix3d(_) => Affine::IDENTITY,
      };
    }

    instance
  }
}

impl<'i> FromCss<'i> for Affine {
  fn from_css(input: &mut Parser<'i, '_>) -> ParseResult<'i, Self> {
    let a = input.expect_number()?;
    input.expect_comma()?;
    let b = input.expect_number()?;
    input.expect_comma()?;
    let c = input.expect_number()?;
    input.expect_comma()?;
    let d = input.expect_number()?;
    input.expect_comma()?;
    let x = input.expect_number()?;
    input.expect_comma()?;
    let y = input.expect_number()?;

    Ok(Affine { a, b, c, d, x, y })
  }

  const VALID_TOKENS: &'static [&'static str] = &["<number>"];
}

/// A collection of transform operations that can be applied together
#[derive(Debug, Clone, PartialEq)]
pub struct Transforms(pub Box<[Transform]>);

impl<'i> FromCss<'i> for Transforms {
  fn from_css(input: &mut Parser<'i, '_>) -> ParseResult<'i, Self> {
    let mut transforms = Vec::new();

    while !input.is_exhausted() {
      let transform = Transform::from_css(input)?;
      transforms.push(transform);
    }

    Ok(Transforms(transforms.into_boxed_slice()))
  }

  const VALID_TOKENS: &'static [&'static str] = Transform::VALID_TOKENS;
}

impl MakeComputed for Transforms {
  fn make_computed(&mut self, sizing: &SizingContext) {
    self.0.make_computed(sizing);
  }
}

impl Animatable for Transforms {
  fn missing_value() -> Option<Self> {
    <Box<[Transform]>>::missing_value().map(Self)
  }

  fn interpolate(
    &mut self,
    from: &Self,
    to: &Self,
    progress: f32,
    sizing: &SizingContext,
    current_color: Color,
  ) {
    self
      .0
      .interpolate(&from.0, &to.0, progress, sizing, current_color);
  }
}

impl Deref for Transforms {
  type Target = Box<[Transform]>;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl DerefMut for Transforms {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl From<Box<[Transform]>> for Transforms {
  fn from(box_slice: Box<[Transform]>) -> Self {
    Self(box_slice)
  }
}

impl From<Vec<Transform>> for Transforms {
  fn from(vec: Vec<Transform>) -> Self {
    Self(vec.into_boxed_slice())
  }
}

impl<const N: usize> From<[Transform; N]> for Transforms {
  fn from(arr: [Transform; N]) -> Self {
    Self(Box::from(arr))
  }
}

impl ToCss for Transform {
  const LIST_SEPARATOR: &'static str = " ";

  fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
    match self {
      Self::Translate(x, y) => {
        dest.write_str("translate(")?;
        x.to_css(dest)?;
        dest.write_str(", ")?;
        y.to_css(dest)?;
        dest.write_char(')')
      }
      Self::Scale(x, y) => write!(dest, "scale({x}, {y})"),
      Self::Rotate(a) => {
        dest.write_str("rotate(")?;
        a.to_css(dest)?;
        dest.write_char(')')
      }
      Self::Skew(x, y) => {
        dest.write_str("skew(")?;
        x.to_css(dest)?;
        dest.write_str(", ")?;
        y.to_css(dest)?;
        dest.write_char(')')
      }
      Self::Matrix(affine) => affine.to_css(dest),
      Self::Perspective(distance) => {
        dest.write_str("perspective(")?;
        distance.to_css(dest)?;
        dest.write_char(')')
      }
      Self::RotateX(a) => {
        dest.write_str("rotateX(")?;
        a.to_css(dest)?;
        dest.write_char(')')
      }
      Self::RotateY(a) => {
        dest.write_str("rotateY(")?;
        a.to_css(dest)?;
        dest.write_char(')')
      }
      Self::Rotate3d(x, y, z, a) => {
        write!(dest, "rotate3d({x}, {y}, {z}, ")?;
        a.to_css(dest)?;
        dest.write_char(')')
      }
      Self::TranslateZ(z) => {
        dest.write_str("translateZ(")?;
        z.to_css(dest)?;
        dest.write_char(')')
      }
      Self::Matrix3d(Matrix3d(values)) => {
        dest.write_str("matrix3d(")?;
        for (index, value) in values.iter().enumerate() {
          if index > 0 {
            dest.write_str(", ")?;
          }
          write!(dest, "{value}")?;
        }
        dest.write_char(')')
      }
    }
  }
}

impl ToCss for Affine {
  fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
    let Self { a, b, c, d, x, y } = self;
    write!(dest, "matrix({a}, {b}, {c}, {d}, {x}, {y})")
  }
}

impl ToCss for Transforms {
  fn to_css<W: fmt::Write>(&self, dest: &mut W) -> fmt::Result {
    self.0.to_css(dest)
  }
}

impl<'i> FromCss<'i> for Transform {
  fn from_css(parser: &mut Parser<'i, '_>) -> ParseResult<'i, Self> {
    let location = parser.current_source_location();
    let token = parser.next()?;

    let Token::Function(function) = token else {
      return Err(
        location
          .new_basic_unexpected_token_error(token.clone())
          .into(),
      );
    };

    let parse: fn(&mut Parser<'i, '_>) -> ParseResult<'i, Self> = match_ignore_ascii_case! {function,
      "translate" => |input| {
        let x = Length::from_css(input)?;
        input.expect_comma()?;
        let y = Length::from_css(input)?;

        Ok(Self::Translate(x, y))
      },
      "translatex" => |input| Ok(Self::Translate(
        Length::from_css(input)?,
        Length::zero(),
      )),
      "translatey" => |input| Ok(Self::Translate(
        Length::zero(),
        Length::from_css(input)?,
      )),
      "scale" => |input| {
        let PercentageNumber(x) = PercentageNumber::from_css(input)?;
        if input.try_parse(Parser::expect_comma).is_ok() {
          let PercentageNumber(y) = PercentageNumber::from_css(input)?;
          Ok(Self::Scale(x, y))
        } else {
          Ok(Self::Scale(x, x))
        }
      },
      "scalex" => |input| Ok(Self::Scale(
        PercentageNumber::from_css(input)?.0,
        DEFAULT_SCALE,
      )),
      "scaley" => |input| Ok(Self::Scale(
        DEFAULT_SCALE,
        PercentageNumber::from_css(input)?.0,
      )),
      "skew" => |input| {
        let x = Angle::from_css(input)?;
        input.expect_comma()?;
        let y = Angle::from_css(input)?;

        Ok(Self::Skew(x, y))
      },
      "skewx" => |input| Ok(Self::Skew(
        Angle::from_css(input)?,
        Angle::default(),
      )),
      "skewy" => |input| Ok(Self::Skew(
        Angle::default(),
        Angle::from_css(input)?,
      )),
      "rotate" | "rotatez" => |input| Ok(Self::Rotate(Angle::from_css(input)?)),
      "matrix" => |input| Ok(Self::Matrix(Affine::from_css(input)?)),
      "perspective" => |input| Ok(Self::Perspective(Length::from_css(input)?)),
      "rotatex" => |input| Ok(Self::RotateX(Angle::from_css(input)?)),
      "rotatey" => |input| Ok(Self::RotateY(Angle::from_css(input)?)),
      "rotate3d" => |input| {
        let x = input.expect_number()?;
        input.expect_comma()?;
        let y = input.expect_number()?;
        input.expect_comma()?;
        let z = input.expect_number()?;
        input.expect_comma()?;
        Ok(Self::Rotate3d(x, y, z, Angle::from_css(input)?))
      },
      "translatez" => |input| Ok(Self::TranslateZ(Length::from_css(input)?)),
      "matrix3d" => |input| {
        let mut values = [0.0; 16];
        for (index, value) in values.iter_mut().enumerate() {
          if index > 0 {
            input.expect_comma()?;
          }
          *value = input.expect_number()?;
        }
        Ok(Self::Matrix3d(Matrix3d(values)))
      },
      _ => return Err(unexpected_token!(location, token)),
    };

    parser.parse_nested_block(parse)
  }

  const VALID_TOKENS: &'static [&'static str] = &["<transform-function>"];
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::style::FromCssStr;

  #[test]
  fn test_transform_from_str() {
    assert_eq!(
      Transform::from_css_str("translate(10, 20px)"),
      Ok(Transform::Translate(Length::Px(10.0), Length::Px(20.0)))
    );
  }

  #[test]
  fn test_transform_scale_from_str() {
    assert_eq!(
      Transform::from_css_str("scale(10)"),
      Ok(Transform::Scale(10.0, 10.0))
    );
  }

  #[test]
  fn transform_functions_keep_their_individual_grammars() {
    for (css, expected) in [
      (
        "translate(1px, 2px)",
        Transform::Translate(Length::Px(1.0), Length::Px(2.0)),
      ),
      (
        "translateX(1px)",
        Transform::Translate(Length::Px(1.0), Length::zero()),
      ),
      (
        "translateY(2px)",
        Transform::Translate(Length::zero(), Length::Px(2.0)),
      ),
      ("scale(2, 3)", Transform::Scale(2.0, 3.0)),
      ("scaleX(2)", Transform::Scale(2.0, DEFAULT_SCALE)),
      ("scaleY(3)", Transform::Scale(DEFAULT_SCALE, 3.0)),
      (
        "skew(1deg, 2deg)",
        Transform::Skew(Angle::new(1.0), Angle::new(2.0)),
      ),
      (
        "skewX(1deg)",
        Transform::Skew(Angle::new(1.0), Angle::default()),
      ),
      (
        "skewY(2deg)",
        Transform::Skew(Angle::default(), Angle::new(2.0)),
      ),
      ("rotate(3deg)", Transform::Rotate(Angle::new(3.0))),
      (
        "matrix(1, 0, 0, 1, 2, 3)",
        Transform::Matrix(Affine::translation(2.0, 3.0)),
      ),
    ] {
      assert_eq!(Transform::from_css_str(css), Ok(expected), "{css}");
    }
  }

  #[test]
  fn transform_3d_functions_parse() {
    for (css, expected) in [
      (
        "perspective(800px)",
        Transform::Perspective(Length::Px(800.0)),
      ),
      ("rotateX(30deg)", Transform::RotateX(Angle::new(30.0))),
      ("rotateY(30deg)", Transform::RotateY(Angle::new(30.0))),
      ("rotateZ(30deg)", Transform::Rotate(Angle::new(30.0))),
      (
        "rotate3d(0, 1, 0, 30deg)",
        Transform::Rotate3d(0.0, 1.0, 0.0, Angle::new(30.0)),
      ),
      ("translateZ(10px)", Transform::TranslateZ(Length::Px(10.0))),
      (
        "matrix3d(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)",
        Transform::Matrix3d(Matrix3d::IDENTITY),
      ),
    ] {
      assert_eq!(Transform::from_css_str(css), Ok(expected), "{css}");
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

  #[test]
  fn transform_errors_reject_missing_trailing_and_unknown_tokens() {
    assert!(Transform::from_css_str("translate(1px)").is_err());
    assert!(Transforms::from_css_str("translateX(1px) trailing").is_err());
    assert!(Transforms::from_css_str("spin(1deg)").is_err());
  }

  #[test]
  fn test_transform_negative_scale_reflects() {
    assert_eq!(
      Transform::from_css_str("scale(-1)"),
      Ok(Transform::Scale(-1.0, -1.0))
    );
    assert_eq!(
      Transform::from_css_str("scaleX(-1)"),
      Ok(Transform::Scale(-1.0, DEFAULT_SCALE))
    );
    assert_eq!(
      Transform::from_css_str("scaleY(-2)"),
      Ok(Transform::Scale(DEFAULT_SCALE, -2.0))
    );
  }

  #[test]
  fn test_transform_invert() {
    let transform = Affine::rotation(Angle::new(45.0));

    assert!(transform.invert().is_some_and(|inverse| {
      let (x, y) = (1234.0, -5678.0);

      let (transformed_x, transformed_y) = transform.transform_point(x, y);
      let (processed_x, processed_y) = inverse.transform_point(transformed_x, transformed_y);

      (x - processed_x).abs() < 1.0 && (y - processed_y).abs() < 1.0
    }));
  }
}
