//! espaciofuturo fork: `perspective`, `perspective-origin` and `backface-visibility`, kept in the
//! `ef_3d_data` style group.

use crate::style::{
  Affine, ComputedStyle, SizingContext,
  properties::{Matrix3d, impl_css_enum},
};

/// Whether the back face of a transformed element is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
pub enum BackfaceVisibility {
  /// The back face is drawn, mirrored.
  #[default]
  Visible,
  /// The element is not drawn while its back faces the viewer.
  Hidden,
}

impl_css_enum!(
  BackfaceVisibility,
  "visible" => BackfaceVisibility::Visible,
  "hidden" => BackfaceVisibility::Hidden
);

impl ComputedStyle {
  /// Whether a 3D property makes the box a stacking context: `perspective` other than `none`.
  pub(crate) fn creates_3d_stacking_context(&self) -> bool {
    self.ef_3d_data.perspective.is_some()
  }

  /// The perspective this box gives its children, `T(origin) * perspective(d) * T(-origin)` in
  /// its border-box space, or `None` for `perspective: none`.
  pub fn child_perspective(
    &self,
    width: f32,
    height: f32,
    sizing: &SizingContext,
  ) -> Option<Matrix3d> {
    let distance = self.ef_3d_data.perspective?.to_px(sizing, 0.0);
    let (origin_x, origin_y) = self
      .ef_3d_data
      .perspective_origin
      .to_point(sizing, width, height);

    Some(
      Matrix3d::from(Affine::translation(origin_x, origin_y))
        * Matrix3d::perspective(distance)
        * Matrix3d::from(Affine::translation(-origin_x, -origin_y)),
    )
  }
}
