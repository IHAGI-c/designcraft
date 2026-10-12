//! `texttest <doc> <frame id> [fonts…]` — click-to-caret, type, shift-select, delete, caret geometry (inline editing).
#![allow(clippy::unwrap_used, clippy::expect_used)]
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut e = designcraft_wasm::Engine::new();
    for f in &a[3..] {
        e.add_font(std::fs::read(f).unwrap());
    }
    e.open(&std::fs::read(&a[1]).unwrap()).unwrap();
    let fid: u64 = a[2].parse().unwrap();
    let items: serde_json::Value = serde_json::from_str(&e.items(0).unwrap()).unwrap();
    let f = items.as_array().unwrap().iter().find(|x| x["id"] == fid).unwrap().clone();
    let ink = f["ink"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect::<Vec<_>>();
    let ps = e.page_size(0).unwrap();
    let (cx, cy) = (ink[2] - 0.5 + ps[2], (ink[1] + ink[3]) / 2.0 + ps[3]); // end of the line
    let t = std::time::Instant::now();
    println!("place {:?}", e.execute("text.placeCaret", &format!(r#"{{"frame":{fid},"point":[{cx},{cy}]}}"#)));
    println!("state {}", e.text_state(0).unwrap());
    println!("insert {:?}", e.execute("text.insert", r#"{"text":" 만점"}"#));
    println!("state {}", e.text_state(0).unwrap());
    println!("shift-left x2 {:?} {:?}", e.execute("text.move", r#"{"dir":"left","extend":true}"#), e.execute("text.move", r#"{"dir":"left","extend":true}"#));
    println!("state {}", e.text_state(0).unwrap());
    println!("delete {:?}", e.execute("text.delete", "{}"));
    println!("story {}", e.execute("story.get", &format!(r#"{{"frame":{fid}}}"#)).unwrap());
    println!("all in {} ms", t.elapsed().as_millis());
    println!("exit {:?} sel {}", e.execute("text.exitToFrame", "{}"), e.text_state(0).unwrap());
}
