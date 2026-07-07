//! Two-viewport demo of [`egui_keyfunnel::KeyFunnel`].
//!
//! A ROOT window and a SECONDARY "cover" window (created with
//! `with_active(false)`, like a kiosk backglass). Click SECONDARY — or alt-tab
//! to it — to give it focus, then type: the keys still show up in **ROOT's**
//! log, because `KeyFunnel` moves them there. SECONDARY's own log stays empty.
//!
//! Run:  cargo run --example funnel_demo

use std::sync::{Arc, Mutex};

use eframe::egui;
use egui::ViewportId;

const SEC_ID: &str = "keyfunnel-demo-secondary";

#[derive(Default)]
struct Log {
    root: Vec<String>,
    secondary: Vec<String>,
}

impl Log {
    fn push(list: &mut Vec<String>, s: String) {
        list.push(s);
        if list.len() > 40 {
            list.remove(0);
        }
    }
}

struct Demo {
    log: Arc<Mutex<Log>>,
}

/// Record the `Key` presses this viewport actually received into `sink`.
fn record_keys(ui: &egui::Ui, sink: &mut Vec<String>) {
    ui.input(|i| {
        for event in &i.events {
            if let egui::Event::Key {
                key, pressed: true, ..
            } = event
            {
                Log::push(sink, format!("{key:?}"));
            }
        }
    });
}

impl eframe::App for Demo {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        ctx.request_repaint();

        {
            let mut log = self.log.lock().unwrap();
            record_keys(ui, &mut log.root);
        }

        ui.heading("egui-keyfunnel demo");
        ui.label(
            "Give the SECONDARY window focus (click it / alt-tab), then type.\n\
             The keys still land here, in ROOT — that's the funnel.",
        );
        ui.separator();
        let root_focused = ctx.input(|i| i.viewport().focused).unwrap_or(false);
        ui.label(format!(
            "ROOT focused: {}",
            if root_focused { "yes" } else { "no" }
        ));
        ui.strong("Keys received by ROOT:");
        egui::ScrollArea::vertical()
            .id_salt("root_log")
            .max_height(220.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                let log = self.log.lock().unwrap();
                ui.monospace(log.root.join("  "));
            });

        // ── SECONDARY cover viewport ──
        let sec_id = ViewportId::from_hash_of(SEC_ID);
        ctx.request_repaint_of(sec_id);
        let log = self.log.clone();
        ctx.show_viewport_deferred(
            sec_id,
            egui::ViewportBuilder::default()
                .with_title("SECONDARY — steal focus, then type")
                .with_inner_size([420.0, 300.0])
                .with_active(false),
            move |ui, _class| {
                let ctx = ui.ctx().clone();
                {
                    let mut log = log.lock().unwrap();
                    record_keys(ui, &mut log.secondary);
                }
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.heading("SECONDARY (cover)");
                    let focused = ctx.input(|i| i.viewport().focused).unwrap_or(false);
                    ui.label(format!("focused: {}", if focused { "yes" } else { "no" }));
                    ui.label("Type while I'm focused — the keys go to ROOT, not here.");
                    ui.separator();
                    ui.strong("Keys received by SECONDARY (should stay empty):");
                    let log = log.lock().unwrap();
                    ui.monospace(log.secondary.join("  "));
                });
            },
        );
    }
}

fn main() -> eframe::Result<()> {
    let log = Arc::new(Mutex::new(Log::default()));
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ROOT — egui-keyfunnel")
            .with_inner_size([520.0, 460.0]),
        ..Default::default()
    };
    eframe::run_native(
        "egui-keyfunnel demo",
        native_options,
        Box::new(|cc| {
            // The whole point: register the plugin once.
            cc.egui_ctx.add_plugin(egui_keyfunnel::KeyFunnel::new());
            Ok(Box::new(Demo { log }))
        }),
    )
}
