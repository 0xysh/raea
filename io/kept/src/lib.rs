//! A long render's 3D parts kept on disk as its frames come, for the checks at its end (docs/stage-guards.feature
//! SG4a). Each frame's scene was held in memory to the end: a 7-minute film grew by about 29 KB a frame (each changed
//! part's props, its node and its parent's list), most of what still grew with the frame count. Each frame's parts go to
//! a temp file instead: each props map, string and material is written once by its content (a part unchanged since the
//! frame before is known by its pointer and not written again), and the parts read back at the end share equal
//! contents. The file is gone when raea is (unlinked once open on unix, delete-on-close on Windows), as the encoders'
//! samples are (raea-io-codec samples.rs).
//!
//! A part's place is its matrix: the props' own place (PLACE) is kept only for a camera, the one part the checks read it
//! from (systems/stage view.rs, systems/contact seen.rs). A part that moves each frame has the same props otherwise,
//! so its frames share one map: the one-scene control's 48 moving boxes were 1.35 GB of maps read back, one a frame.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, Seek, Write};
use std::sync::{Arc, Mutex};

use bevy::math::Mat4;
use raea_core_scene::{Item, Node3};
use serde_json::{Map, Value, json};

type R<T> = Result<T, String>;
type Props = Arc<Map<String, Value>>;
/// a shared value's pointers: what (0 props, 1 a string, 2 a material) and up to three of its Arcs
type Key = (u8, usize, usize, usize);
/// a part's own place in its props, left out but for a camera's
const PLACE: [&str; 4] = ["position", "rotation", "scale", "quaternion"];

/// What a part held the frame before: kept for the frame, so its pointer isn't another value's (held, never read).
#[allow(dead_code)]
enum Held {
    Props(Props),
    Str(Arc<str>),
    Node(Node3),
}

/// Every frame's parts, on disk.
pub struct Kept {
    file: std::io::BufWriter<std::fs::File>,
    frames: usize,
    /// the frame before's shared values, by pointer, with their ids
    last: HashMap<Key, (String, Held)>,
}

impl Kept {
    pub fn new() -> R<Kept> {
        let f = raea_io_home::scratch_file("kept")?;
        Ok(Kept { file: std::io::BufWriter::with_capacity(1 << 20, f), frames: 0, last: HashMap::new() })
    }

    /// The next frame's parts.
    pub fn put(&mut self, items: &[Item]) -> R<()> {
        let mut now: HashMap<Key, (String, Held)> = HashMap::with_capacity(self.last.len());
        let mut defs = String::new();
        let mut row = Vec::with_capacity(items.len());
        for it in items {
            let props = &it.props;
            let p = self.id((0, Arc::as_ptr(props) as *const () as usize, 0, 0), &mut now, &mut defs, || {
                let text = if it.kind != "camera" && PLACE.iter().any(|k| props.contains_key(*k)) {
                    serde_json::to_string(&Placeless(props))
                } else {
                    serde_json::to_string(&**props)
                };
                (Held::Props(props.clone()), text.unwrap_or_default())
            });
            let s = it.shape.as_ref().map(|s| self.string(s, &mut now, &mut defs));
            let a = it.at.as_ref().map(|s| self.string(s, &mut now, &mut defs));
            let m = it.material.as_ref().map(|n| {
                let key = (2, Arc::as_ptr(&n.props) as *const () as usize, Arc::as_ptr(&n.kids) as *const () as usize, Arc::as_ptr(&n.kind) as *const () as usize);
                self.id(key, &mut now, &mut defs, || (Held::Node(n.clone()), node_json(n).to_string()))
            });
            let world: Vec<u32> = it.world.to_cols_array().iter().map(|v| v.to_bits()).collect();
            row.push(json!([it.kind, p, s, a, m, world]));
        }
        self.last = now;
        defs.push_str(&json!({ "f": row }).to_string());
        defs.push('\n');
        self.file.write_all(defs.as_bytes()).map_err(|e| format!("keeping a frame's parts on disk: {e}"))?;
        self.frames += 1;
        Ok(())
    }

    /// Frames given so far.
    pub fn len(&self) -> usize {
        self.frames
    }

    pub fn is_empty(&self) -> bool {
        self.frames == 0
    }

    /// Every frame's parts, as they were given, equal props, strings and materials shared. A frame's line is read
    /// into the parts alone and a definition only when its id is new: each line parsed whole into JSON values left
    /// 2.7 GB of the film's memory freed but not yet given back at the checks' start.
    pub fn read(self) -> R<Vec<Vec<Item>>> {
        let bad = |e: String| format!("reading the kept parts back: {e}");
        let mut f = self.file.into_inner().map_err(|e| bad(e.to_string()))?;
        f.seek(std::io::SeekFrom::Start(0)).map_err(|e| bad(e.to_string()))?;
        let (mut props, mut strs, mut nodes) = (HashMap::<String, Props>::new(), HashMap::<String, Arc<str>>::new(), HashMap::<String, Node3>::new());
        let mut out = Vec::with_capacity(self.frames);
        for line in std::io::BufReader::new(f).lines() {
            let line = line.map_err(|e| bad(e.to_string()))?;
            if line.starts_with("{\"f\":") {
                let fr: Frame = serde_json::from_str(&line).map_err(|e| bad(e.to_string()))?;
                out.push(fr.f.iter().map(|r| item(r, &props, &strs, &nodes)).collect::<R<Vec<_>>>().map_err(bad)?);
                continue;
            }
            // {"p":"<id>","v":<value>}, as `id` writes it
            let (Some(tag), Some(id), Some(v)) = (line.get(2..3), line.get(6..38), line.get(44..line.len().saturating_sub(1))) else { return Err(bad("a line that isn't a part or a definition".into())) };
            let json = |e: serde_json::Error| bad(e.to_string());
            match tag {
                "p" if !props.contains_key(id) => drop(props.insert(id.into(), Arc::new(serde_json::from_str(v).map_err(json)?))),
                "s" if !strs.contains_key(id) => drop(strs.insert(id.into(), Arc::from(serde_json::from_str::<String>(v).map_err(json)?))),
                "m" if !nodes.contains_key(id) => drop(nodes.insert(id.into(), node_of(&serde_json::from_str(v).map_err(json)?).map_err(bad)?)),
                _ => {}
            }
        }
        if out.len() != self.frames {
            return Err(bad(format!("{} frames back of {}", out.len(), self.frames)));
        }
        Ok(out)
    }

    fn string(&mut self, s: &Arc<str>, now: &mut HashMap<Key, (String, Held)>, defs: &mut String) -> String {
        let key = (1, Arc::as_ptr(s) as *const () as usize, s.len(), 0);
        self.id(key, now, defs, || (Held::Str(s.clone()), serde_json::to_string(&**s).unwrap_or_default()))
    }

    /// A shared value's id: the one it had this frame or the frame before (same pointer), else its content's, its
    /// definition written ahead of the frame.
    fn id(&mut self, key: Key, now: &mut HashMap<Key, (String, Held)>, defs: &mut String, made: impl FnOnce() -> (Held, String)) -> String {
        if let Some((id, _)) = now.get(&key) {
            return id.clone();
        }
        let (id, held) = self.last.remove(&key).unwrap_or_else(|| {
            let (held, text) = made();
            let id = id_of(&text);
            let tag = ["p", "s", "m"][key.0 as usize];
            defs.push_str(&format!("{{\"{tag}\":\"{id}\",\"v\":{text}}}\n"));
            (id, held)
        });
        now.insert(key, (id.clone(), held));
        id
    }
}

/// A part's props without its own place (PLACE), written as they are otherwise.
struct Placeless<'a>(&'a Map<String, Value>);

impl serde::Serialize for Placeless<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_map(self.0.iter().filter(|(k, _)| !PLACE.contains(&k.as_str())))
    }
}

/// 128 bits of a value's text: two SipHashes with fixed keys, told apart by a first byte.
fn id_of(text: &str) -> String {
    let half = |k: u8| {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        k.hash(&mut h);
        text.hash(&mut h);
        h.finish()
    };
    format!("{:016x}{:016x}", half(0), half(1))
}

fn node_json(n: &Node3) -> Value {
    json!({ "k": &*n.kind, "p": &*n.props, "s": n.shape.as_deref(), "a": n.at.as_deref(), "c": n.kids.iter().map(node_json).collect::<Vec<_>>() })
}

fn node_of(v: &Value) -> R<Node3> {
    let Some(Value::Object(p)) = v.get("p") else { return Err("a material without props".into()) };
    let kids = v.get("c").and_then(Value::as_array).map_or(Ok(Vec::new()), |c| c.iter().map(node_of).collect::<R<Vec<_>>>())?;
    let s = |k: &str| v.get(k).and_then(Value::as_str).map(Arc::from);
    Ok(Node3 { kind: s("k").unwrap_or_else(|| Arc::from("")), props: Arc::new(p.clone()), shape: s("s"), at: s("a"), kids: kids.into() })
}

/// A frame's line: its parts, each its kind, the ids of its props, shape, placements and material, and its place's
/// 16 numbers as bits.
#[derive(serde::Deserialize)]
struct Frame<'a> {
    #[serde(borrow)]
    f: Vec<Row<'a>>,
}

#[derive(serde::Deserialize)]
struct Row<'a>(&'a str, &'a str, Option<&'a str>, Option<&'a str>, Option<&'a str>, [u32; 16]);

fn item(r: &Row, props: &HashMap<String, Props>, strs: &HashMap<String, Arc<str>>, nodes: &HashMap<String, Node3>) -> R<Item> {
    let p = props.get(r.1).ok_or("a part whose props weren't written")?.clone();
    let (shape, at, material) = (named(strs, r.2, "shape")?, named(strs, r.3, "placements")?, named(nodes, r.4, "material")?);
    Ok(Item { kind: kind_of(r.0), props: p, shape, at, material, world: Mat4::from_cols_array(&r.5.map(f32::from_bits)) })
}

/// What an id names, None for no id; an id that wasn't written is an error saying which of the part's it was.
fn named<T: Clone>(m: &HashMap<String, T>, id: Option<&str>, what: &str) -> R<Option<T>> {
    id.map(|id| m.get(id).cloned().ok_or(format!("a part whose {what} wasn't written"))).transpose()
}

/// A part's kind as the static name items_of gives it: the kinds are few, each kept once.
fn kind_of(k: &str) -> &'static str {
    static KINDS: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let mut ks = KINDS.lock().unwrap();
    if let Some(s) = ks.iter().find(|s| **s == k) {
        return s;
    }
    let s: &'static str = Box::leak(k.to_string().into_boxed_str());
    ks.push(s);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(kind: &'static str, props: &Props, shape: Option<&Arc<str>>, x: f32) -> Item {
        let material = Some(Node3 { kind: Arc::from("meshStandardMaterial"), props: Arc::new(json!({"color": "#ff7a00"}).as_object().unwrap().clone()), shape: None, at: None, kids: Vec::new().into() });
        Item { kind, props: props.clone(), shape: shape.cloned(), at: None, material, world: Mat4::from_translation(bevy::math::Vec3::new(x, 0.1, -2.0)) }
    }

    #[test]
    fn parts_come_back_as_they_went_with_equal_props_shared() {
        let still: Props = Arc::new(json!({"name": "floor", "position": [0, 0, 0]}).as_object().unwrap().clone());
        let shape: Arc<str> = Arc::from("box(4, 0.1, 4)");
        let mut k = Kept::new().unwrap();
        // the floor unchanged across three frames; a card and a camera whose props are made again each frame as they move
        for x in [0.5f32, 0.75, 0.5] {
            let card: Props = Arc::new(json!({"name": "card", "position": [x, 1, 0], "rotation": [0, x, 0]}).as_object().unwrap().clone());
            let cam: Props = Arc::new(json!({"position": [x, 1, 5], "fov": 40}).as_object().unwrap().clone());
            k.put(&[part("mesh", &still, Some(&shape), 0.0), part("panel", &card, None, x), part("camera", &cam, None, 0.0)]).unwrap();
        }
        assert_eq!(k.len(), 3);
        let back = k.read().unwrap();
        assert_eq!(back.len(), 3);
        assert_eq!((back[1][0].kind, back[1][1].kind), ("mesh", "panel"));
        assert!(Arc::ptr_eq(&back[0][0].props, &back[2][0].props), "one floor's props, not three");
        assert_eq!(*back[2][0].props, *json!({"name": "floor"}).as_object().unwrap(), "a part's place is its matrix");
        assert!(Arc::ptr_eq(&back[0][1].props, &back[1][1].props) && Arc::ptr_eq(&back[1][1].props, &back[2][1].props), "a moving card's frames share one map");
        assert_eq!(back[1][2].props["position"][0], json!(0.75), "a camera keeps its own place: the checks read it");
        assert!(Arc::ptr_eq(&back[0][2].props, &back[2][2].props) && !Arc::ptr_eq(&back[0][2].props, &back[1][2].props));
        assert_eq!(back[0][0].shape.as_deref(), Some("box(4, 0.1, 4)"));
        assert_eq!(back[1][1].world.to_cols_array().map(f32::to_bits), Mat4::from_translation(bevy::math::Vec3::new(0.75, 0.1, -2.0)).to_cols_array().map(f32::to_bits));
        assert_eq!(back[2][1].material.as_ref().map(|m| m.props["color"].clone()), Some(json!("#ff7a00")));
    }
}
