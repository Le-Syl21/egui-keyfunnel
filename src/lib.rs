//! [`KeyFunnel`] — funnel keyboard input to a single egui viewport.
//!
//! In a multi-viewport egui app — think a kiosk or pinball cabinet with a main
//! "playfield" window plus decorative backglass / DMD / topper windows — the OS
//! may hand keyboard focus to a *secondary* window. On Wayland especially, a
//! compositor can ignore [`ViewportBuilder::with_active(false)`] and focus a
//! freshly-mapped cover window; the keyboard events then land in that viewport,
//! whose UI ignores them, and your main viewport never sees the keys.
//!
//! [`KeyFunnel`] is an [`egui::Plugin`] that moves keyboard events out of the
//! secondary viewports and into a single **target** viewport (the root by
//! default), regardless of which window the compositor focused. Register it once
//! and keyboard input always reaches your main UI — no window-manager tricks, no
//! focus juggling, no backend fork.
//!
//! ```no_run
//! # let ctx = egui::Context::default();
//! use egui_keyfunnel::KeyFunnel;
//!
//! ctx.add_plugin(KeyFunnel::new());
//! ```
//!
//! Only **keyboard** events ([`Event::Key`], [`Event::Text`], [`Event::Ime`])
//! are moved. Pointer, touch and scroll are intentionally left alone: pointer
//! input should follow the window it is over (a click both focuses and interacts
//! with that window). Gamepad / joystick input read outside egui (e.g. via SDL)
//! is unaffected either way.
//!
//! [`ViewportBuilder::with_active(false)`]: egui::ViewportBuilder::with_active

use std::collections::HashSet;

use egui::{Context, Event, RawInput, ViewportId};

/// Which viewports keyboard events are pulled *from*.
#[derive(Clone, Debug, Default)]
enum Sources {
    /// Every viewport except the target (the default).
    #[default]
    AllButTarget,
    /// Only these viewports.
    Only(HashSet<ViewportId>),
}

/// An [`egui::Plugin`] that funnels keyboard events to a single viewport.
///
/// See the [crate-level docs](crate) for the rationale and a usage example.
///
/// ```
/// use egui_keyfunnel::KeyFunnel;
/// use egui::ViewportId;
///
/// // Default: every non-root viewport's keyboard input goes to the root.
/// let _ = KeyFunnel::new();
///
/// // Or funnel into a specific viewport, from specific sources only.
/// let dmd = ViewportId::from_hash_of("dmd");
/// let bg = ViewportId::from_hash_of("backglass");
/// let _ = KeyFunnel::new().to_viewport(ViewportId::ROOT).from_viewports([dmd, bg]);
/// ```
#[derive(Clone, Debug)]
pub struct KeyFunnel {
    target: ViewportId,
    sources: Sources,
    /// Keyboard events captured from the source viewports, awaiting delivery to
    /// the target viewport. Filled as each source viewport's input is hooked,
    /// drained when the target viewport's input is hooked.
    buffer: Vec<Event>,
}

impl Default for KeyFunnel {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyFunnel {
    /// Funnel keyboard input from every non-root viewport into the **root**
    /// viewport ([`ViewportId::ROOT`]).
    pub fn new() -> Self {
        Self {
            target: ViewportId::ROOT,
            sources: Sources::AllButTarget,
            buffer: Vec::new(),
        }
    }

    /// Set the viewport that should receive the funneled keyboard input
    /// (default [`ViewportId::ROOT`]).
    pub fn to_viewport(mut self, target: ViewportId) -> Self {
        self.target = target;
        self
    }

    /// Restrict the source viewports to `sources` (default: every viewport
    /// except the target). The target is never a source, even if listed here.
    pub fn from_viewports(mut self, sources: impl IntoIterator<Item = ViewportId>) -> Self {
        self.sources = Sources::Only(sources.into_iter().collect());
        self
    }

    /// The viewport that receives the funneled keyboard input.
    pub fn target(&self) -> ViewportId {
        self.target
    }

    fn is_source(&self, viewport: ViewportId) -> bool {
        if viewport == self.target {
            return false;
        }
        match &self.sources {
            Sources::AllButTarget => true,
            Sources::Only(set) => set.contains(&viewport),
        }
    }
}

/// The keyboard-family events that get funneled. Pointer / touch / scroll /
/// window events stay on their original viewport.
fn is_keyboard_event(event: &Event) -> bool {
    matches!(event, Event::Key { .. } | Event::Text(_) | Event::Ime(_))
}

impl egui::Plugin for KeyFunnel {
    fn debug_name(&self) -> &'static str {
        "egui_keyfunnel::KeyFunnel"
    }

    fn input_hook(&mut self, _ctx: &Context, input: &mut RawInput) {
        let viewport = input.viewport_id;

        if viewport == self.target {
            // Deliver everything captured from the source viewports. Prepend so
            // the funneled keys keep their order ahead of the target's own
            // input for this frame.
            if !self.buffer.is_empty() {
                let mut merged = std::mem::take(&mut self.buffer);
                merged.append(&mut input.events);
                input.events = merged;
            }
        } else if self.is_source(viewport) {
            // Pull keyboard events out of this viewport into the buffer, leaving
            // its pointer / touch / scroll events untouched.
            let mut kept = Vec::with_capacity(input.events.len());
            for event in input.events.drain(..) {
                if is_keyboard_event(&event) {
                    self.buffer.push(event);
                } else {
                    kept.push(event);
                }
            }
            input.events = kept;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Key, Modifiers, Plugin};

    fn raw(viewport: ViewportId, events: Vec<Event>) -> RawInput {
        RawInput {
            viewport_id: viewport,
            events,
            ..Default::default()
        }
    }

    fn key(k: Key, pressed: bool) -> Event {
        Event::Key {
            key: k,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::default(),
        }
    }

    fn keys(events: &[Event]) -> Vec<&Event> {
        events.iter().filter(|e| is_keyboard_event(e)).collect()
    }

    #[test]
    fn keyboard_moves_from_source_to_target() {
        let secondary = ViewportId::from_hash_of("secondary");
        let mut funnel = KeyFunnel::new(); // target = ROOT

        // Secondary viewport holds focus and receives the keys.
        let mut sec_in = raw(
            secondary,
            vec![key(Key::ArrowLeft, true), Event::Text("a".into())],
        );
        funnel.input_hook(&Context::default(), &mut sec_in);
        assert!(
            keys(&sec_in.events).is_empty(),
            "keyboard events must be removed from the source viewport"
        );

        // Root viewport gets them, even though it never had focus.
        let mut root_in = raw(ViewportId::ROOT, vec![]);
        funnel.input_hook(&Context::default(), &mut root_in);
        assert_eq!(
            keys(&root_in.events).len(),
            2,
            "the two keyboard events must arrive on the target viewport"
        );
    }

    #[test]
    fn pointer_stays_on_its_viewport() {
        let secondary = ViewportId::from_hash_of("secondary");
        let mut funnel = KeyFunnel::new();

        let mut sec_in = raw(
            secondary,
            vec![
                Event::PointerMoved(egui::pos2(1.0, 2.0)),
                key(Key::Enter, true),
            ],
        );
        funnel.input_hook(&Context::default(), &mut sec_in);

        // Pointer event kept, key event taken.
        assert_eq!(sec_in.events.len(), 1);
        assert!(matches!(sec_in.events[0], Event::PointerMoved(_)));
    }

    #[test]
    fn target_input_is_untouched_and_ordered() {
        let mut funnel = KeyFunnel::new();
        let sec = ViewportId::from_hash_of("s");

        let mut sec_in = raw(sec, vec![key(Key::A, true)]);
        funnel.input_hook(&Context::default(), &mut sec_in);

        // Root already has its own key; funneled key should come first.
        let mut root_in = raw(ViewportId::ROOT, vec![key(Key::B, true)]);
        funnel.input_hook(&Context::default(), &mut root_in);
        assert_eq!(keys(&root_in.events).len(), 2);
    }

    #[test]
    fn restricted_sources_are_respected() {
        let allowed = ViewportId::from_hash_of("allowed");
        let ignored = ViewportId::from_hash_of("ignored");
        let mut funnel = KeyFunnel::new().from_viewports([allowed]);

        // Ignored viewport keeps its keys.
        let mut ignored_in = raw(ignored, vec![key(Key::Escape, true)]);
        funnel.input_hook(&Context::default(), &mut ignored_in);
        assert_eq!(keys(&ignored_in.events).len(), 1);

        // Allowed viewport is funneled.
        let mut allowed_in = raw(allowed, vec![key(Key::Escape, true)]);
        funnel.input_hook(&Context::default(), &mut allowed_in);
        assert!(keys(&allowed_in.events).is_empty());
    }

    #[test]
    fn custom_target_receives_input() {
        let target = ViewportId::from_hash_of("dmd");
        let source = ViewportId::from_hash_of("src");
        let mut funnel = KeyFunnel::new().to_viewport(target);

        let mut src_in = raw(source, vec![key(Key::Space, true)]);
        funnel.input_hook(&Context::default(), &mut src_in);
        assert!(keys(&src_in.events).is_empty());

        // ROOT is not the target here → it must NOT receive the funneled keys.
        let mut root_in = raw(ViewportId::ROOT, vec![]);
        funnel.input_hook(&Context::default(), &mut root_in);
        assert!(keys(&root_in.events).is_empty());

        let mut target_in = raw(target, vec![]);
        funnel.input_hook(&Context::default(), &mut target_in);
        assert_eq!(keys(&target_in.events).len(), 1);
    }
}
