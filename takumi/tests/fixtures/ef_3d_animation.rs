//! espaciofuturo fork: a keyframe animation through the 3D transform path.

use takumi::{prelude::*, render_animation};

use crate::test_utils::{CONTEXT, run_animation_fixture_test};

const ROTATE_Y_FPS: u32 = 10;
const ROTATE_Y_DURATION_MS: u32 = 1000;

const ROTATE_Y_HTML: &str = r#"<div class="stage"><div class="card">Spin</div></div>"#;

const ROTATE_Y_CSS: &str = r#"
    .stage {
      display: flex;
      width: 100%;
      height: 100%;
      align-items: center;
      justify-content: center;
      background: rgb(240, 240, 240);
      perspective: 600px;
    }

    .card {
      display: flex;
      width: 260px;
      height: 180px;
      align-items: center;
      justify-content: center;
      border-radius: 20px;
      background: rgb(56, 189, 248);
      color: white;
      font-size: 48px;
      font-weight: 700;
      backface-visibility: hidden;
      animation: spin 1000ms linear both;
    }

    @keyframes spin {
      from {
        transform: rotateY(-80deg) rotateX(10deg);
      }

      50% {
        transform: rotateY(15deg) rotateX(10deg);
      }

      to {
        transform: rotateY(110deg) rotateX(10deg);
      }
    }
  "#;

fn rotate_y_frames() -> Vec<AnimationFrame> {
  let node = Node::from_html(ROTATE_Y_HTML, FromHtmlOptions::default())
    .unwrap_or_else(|error| panic!("expected html to parse: {error:?}"));
  let stylesheet = StyleSheet::parse(ROTATE_Y_CSS)
    .unwrap_or_else(|error| panic!("expected stylesheet to parse: {error:?}"));
  let scene = SequentialScene::builder()
    .options(
      RenderOptions::builder()
        .viewport(Viewport::new((480, 320)))
        .node(node)
        .fonts(&CONTEXT)
        .stylesheet(stylesheet.into())
        .build(),
    )
    .duration_ms(ROTATE_Y_DURATION_MS)
    .build();

  render_animation(&[scene], ROTATE_Y_FPS).unwrap()
}

#[test]
fn animation_keyframe_rotate_y() {
  run_animation_fixture_test(
    rotate_y_frames(),
    "animation_keyframe_rotate_y",
    ROTATE_Y_DURATION_MS,
    ROTATE_Y_FPS,
  );
}
