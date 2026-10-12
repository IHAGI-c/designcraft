//! `frames <doc.designcraft> [font files…]` → the page-0 frames JSON (compare with the server's items-ink.json).
#![allow(clippy::unwrap_used, clippy::expect_used)]
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut e = designcraft_wasm::Engine::new();
    for f in &a[2..] {
        e.add_font(std::fs::read(f).unwrap());
    }
    e.open(&std::fs::read(&a[1]).unwrap()).unwrap();
    println!("{}", e.items(0).unwrap_or_else(|_| "ERR".into()));
}
