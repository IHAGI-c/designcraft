//! `cmds <doc> <fonts dir> 'id=json' …` — run engine commands in order, print each result or error (probe for the host toolbar).
#![allow(clippy::unwrap_used, clippy::expect_used)]
use designcraft_engine::{DocState, Session};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    for f in std::fs::read_dir(&a[2]).unwrap() {
        let p = f.unwrap().path();
        if p.extension().is_some_and(|x| x == "otf" || x == "ttf") {
            designcraft_fonts::FontDb::global().add_font(std::fs::read(p).unwrap());
        }
    }
    let mut s = Session::new();
    let i = s.add_document(DocState::new(designcraft_format::load(&std::fs::read(&a[1]).unwrap()).unwrap(), None));
    s.set_active(i);
    for c in &a[3..] {
        let (id, p) = c.split_once('=').unwrap_or((c, "{}"));
        let r = s.execute(id, &serde_json::from_str(p).unwrap());
        let t = match r { Ok(v) => v.to_string(), Err(e) => format!("ERR {e}") };
        println!("{id} → {}", t.chars().take(20000).collect::<String>());
    }
}
