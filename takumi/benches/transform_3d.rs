use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use takumi::{
  prelude::{Fonts, FromHtml, FromHtmlOptions, Node, RenderOptions, StyleSheet, Viewport},
  render,
};

const CARD_CSS: &str = r#"
  .stage {
    display: flex;
    width: 100%;
    height: 100%;
    align-items: center;
    justify-content: center;
    background: rgb(240, 240, 240);
    perspective: 1200px;
  }

  .card {
    display: flex;
    width: 600px;
    height: 400px;
    border-radius: 32px;
    background: linear-gradient(135deg, rgb(56, 189, 248), rgb(168, 85, 247));
  }
"#;

fn card(transform: &str) -> Node {
  Node::from_html(
    &format!(r#"<div class="stage"><div class="card" style="transform: {transform}"></div></div>"#),
    FromHtmlOptions::default(),
  )
  .unwrap()
}

fn render_card(fonts: &Fonts, stylesheet: &StyleSheet, node: Node) {
  let options = RenderOptions::builder()
    .viewport(Viewport::new((1080, 1920)))
    .node(node)
    .fonts(fonts)
    .stylesheet(stylesheet.clone().into())
    .build();

  black_box(render(options).unwrap());
}

fn bench_transform_3d(c: &mut Criterion) {
  let fonts = Fonts::default();
  let stylesheet = StyleSheet::parse(CARD_CSS).unwrap();
  let mut group = c.benchmark_group("transform_3d");

  group.bench_function("flat_card_1080x1920", |b| {
    b.iter(|| render_card(&fonts, &stylesheet, card("rotate(4deg)")))
  });
  group.bench_function("tilted_card_1080x1920", |b| {
    b.iter(|| {
      render_card(
        &fonts,
        &stylesheet,
        card("perspective(1200px) rotateX(18deg) rotateY(-28deg)"),
      )
    })
  });
  group.finish();
}

criterion_group! {
  name = benches;
  config = Criterion::default().sample_size(30);
  targets = bench_transform_3d
}
criterion_main!(benches);
