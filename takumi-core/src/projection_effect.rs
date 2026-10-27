//! espaciofuturo fork: the projection stage of the effect tree.
//!
//! A box whose composed 3D transform is projective paints flat into a layer of its own, which the
//! raster backend resamples through the device homography when the layer closes. The stage is
//! the box's outermost effect, so its clip-path, masks, filter and opacity apply to the flat paint
//! before the projection, and the clips of its ancestors after it. An affine result (`rotateX`
//! without perspective) takes upstream's 2D path untouched, as does every box without a 3D
//! function.

use crate::{
  context::RenderContext,
  geometry::ComputedLayout,
  paint_property::{EffectKind, EffectNode, PropertyState, PropertyTrees},
  scene::SceneBounds,
  style::{Affine, BackfaceVisibility, Homography, Matrix3d},
};

/// The device-space homography a projective box's flat layer is resampled through.
#[derive(Debug, Clone, Copy)]
pub struct ProjectionStage(Homography);

// Compared bit for bit, so the stage keeps `EffectKind`'s `Eq`.
impl PartialEq for ProjectionStage {
  fn eq(&self, other: &Self) -> bool {
    self.0.0.map(f32::to_bits) == other.0.0.map(f32::to_bits)
  }
}

impl Eq for ProjectionStage {}

impl ProjectionStage {
  /// The device-space map from the flat layer to the projected one.
  pub fn homography(self) -> Homography {
    self.0
  }

  /// The box's local transform for the scene and, when its 3D transform is projective, the
  /// homography its flat paint projects through in its own space; `None` when it faces away
  /// with `backface-visibility: hidden`. `perspective` is its containing block's, in that
  /// block's border-box space.
  pub(crate) fn local_transform(
    context: &RenderContext,
    layout: ComputedLayout,
    perspective: Option<Matrix3d>,
  ) -> Option<(Affine, Option<Homography>)> {
    let style = &context.style;
    let (width, height, sizing) = (layout.size.width, layout.size.height, &context.sizing);
    let perspective = perspective.map(|perspective| {
      let location = layout.location;

      Matrix3d::from(Affine::translation(-location.x, -location.y))
        * perspective
        * Matrix3d::from(Affine::translation(location.x, location.y))
    });
    let Some(matrix) = style.local_matrix3d(width, height, sizing, perspective) else {
      return Some((style.local_transform(width, height, sizing), None));
    };
    if style.ef_3d_data.backface_visibility == BackfaceVisibility::Hidden && matrix.is_back_facing()
    {
      return None;
    }
    let flat = matrix.flatten();

    Some(match flat.to_affine() {
      Some(affine) => (affine, None),
      None => (Affine::IDENTITY, Some(flat)),
    })
  }

  /// The perspective the box gives its children.
  pub(crate) fn child_perspective(
    context: &RenderContext,
    layout: ComputedLayout,
  ) -> Option<Matrix3d> {
    let size = layout.size;

    context
      .style
      .child_perspective(size.width, size.height, &context.sizing)
  }

  /// Adds the projection stage of the box at `path`, which sits at device `transform` and
  /// projects through the local `projection`, under `state`; the box's own effects nest inside.
  pub(crate) fn enter(
    trees: &mut PropertyTrees,
    state: PropertyState,
    path: &[usize],
    transform: Affine,
    projection: Option<Homography>,
  ) -> PropertyState {
    let Some(projection) = projection else {
      return state;
    };
    let Some(inverse) = transform.invert() else {
      return state;
    };
    let device = Homography::from(transform) * projection * Homography::from(inverse);
    let effect = trees.add_effect(EffectNode {
      kind: EffectKind::Projection(Self(device)),
      parent: state.effect,
      output_clip: state.clip,
      owner: path.to_vec(),
      bounds: None,
    });

    PropertyState {
      effect: Some(effect),
      ..state
    }
  }

  /// The bounds of an effect of `kind` that holds `flat`: for a projection stage the flat paint
  /// plus where it projects, unknown when a corner lands behind the viewer.
  pub(crate) fn grow(
    kind: EffectKind,
    flat: Option<SceneBounds>,
    unknown: bool,
  ) -> (Option<SceneBounds>, bool) {
    let (EffectKind::Projection(stage), Some(flat)) = (kind, flat) else {
      return (flat, unknown);
    };
    let projected = stage.0.map_bounds(
      flat.left as f32,
      flat.top as f32,
      flat.right as f32,
      flat.bottom as f32,
    );

    match projected {
      Some([left, top, right, bottom]) => (
        Some(flat.union(SceneBounds::enclosing(
          left - 1.0,
          top - 1.0,
          right + 1.0,
          bottom + 1.0,
        ))),
        unknown,
      ),
      None => (None, true),
    }
  }
}
