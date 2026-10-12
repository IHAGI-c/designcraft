//! `childmove <doc> <id> [fonts…]` — move one item that sits inside a group: which items move, and what items() reports.
#![allow(clippy::unwrap_used, clippy::expect_used)]
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut e = designcraft_wasm::Engine::new();
    for f in &a[3..] {
        e.add_font(std::fs::read(f).unwrap());
    }
    e.open(&std::fs::read(&a[1]).unwrap()).unwrap();
    let id = &a[2];
    let boxes = |e: &designcraft_wasm::Engine| -> std::collections::BTreeMap<u64, Vec<f64>> {
        let v: serde_json::Value = serde_json::from_str(&e.items(0).unwrap()).unwrap();
        v.as_array().unwrap().iter().map(|x| (x["id"].as_u64().unwrap(), x["box"].as_array().unwrap().iter().map(|n| n.as_f64().unwrap()).collect())).collect()
    };
    let b0 = boxes(&e);
    println!("move {:?}", e.execute("transform.move", &format!(r#"{{"dx":-10,"dy":5,"ids":[{id}]}}"#)));
    let b1 = boxes(&e);
    for (k, v) in &b0 {
        if let Some(w) = b1.get(k) && (w[0] - v[0]).abs() > 0.1 {
            println!("  items() {k} moved {:.1}", w[0] - v[0]);
        }
    }
    std::fs::write("/tmp/childmove.designcraft", e.save().unwrap()).unwrap();
}
