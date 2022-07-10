use bevy::prelude::{AssetServer, Res, ResMut};
use bevy_egui::{egui::CentralPanel, EguiContext};
use puffin_egui::puffin::GlobalProfiler;
use rfd::FileDialog;

use crate::{asset::RocksmithAsset, Phase, State};

/// The UI for showing profiling information.
pub fn ui(mut context: ResMut<EguiContext>) {
    // Update the profiler, should be called once per frame
    GlobalProfiler::lock().new_frame();

    puffin_egui::profiler_window(context.ctx_mut());
}
