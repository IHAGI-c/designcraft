//! `grouptest <doc> <group id> <story id> <out prefix> [fonts…]` — text frames inside a group: draw, move the group, type into a story inside it.
#![allow(clippy::unwrap_used, clippy::expect_used)]
fn png(e: &designcraft_wasm::Engine, px: Vec<u8>, p: &str) {
    image::RgbaImage::from_raw(e.width(), e.height(), px).unwrap().save(p).unwrap();
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut e = designcraft_wasm::Engine::new();
    for f in &a[5..] {
        e.add_font(std::fs::read(f).unwrap());
    }
    e.open(&std::fs::read(&a[1]).unwrap()).unwrap();
    let (g, story, out) = (&a[2], &a[3], &a[4]);
    let px = e.render(0, 2.0).unwrap();
    png(&e, px, &format!("{out}-0.png"));
    let r = e.execute("transform.move", &format!(r#"{{"dx":-40,"dy":20,"ids":[{g}]}}"#));
    println!("move {r:?}");
    let px = e.render(0, 2.0).unwrap();
    png(&e, px, &format!("{out}-1.png"));
    let st = e.execute("story.get", &format!(r#"{{"story":{story}}}"#)).unwrap();
    println!("story {st}");
    println!("select {:?}", e.execute("text.select", &format!(r#"{{"story":{story},"anchor":0,"focus":0}}"#)));
    println!("insert {:?}", e.execute("text.insert", r#"{"text":"총 "}"#));
    let px = e.render(0, 2.0).unwrap();
    png(&e, px, &format!("{out}-2.png"));
    let ins: serde_json::Value = serde_json::from_str(&e.execute("document.inspect", "{}").unwrap()).unwrap();
    println!("overset stories {}", ins["stories"].as_array().unwrap().iter().filter(|s| s["overset"] == true).count());
    println!("undo {:?}", e.execute("edit.undo", "{}"));
    std::fs::write(format!("{out}.designcraft"), e.save().unwrap()).unwrap();
}
