//! Anyinstaller Packager GUI: author AIP v1 config and build installers in one place.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]

mod app;
mod build_job;
mod config_file;
mod detect;
mod market;
mod model;
mod settings;
mod ui;
mod validation;

use rune::{App, AppSpec, ColorScheme};
use ui::view;

fn scene_app(state: &mut app::State) {
    state.step = model::Step::App;
}

fn scene_defaults(state: &mut app::State) {
    state.step = model::Step::Defaults;
}

fn scene_market(state: &mut app::State) {
    state.step = model::Step::Market;
    state.region_status = model::RegionStatus::Overseas {
        country_code: "HK".into(),
        country: "Hong Kong".into(),
    };
    if let Ok(text) = std::fs::read_to_string("../client-backend-marketplace/catalog.toml") {
        if let Ok(catalog) = toml::from_str(&text) {
            state.catalog = Some(catalog);
        }
    }
}

fn scene_settings(state: &mut app::State) {
    scene_market(state);
    state.step = model::Step::Settings;
    state.region_status = model::RegionStatus::Domestic {
        country_code: "CN".into(),
        country: "China".into(),
    };
    state.accelerator_enabled = true;
}

fn scene_inputs(state: &mut app::State) {
    state.step = model::Step::Inputs;
}

fn scene_package(state: &mut app::State) {
    state.step = model::Step::Package;
}

fn scene_pfx(state: &mut app::State) {
    state.step = model::Step::Package;
    state.sign_mode = model::SignMode::Pfx;
    state.pfx_edit.replace_text(r"D:\keys\release.pfx");
    state
        .thumbprint_edit
        .replace_text("0123456789ABCDEF0123456789ABCDEF01234567");
}

fn main() {
    let spec = AppSpec::<app::State> {
        min_size: Some((1080.0, 680.0)),
        setup: Some(app::setup),
        poll: Some(app::poll),
        families: ("Microsoft YaHei UI, Inter", "Cascadia Mono, Consolas"),
        scenes: &[
            ("app", scene_app),
            ("defaults", scene_defaults),
            ("market", scene_market),
            ("settings", scene_settings),
            ("inputs", scene_inputs),
            ("package", scene_package),
            ("pfx", scene_pfx),
        ],
        ..AppSpec::new("Anyinstaller Packager", (1280.0, 800.0), app::init, view)
    };
    App::new(spec).color_scheme(ColorScheme::Dark).run();
}
