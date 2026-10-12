//! Native twin of the browser component (same code path): `render <doc.designcraft> <scale> <out.png> [font files…]`.
//! Used to check browser pixels == server pixels for one source version.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use designcraft_wasm::Engine;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut e = Engine::new();
    for f in &a[4..] {
        e.add_font(std::fs::read(f).unwrap());
    }
    e.open(&std::fs::read(&a[1]).unwrap()).unwrap();
    let t = std::time::Instant::now();
    let px = e.render(0, a[2].parse().unwrap()).unwrap();
    eprintln!("render {} ms ({}x{})", t.elapsed().as_millis(), e.width(), e.height());
    let img = image::RgbaImage::from_raw(e.width(), e.height(), px).unwrap();
    img.save(&a[3]).unwrap();
}
