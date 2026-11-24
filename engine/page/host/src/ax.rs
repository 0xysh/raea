//! The page's accessibility tree published to the OS (raea's window, the `window` feature): VoiceOver and macOS
//! accessibility clients (an AX test driving raea.app, Accessibility Inspector) read every element of the page as
//! AccessKit has it, by raea-sys-a11y's computation (the role, name and states Chrome gives the same markup), with its
//! box. Bevy's winit adapter carries it: Bevy's own tree (its UI entities' AccessibilityNode) is turned off
//! (ManageAccessibilityUpdates), and this sends the page's instead, while a client is listening (AccessibilityRequested)
//! and at most every 250 ms. An action a client asks of an element comes back as the input a person would give:
//! Click and SetValue press the element's middle (a pointer down and up, as the MCP input tool queues them), and
//! SetValue then types its text over what the field held.
//!
//! The tree is the Blitz document's own, walked from its root: an element's attributes are its props, and React's
//! props over them where the element came from React (the wire's ids). A page's markup set as HTML (innerHTML: raea
//! cloud's dashboard builds its rail so) has no wire ids, and the BRP `a11y` method's elements (page.rs) leave it out.
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use accesskit::{Action, ActionData, NodeId};
use bevy::a11y::{AccessibilityRequested, ActionRequest, ManageAccessibilityUpdates};
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::accessibility::ACCESS_KIT_ADAPTERS;
use blitz_dom::{BaseDocument, Node, NodeId as Id};
use raea_sys_a11y::El;
use raea_sys_page::ops::Nodes;
use serde_json::{Map, Value, json};

use crate::inject::{Frame, Pending};

/// A page node's AccessKit id: its Blitz id (slot and version) with the top bit set, clear of the window entity's
/// (Entity::to_bits: its generation never reaches it, nor does a Blitz slot's version)
const PAGE: u64 = 1 << 63;
const EVERY: Duration = Duration::from_millis(250);
/// what a page never draws, and an assistive client has no use for
const UNDRAWN: &[&str] = &["head", "script", "style", "template", "noscript", "link", "meta", "title"];

pub struct AxPlugin;

#[derive(Resource, Default)]
struct Ax {
    last: Option<Instant>,
    /// the frame last sent: a window at rest draws none, and has nothing new to say
    frame: u64,
}

impl Plugin for AxPlugin {
    fn build(&self, app: &mut App) {
        let mut manage = ManageAccessibilityUpdates::default();
        manage.set(false);
        app.insert_resource(manage).init_resource::<Ax>().add_systems(PostUpdate, (act, publish).chain());
    }
}

/// The page's tree sent to the window's adapter, while a client listens.
fn publish(world: &mut World) {
    let listening = world.get_resource::<AccessibilityRequested>().is_some_and(|r| r.get());
    if world.get_resource::<crate::idle::Listening>().is_none_or(|l| l.0 != listening) {
        world.insert_resource(crate::idle::Listening(listening));
    }
    if !listening {
        return;
    }
    let (now, frame) = (Instant::now(), world.resource::<Frame>().0);
    let ax = world.resource::<Ax>();
    if ax.frame == frame || ax.last.is_some_and(|t| now - t < EVERY) {
        return;
    }
    let Ok((win, w)) = world.query_filtered::<(Entity, &Window), With<PrimaryWindow>>().single(world) else { return };
    let (scale, title) = (w.scale_factor() as f64, w.title.clone());
    let root = NodeId(win.to_bits());
    let Some(update) = crate::page::with_doc(world, |doc, nodes| {
        let (els, tops) = elements(doc, nodes);
        let facts = Facts { doc, nodes };
        let focus = doc.get_focussed_node_id().filter(|f| els.contains_key(f));
        let memo = RefCell::new(blitz_dom::RectMemo::default());
        // in the window's physical pixels, as Bevy's own nodes are
        let bounds = |k: Id| {
            let b = doc.get_client_bounding_rect_in(element(doc, k)?, &mut memo.borrow_mut())?;
            Some([b.x * scale, b.y * scale, (b.x + b.width) * scale, (b.y + b.height) * scale])
        };
        raea_sys_a11y::tree(els, tops, &facts, focus, |k| k.as_u64() | PAGE, root, Some(&title), bounds).0
    }) else {
        return;
    };
    ACCESS_KIT_ADAPTERS.with_borrow_mut(|adapters| {
        if let Some(a) = adapters.get_mut(&win) {
            a.update_if_active(|| update);
        }
    });
    let mut ax = world.resource_mut::<Ax>();
    (ax.last, ax.frame) = (Some(now), frame);
}

/// The document's drawn nodes, from its root: elements with their attributes (React's props over them) and text.
/// One document: every node shares one root for `id` lookups.
fn elements(doc: &BaseDocument, nodes: &Nodes) -> (HashMap<Id, El<Id>>, HashMap<Id, Id>) {
    let mut els = HashMap::new();
    // from the document's root, and from any React tree drawn apart from it (a layer whose top isn't the document:
    // raea.app's welcome over the dashboard), each top once
    let root = doc.root_node().id;
    let mut apart = std::collections::BTreeSet::new();
    for &n in nodes.by_id.values() {
        let mut top = n;
        while let Some(p) = doc.get_node(top).and_then(|t| t.parent) {
            top = p;
        }
        if top != root && doc.get_node(top).is_some() {
            apart.insert(top.as_u64());
        }
    }
    let mut stack: Vec<(Id, Option<Id>)> = apart.iter().rev().map(|t| (Id::from_u64(*t), None)).collect();
    stack.extend(doc.root_node().children.iter().rev().map(|c| (*c, None)));
    while let Some((k, parent)) = stack.pop() {
        let Some(n) = doc.get_node(k) else { continue };
        let el = if n.is_text_node() {
            El { tag: "text".into(), props: Map::new(), parent, kids: vec![], text: Some(n.text_content()), title: None }
        } else if let Some(e) = n.element_data() {
            let tag = e.name.local.to_string();
            if UNDRAWN.contains(&tag.as_str()) {
                continue;
            }
            let mut props: Map<String, Value> = e.attrs().iter().map(|a| (a.name.local.to_string(), Value::String(a.value.to_string()))).collect();
            // the computation reads React's spelling
            if let Some(t) = props.remove("tabindex") {
                props.insert("tabIndex".into(), t);
            }
            let wire = nodes.id_of(k);
            if let Some(p) = wire.and_then(|id| nodes.props.get(&id)) {
                props.extend(p.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
            let tag = match wire.and_then(|id| nodes.kind(id)) {
                Some("editableText") if props.get("multiline") == Some(&Value::Bool(true)) => "textarea".into(),
                Some("editableText") => "input".into(),
                _ => tag,
            };
            let title = (tag == "svg")
                .then(|| n.children.iter().filter_map(|c| doc.get_node(*c)).find(|c| c.element_data().is_some_and(|e| &*e.name.local == "title")).map(|t| t.text_content()))
                .flatten();
            let kids: Vec<Id> = n.children.iter().copied().filter(|c| doc.get_node(*c).is_some_and(|c| c.is_text_node() || c.is_element())).collect();
            stack.extend(kids.iter().map(|c| (*c, Some(k))));
            El { tag, props, parent, kids, text: None, title }
        } else {
            continue;
        };
        els.insert(k, el);
    }
    // a kid left out (an undrawn element) is no kid
    let have: HashSet<Id> = els.keys().copied().collect();
    for el in els.values_mut() {
        el.kids.retain(|c| have.contains(c));
    }
    let tops = els.keys().map(|k| (*k, Id::default())).collect();
    (els, tops)
}

/// What the computation asks of the Blitz document: its computed style and its controls' state, by Blitz id.
struct Facts<'a> {
    doc: &'a BaseDocument,
    nodes: &'a Nodes,
}

impl Facts<'_> {
    fn el(&self, k: Id) -> Option<&Node> {
        self.doc.get_node(k).filter(|n| n.is_element())
    }
}

impl raea_sys_a11y::Facts<Id> for Facts<'_> {
    fn display_none(&self, k: Id) -> bool {
        // an element with no style isn't laid out (it or an ancestor is display: none, or it was never styled)
        self.el(k).is_some_and(|n| n.primary_styles().is_none_or(|s| s.get_box().display.is_none()))
    }
    fn invisible(&self, k: Id) -> bool {
        use style::computed_values::visibility::T as V;
        self.el(k).and_then(|n| n.primary_styles()).is_some_and(|s| matches!(s.clone_visibility(), V::Hidden | V::Collapse))
    }
    fn natively_disabled(&self, _: Id) -> bool {
        false
    }
    fn editor_value(&self, k: Id) -> Option<String> {
        self.el(k).and_then(|n| n.element_data()).and_then(|e| e.text_input_data()).map(|t| t.editor.raw_text().to_string())
    }
    fn tabbable(&self, k: Id) -> bool {
        self.el(k).is_some_and(|n| n.is_focussable())
    }
    fn checked(&self, k: Id) -> bool {
        self.el(k).and_then(|n| n.element_data()).and_then(|e| e.checkbox_input_checked()).unwrap_or(false)
    }
    fn test_id(&self, k: Id) -> Option<String> {
        let wire = self.nodes.id_of(k).and_then(|id| self.nodes.props.get(&id)).and_then(|p| p.get("data-testid")).and_then(Value::as_str).map(str::to_string);
        wire.or_else(|| self.el(k)?.element_data()?.attrs().iter().find(|a| &*a.name.local == "data-testid").map(|a| a.value.to_string()))
    }
}

/// A Blitz node when it's an element: only an element has a box to ask for (Blitz panics asking a text node's
/// transform, which the rect's walk does: the first raea.app the AX test drove quit on its welcome, 11-23)
fn element(doc: &BaseDocument, k: Id) -> Option<Id> {
    doc.get_node(k).is_some_and(|n| n.is_element()).then_some(k)
}

/// What clients asked of the page's elements, as input: a press at the element's middle (and SetValue's text typed).
fn act(world: &mut World, reader: &mut SystemState<MessageReader<ActionRequest>>) {
    let Ok(mut r) = reader.get_mut(world) else { return };
    let asks: Vec<accesskit::ActionRequest> = r.read().map(|r| r.0.clone()).collect();
    for ask in asks {
        let Some(k) = (ask.target_node.0 & PAGE != 0).then(|| Id::from_u64(ask.target_node.0 & !PAGE)) else { continue };
        // Focus is no press: macOS asks it beside a press (AXPress), and a second press where the first one's new page
        // has something else clicked that (a press on Projects opened Publishing, 11-23)
        if !matches!(ask.action, Action::Click | Action::SetValue) {
            continue;
        }
        // a text node is pressed where its element is
        let at = crate::page::with_doc(world, |doc, _| {
            let k = element(doc, k).or_else(|| doc.get_node(k)?.parent)?;
            let b = doc.get_client_bounding_rect(element(doc, k)?)?;
            (b.width > 0.0 && b.height > 0.0).then(|| (b.x + b.width / 2.0, b.y + b.height / 2.0))
        })
        .flatten();
        let Some((x, y)) = at else { continue };
        let next = world.resource::<Frame>().0 + 1;
        let mut p = world.resource_mut::<Pending>();
        p.queue(json!({"type": "down", "x": x, "y": y}), next);
        let mut last = p.queue(json!({"type": "up", "x": x, "y": y}), next);
        if let (Action::SetValue, Some(ActionData::Value(v))) = (ask.action, &ask.data) {
            // what the field held selected, then the new text over it
            p.queue(json!({"type": "key", "key": "a", "mods": [if cfg!(target_os = "macos") { "cmd" } else { "ctrl" }]}), next);
            last = p.queue(json!({"type": "text", "text": v.to_string()}), next);
        }
        crate::inject::LAST_INPUT.fetch_max(last, std::sync::atomic::Ordering::SeqCst);
    }
}
