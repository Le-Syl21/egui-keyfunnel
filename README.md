# egui-keyfunnel

[![crates.io](https://img.shields.io/crates/v/egui-keyfunnel.svg)](https://crates.io/crates/egui-keyfunnel)
[![docs.rs](https://docs.rs/egui-keyfunnel/badge.svg)](https://docs.rs/egui-keyfunnel)

**Funnel keyboard input to a single [egui](https://github.com/emilk/egui) viewport.**

An `egui::Plugin` for multi-viewport apps — kiosks, pinball cabinets, digital
signage — where a **main** window shares the screen with **secondary** windows
(backglass, DMD, topper, status displays) that must never steal the keyboard.

## The problem

In a multi-viewport egui app, the OS decides which window has keyboard focus.
On Wayland especially, a compositor can **ignore `ViewportBuilder::with_active(false)`**
and hand focus to a freshly-mapped secondary window. Keystrokes then land in that
viewport — whose UI ignores them — and your main viewport never sees the keys.
Your pincab's flipper keys, your kiosk's shortcuts: dead, until the user manually
clicks the main window.

## The fix

Register `KeyFunnel` once. It moves keyboard events out of the secondary
viewports and into a single **target** viewport (the root by default), no matter
which window the compositor focused:

```rust
use egui_keyfunnel::KeyFunnel;

// e.g. in eframe's creation closure:
cc.egui_ctx.add_plugin(KeyFunnel::new());
```

That's it. Keyboard input always reaches your main UI — **no window-manager
tricks, no focus juggling, no backend fork.**

### Configuring

```rust
use egui_keyfunnel::KeyFunnel;
use egui::ViewportId;

let dmd = ViewportId::from_hash_of("dmd");
let backglass = ViewportId::from_hash_of("backglass");

KeyFunnel::new()
    .to_viewport(ViewportId::ROOT)        // where the keys should land (default: root)
    .from_viewports([dmd, backglass]);    // pull only from these (default: all non-target)
```

## What it does *not* touch

Only **keyboard** events (`Event::Key`, `Event::Text`, `Event::Ime`) are moved.

- **Pointer / touch / scroll** stay on the window they happen over — a click both
  focuses and interacts with that window, which is what you want.
- **Gamepad / joystick** input read outside egui (e.g. via SDL) is unaffected: it
  is delivered at the device level, independent of window focus.

## How it works

`KeyFunnel` implements `egui::Plugin`. In `input_hook`, called once per viewport
per frame with that viewport's `RawInput`:

- on a **source** viewport, it removes the keyboard events and buffers them;
- on the **target** viewport, it prepends the buffered events.

Because a single plugin instance is shared across all of a `Context`'s viewport
passes, the events simply move from one `RawInput` to another. Pure egui, public
API only.

## Compatibility

Each release targets **one egui minor** (egui's `Plugin` trait can change between
minors).

| egui-keyfunnel | egui   |
|----------------|--------|
| `0.1.x`        | `0.35` |

## License

Dual-licensed under MIT or Apache-2.0.
