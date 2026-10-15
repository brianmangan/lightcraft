//! ⌫ in the grid asks before moving photos to Recently Deleted; Return confirms, Esc cancels,
//! Shift+⌫ moves them without asking, except from inside an album.

use std::time::Duration;

use lightcraft_catalog::PhotoId;
use serde_json::{Value, json};

use crate::LightcraftApp;
use crate::headless::Headless;
use crate::state::Dialog;

const SETTLE: Duration = Duration::from_secs(120);
const T: Duration = Duration::from_secs(10);

/// The demo library's grid with its first photo selected and ⌫ pressed; returns that photo's id.
fn asking() -> (Headless, u64) {
    let services = crate::Services { png: None, ..Default::default() };
    let mut app = LightcraftApp::new(lightcraft_engine::Session::with_demo(), services);
    app.ui.view = crate::state::ViewMode::PhotoGrid;
    let mut h = Headless::new(app, [1200.0, 900.0], 1.0);
    let id = h.app.session.visible_cloned()[0].0;
    let r = h.request("engine.execute", json!({"command": "library.select", "params": {"ids": [id]}}), T);
    assert_eq!(r["ok"], true, "{r}");
    key(&mut h, json!({"key": "delete"}));
    assert_eq!(h.app.ui.dialog, Some(Dialog::ConfirmDelete { count: 1 }));
    (h, id)
}

fn key(h: &mut Headless, k: Value) {
    let r = h.request("ui.key", k.clone(), T);
    assert_eq!(r["ok"], true, "{k}: {r}");
    h.settle(SETTLE);
}

fn deleted(h: &Headless, id: u64) -> bool {
    h.app.session.catalog.photo(PhotoId(id)).unwrap().deleted
}

#[test]
fn return_confirms_the_delete() {
    let (mut h, id) = asking();
    key(&mut h, json!({"key": "Enter"}));
    assert!(h.app.ui.dialog.is_none(), "Return closes the dialog");
    assert!(deleted(&h, id), "Return deletes the photo");
}

#[test]
fn escape_cancels_the_delete() {
    let (mut h, id) = asking();
    key(&mut h, json!({"key": "Escape"}));
    assert!(h.app.ui.dialog.is_none(), "Esc closes the dialog");
    assert!(!deleted(&h, id), "Esc deletes nothing");
}

#[test]
fn shift_delete_skips_the_question() {
    let (mut h, id) = asking();
    key(&mut h, json!({"key": "Escape"}));
    key(&mut h, json!({"key": "delete", "shift": true}));
    assert!(h.app.ui.dialog.is_none(), "Shift+⌫ doesn't ask");
    assert!(deleted(&h, id), "Shift+⌫ deletes the photo");
}

/// On Windows the windowing layer turns ⇧Delete into a Cut event instead of a key press.
#[test]
fn windows_shift_delete_cut_event_skips_the_question() {
    let (mut h, id) = asking();
    key(&mut h, json!({"key": "Escape"}));
    h.events.extend([egui::Event::ModifiersChanged(egui::Modifiers::SHIFT), egui::Event::Cut]);
    h.settle(SETTLE);
    assert!(h.app.ui.dialog.is_none(), "⇧Delete doesn't ask");
    assert!(deleted(&h, id), "⇧Delete deletes the photo");
}

#[test]
fn shift_delete_does_nothing_in_an_album() {
    let (mut h, id) = asking();
    key(&mut h, json!({"key": "Escape"}));
    let album = h.app.run("album.create", json!({"name": "Keepers"})).unwrap()["id"].as_u64().unwrap();
    let r = h.request("engine.execute", json!({"command": "library.source", "params": {"kind": "album", "id": album}}), T);
    assert_eq!(r["ok"], true, "{r}");
    let r = h.request("engine.execute", json!({"command": "library.select", "params": {"ids": [id]}}), T);
    assert_eq!(r["ok"], true, "{r}");
    key(&mut h, json!({"key": "delete", "shift": true}));
    assert!(h.app.ui.dialog.is_none(), "Shift+⌫ doesn't ask");
    assert!(!deleted(&h, id), "Shift+⌫ deletes nothing from inside an album");
}

/// Delete Photo Without Asking is a keymap command (Shift+⌫ by default), so it can be rebound.
#[test]
fn delete_without_asking_can_be_rebound() {
    let b = crate::shortcuts::find_bindable("photo.deleteNow").expect("bindable");
    assert_eq!(b.default, Some("Shift+Delete"));
    let (mut h, id) = asking();
    key(&mut h, json!({"key": "Escape"}));
    crate::shortcuts::assign(&mut h.app.ui.settings.keymap, "photo.deleteNow", Some("F9")).unwrap();
    key(&mut h, json!({"key": "delete", "shift": true}));
    assert!(!deleted(&h, id), "the old key no longer deletes");
    key(&mut h, json!({"key": "F9"}));
    assert!(h.app.ui.dialog.is_none(), "the new key doesn't ask");
    assert!(deleted(&h, id), "the new key deletes the photo");
}
