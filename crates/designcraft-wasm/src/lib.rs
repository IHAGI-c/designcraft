//! DesignCraft engine for a host web page: the page owns the UI, this crate owns layout and drawing.
//! Every edit goes through `Session::execute` (the same command channel as the CLI, MCP and the app),
//! so what the browser draws is what the server exports from the same version.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::sync::Arc;

use designcraft_engine::doc::{Content, Document, SpreadRef};
use designcraft_engine::{DocState, Session};
use designcraft_geom::{Affine, Vec2};
use designcraft_render::{Placed, RenderOptions, Renderer};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Engine {
    session: Session,
    renderer: Renderer,
    width: u32,
    height: u32,
    /// Patch origin (px) within the page image of the last `renderDirty`.
    x: u32,
    y: u32,
    /// The document as last drawn (what the host canvas shows) — damage is measured against it.
    shown: Option<Arc<Document>>,
}

/// Screen drawing: hairlines at least one device pixel (the file keeps its weights — same rule as the host app's PPT viewer).
fn screen() -> RenderOptions {
    RenderOptions { min_stroke_px: 1.0, ..RenderOptions::default() }
}

fn err(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Engine {
        Engine { session: Session::new(), renderer: Renderer::new(), width: 0, height: 0, x: 0, y: 0, shown: None }
    }

    /// Add TTF/OTF/TTC bytes to the shared fonts. Returns the number of faces added.
    #[wasm_bindgen(js_name = addFont)]
    pub fn add_font(&self, bytes: Vec<u8>) -> usize {
        designcraft_fonts::FontDb::global().add_font(bytes)
    }

    /// Open `.designcraft` bytes as the active document. Returns its page count.
    pub fn open(&mut self, bytes: &[u8]) -> Result<usize, JsError> {
        let doc = designcraft_format::load(bytes).map_err(err)?;
        self.shown = None;
        let i = self.session.add_document(DocState::new(doc, None));
        self.session.set_active(i);
        Ok(self.session.doc().map_err(err)?.doc.page_count())
    }

    /// Run one engine command (`id`, JSON params) → JSON result.
    pub fn execute(&mut self, id: &str, params: &str) -> Result<String, JsError> {
        let p: serde_json::Value = serde_json::from_str(params).map_err(err)?;
        let v = self.session.execute(id, &p).map_err(err)?;
        Ok(v.to_string())
    }

    /// Draw page `abs` (0-based) at `scale` px/pt → straight RGBA8; size in `width` / `height`.
    pub fn render(&mut self, abs: usize, scale: f64) -> Result<Vec<u8>, JsError> {
        let st = self.session.doc().map_err(err)?;
        let img = self
            .renderer
            .render_page(&st.doc, &self.session.cache, abs, scale, false, &screen())
            .ok_or_else(|| err("no such page"))?;
        self.width = img.width;
        self.height = img.height;
        self.x = 0;
        self.y = 0;
        self.shown = Some(st.doc.clone());
        Ok(img.to_straight())
    }

    /// Redraw only what changed on page `abs` since the last draw (the engine's damage, as the
    /// desktop app repaints while typing). Returns the patch pixels at (`x`, `y`) sized `width` ×
    /// `height` in the page image; the whole page when the change can reach anywhere; empty
    /// (`width` 0) when nothing on this page changed.
    #[wasm_bindgen(js_name = renderDirty)]
    pub fn render_dirty(&mut self, abs: usize, scale: f64) -> Result<Vec<u8>, JsError> {
        let st = self.session.doc().map_err(err)?;
        let doc = st.doc.clone();
        let Some(old) = self.shown.clone() else { return self.render(abs, scale) };
        let (si, pi) = doc.page_loc(abs).ok_or_else(|| err("no such page"))?;
        let page = doc.spreads.get(si).and_then(|s| s.pages.get(pi)).ok_or_else(|| err("no such page"))?.bounds();
        let Some(rects) = designcraft_render::damage::damage_with(&old, &doc, Some(&self.session.cache)) else {
            return self.render(abs, scale);
        };
        let (pw, ph) = ((page.width() * scale).round().max(1.0), (page.height() * scale).round().max(1.0));
        let mut b: Option<[f64; 4]> = None;
        for (sr, r) in rects {
            if sr != SpreadRef::Doc(si) {
                continue;
            }
            // Page pixels, padded for antialiasing, clipped to the page.
            let q = [
                ((r.x0 - page.x0) * scale - 2.0).floor().max(0.0),
                ((r.y0 - page.y0) * scale - 2.0).floor().max(0.0),
                ((r.x1 - page.x0) * scale + 2.0).ceil().min(pw),
                ((r.y1 - page.y0) * scale + 2.0).ceil().min(ph),
            ];
            if q[2] <= q[0] || q[3] <= q[1] {
                continue;
            }
            b = Some(b.map_or(q, |a| [a[0].min(q[0]), a[1].min(q[1]), a[2].max(q[2]), a[3].max(q[3])]));
        }
        self.shown = Some(doc.clone());
        let Some([x0, y0, x1, y1]) = b else {
            self.width = 0;
            self.height = 0;
            return Ok(vec![]);
        };
        // Snap to the rasteriser's tile grid so edge coverage is computed exactly as in the full page.
        const T: f64 = 16.0;
        let (x0, y0) = ((x0 / T).floor() * T, (y0 / T).floor() * T);
        let (x1, y1) = (((x1 / T).ceil() * T).min(pw), ((y1 / T).ceil() * T).min(ph));
        // Same transform as `render_page`, shifted by whole pixels — the patch's pixels equal the full page's.
        let view = Affine::translate(Vec2::new(-x0, -y0)) * Affine::scale(scale) * Affine::translate(-page.origin().to_vec2());
        let mut o = screen();
        o.background = Some([255, 255, 255, 255]);
        let placed = [Placed { spread: SpreadRef::Doc(si), xf: Affine::translate(Vec2::ZERO) }];
        let img = self.renderer.render(&doc, &self.session.cache, &placed, (x1 - x0) as u32, (y1 - y0) as u32, view, &o);
        self.x = x0 as u32;
        self.y = y0 as u32;
        self.width = img.width;
        self.height = img.height;
        Ok(img.to_straight())
    }

    /// Top-level items on page `abs` (page coordinates, pt), front-most last, as JSON:
    /// `[{id, kind: text|picture|group|shape|line, box, label?, story?, ink?, text?}]` — `box` = visible bounds,
    /// text frames add `ink` (where the composed lines sit — the frame can be taller than its text) and the
    /// part of the story shown in the frame.
    pub fn items(&self, abs: usize) -> Result<String, JsError> {
        let doc = &self.session.doc().map_err(err)?.doc;
        let (si, pi) = doc.page_loc(abs).ok_or_else(|| err("no such page"))?;
        let sp = doc.spreads.get(si).ok_or_else(|| err("no such page"))?;
        let page = sp.pages.get(pi).ok_or_else(|| err("no such page"))?.bounds();
        let rel = |v: [f64; 4]| [v[0] - page.x0, v[1] - page.y0, v[2] - page.x0, v[3] - page.y0];
        let mut out = Vec::new();
        for it in &sp.items {
            let b = it.visible_bounds();
            if b.x1 <= page.x0 || b.x0 >= page.x1 || it.hidden || it.opacity <= 0.0 {
                continue;
            }
            out.push(self.item_json(doc, it, b, &rel, None));
            // Text frames inside a group (chart numbers, legend, table cells): listed after it with `parent`,
            // so the host can select the group first and the text on a second click.
            if let Content::Group { items } = &it.content {
                let mut stack: Vec<&designcraft_engine::doc::Item> = items.iter().map(|c| c.as_ref()).collect();
                while let Some(c) = stack.pop() {
                    match &c.content {
                        Content::Text(_) => {
                            let cb = it.xf.transform_rect_bbox(c.visible_bounds());
                            out.push(self.item_json(doc, c, cb, &rel, Some(it.id.0)));
                        }
                        Content::Group { items } => stack.extend(items.iter().map(|x| x.as_ref())),
                        _ => {}
                    }
                }
            }
        }
        Ok(serde_json::Value::Array(out).to_string())
    }

    /// The text caret or selection on page `abs` as JSON, or `null` when no text is being edited:
    /// `{story, anchor, focus, frame, caret: [x0, y0, x1, y1] | null, quads: [[[x, y] ×4] …]}` (page coordinates, pt).
    /// The host draws the caret and the highlight over its canvas; typing goes through `text.*` commands.
    #[wasm_bindgen(js_name = textState)]
    pub fn text_state(&self, abs: usize) -> Result<String, JsError> {
        use designcraft_engine::compose;
        use designcraft_geom::Point;
        let st = self.session.doc().map_err(err)?;
        let doc = &st.doc;
        let Some(ts) = st.selection.text else { return Ok("null".into()) };
        let (si, pi) = doc.page_loc(abs).ok_or_else(|| err("no such page"))?;
        let page = doc.spreads.get(si).and_then(|s| s.pages.get(pi)).ok_or_else(|| err("no such page"))?.bounds();
        let cs = self.session.cache.get(doc, ts.story, None);
        let frame_xf = |id: designcraft_engine::doc::ItemId| {
            let loc = doc.find(id)?;
            let it = doc.item(id)?;
            Some(doc.parent_xf(&loc) * doc.text_xf(it))
        };
        let pt = |p: Point| [p.x - page.x0, p.y - page.y0];
        let range = ts.range();
        let mut quads = Vec::new();
        if !range.is_empty() {
            for ft in &cs.frames {
                let Some(m) = frame_xf(ft.frame) else { continue };
                for l in &ft.lines {
                    let s = range.start.max(l.range.start);
                    let e = range.end.min(l.range.end);
                    if s > e || (s == e && !(l.range.end < range.end && e == l.range.end)) {
                        continue;
                    }
                    let past = e == l.range.end && range.end > l.range.end;
                    for q in compose::highlight_quads(l, s, e, past) {
                        quads.push(q.map(|p| pt(m * p)));
                    }
                }
            }
        }
        let mut caret = serde_json::Value::Null;
        let mut frame = serde_json::Value::Null;
        if let Some((fi, x, bl, asc, desc)) = compose::caret(&cs, ts.focus)
            && let Some(ft) = cs.frames.get(fi)
            && let Some(m) = frame_xf(ft.frame)
        {
            let a = pt(m * Point::new(x, bl - asc));
            let b = pt(m * Point::new(x, bl + desc));
            caret = serde_json::json!([a[0], a[1], b[0], b[1]]);
            frame = serde_json::json!(ft.frame.0);
        }
        Ok(serde_json::json!({"story": ts.story.0, "anchor": ts.anchor, "focus": ts.focus, "frame": frame, "caret": caret, "quads": quads}).to_string())
    }

    /// Page `abs` in pt `[width, height, x0, y0]` — x0/y0 = its origin in spread coordinates
    /// (commands such as `transform.resize` take spread coordinates; `frames` reports page coordinates).
    #[wasm_bindgen(js_name = pageSize)]
    pub fn page_size(&self, abs: usize) -> Result<Vec<f64>, JsError> {
        let doc = &self.session.doc().map_err(err)?.doc;
        let (si, pi) = doc.page_loc(abs).ok_or_else(|| err("no such page"))?;
        let p = doc.spreads.get(si).and_then(|s| s.pages.get(pi)).ok_or_else(|| err("no such page"))?.bounds();
        Ok(vec![p.width(), p.height(), p.x0, p.y0])
    }

    /// Close every open document (keeps fonts) — call before opening another page so memory doesn't grow.
    #[wasm_bindgen(js_name = closeAll)]
    pub fn close_all(&mut self) {
        while !self.session.documents().is_empty() {
            self.session.close_document(0);
        }
        self.shown = None;
    }

    #[wasm_bindgen(getter)]
    pub fn x(&self) -> u32 {
        self.x
    }

    #[wasm_bindgen(getter)]
    pub fn y(&self) -> u32 {
        self.y
    }

    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The active document as `.designcraft` bytes.
    pub fn save(&self) -> Result<Vec<u8>, JsError> {
        designcraft_format::save(&self.session.doc().map_err(err)?.doc).map_err(err)
    }
}

impl Engine {
    fn item_json(
        &self,
        doc: &Document,
        it: &designcraft_engine::doc::Item,
        b: designcraft_geom::Rect,
        rel: &dyn Fn([f64; 4]) -> [f64; 4],
        parent: Option<u64>,
    ) -> serde_json::Value {
        let kind = match &it.content {
            Content::Text(_) => "text",
            Content::Graphic(_) => "picture",
            Content::Group { .. } => "group",
            Content::Unassigned if b.width() < 2.0 || b.height() < 2.0 => "line",
            Content::Unassigned => "shape",
        };
        let mut v = serde_json::json!({"id": it.id.0, "kind": kind, "box": rel([b.x0, b.y0, b.x1, b.y1])});
        if let Some(p) = parent {
            v["parent"] = serde_json::json!(p);
        }
        if !it.label.is_empty() {
            v["label"] = serde_json::json!(it.label);
        }
        if let Content::Text(t) = &it.content {
            let cs = self.session.cache.get(doc, t.story, None);
            if let Some(ft) = cs.frame(it.id) {
                let xf = doc.text_xf(it);
                let mut ink: Option<[f64; 4]> = None;
                for l in &ft.lines {
                    let Some(g0) = l.glyphs.iter().find(|g| g.visible) else { continue };
                    let r = designcraft_geom::Rect::new(g0.x, l.baseline - l.ascent, l.end_x, l.baseline + l.descent);
                    let q = xf.transform_rect_bbox(r);
                    ink = Some(ink.map_or([q.x0, q.y0, q.x1, q.y1], |a| [a[0].min(q.x0), a[1].min(q.y0), a[2].max(q.x1), a[3].max(q.y1)]));
                }
                v["story"] = serde_json::json!(t.story.0);
                v["text"] = serde_json::json!(doc.story(t.story).and_then(|st| st.text.get(ft.range.clone())).unwrap_or(""));
                if let Some(i) = ink {
                    v["ink"] = serde_json::json!(rel(i));
                }
            }
        }
        v
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
