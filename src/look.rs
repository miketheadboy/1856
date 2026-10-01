//! The look: one fullscreen pass over everything, sprites and UI alike, so
//! the county reads as printed matter instead of pixels: paper fiber and
//! grain, ink that bleeds a hair into the paper, the oxblood and brass
//! plates a little off register, a sepia vignette, saturation down.
//!
//! Every number lives in `assets/look.txt` (`key = value`, `#` comments),
//! re-read while the game runs, so it can be tuned against a frame the way
//! an adjustment layer is. The county moves some of them: tension pushes the
//! registration and the grain (the press running ragged), night pulls the
//! vignette in. `BK_LOOK=0` turns the pass off. docs/ART.md has the why of
//! each knob.

use bevy::core_pipeline::core_2d::graph::Node2d;
use bevy::core_pipeline::fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin};
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_graph::{InternedRenderLabel, RenderLabel};
use bevy::render::render_resource::ShaderType;
use bevy::shader::ShaderRef;
use bevy::ui_render::graph::NodeUi;

use crate::Sim;

/// The uniform the shader reads. Field order is the WGSL struct's order.
#[derive(Component, ExtractComponent, Clone, Copy, ShaderType, Debug)]
pub struct Look {
    /// Seconds, stepped at `boil_fps` so the grain changes like printed
    /// frames, not like video noise.
    pub time: f32,
    pub grain: f32,
    pub grain_size: f32,
    pub fiber: f32,
    pub bleed: f32,
    pub misreg: f32,
    pub vignette: f32,
    pub sepia: f32,
    pub desat: f32,
    pub halftone: f32,
    pub tension: f32,
    pub night: f32,
    pub boil_fps: f32,
    pub tension_misreg: f32,
    pub tension_grain: f32,
    pub night_vignette: f32,
}

impl Default for Look {
    fn default() -> Self {
        Look {
            time: 0.0,
            grain: 0.14,
            grain_size: 1.6,
            fiber: 0.07,
            bleed: 0.15,
            misreg: 1.5,
            vignette: 0.35,
            sepia: 0.22,
            desat: 0.12,
            halftone: 0.0,
            tension: 0.0,
            night: 0.0,
            boil_fps: 8.0,
            tension_misreg: 1.5,
            tension_grain: 0.08,
            night_vignette: 0.3,
        }
    }
}

impl FullscreenMaterial for Look {
    fn fragment_shader() -> ShaderRef {
        "shaders/look.wgsl".into()
    }

    // After the UI is drawn, before the frame goes to the window: the look
    // is over the menus and the type too, the way ink sits on the whole page.
    fn node_edges() -> Vec<InternedRenderLabel> {
        vec![
            NodeUi::UiPass.intern(),
            Self::node_label().intern(),
            Node2d::Upscaling.intern(),
        ]
    }
}

pub struct LookPlugin;

impl Plugin for LookPlugin {
    fn build(&self, app: &mut App) {
        if std::env::var("BK_LOOK").is_ok_and(|v| v == "0") {
            return;
        }
        app.add_plugins(FullscreenMaterialPlugin::<Look>::default())
            .init_resource::<Tuning>()
            .add_systems(Update, (reload, drive).chain());
    }
}

/// The last-read `look.txt`, and when it was read.
#[derive(Resource, Default)]
pub struct Tuning {
    base: Look,
    stamp: Option<std::time::SystemTime>,
    checked: f32,
}

fn path() -> std::path::PathBuf {
    let root = std::env::var("BEVY_ASSET_ROOT")
        .or_else(|_| std::env::var("CARGO_MANIFEST_DIR"))
        .map(std::path::PathBuf::from)
        .unwrap_or_default();
    root.join("assets").join("look.txt")
}

/// Read `key = value` lines over the defaults. Unknown keys are ignored.
pub fn parse(text: &str) -> Look {
    let mut l = Look::default();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let Ok(v) = v.trim().parse::<f32>() else {
            continue;
        };
        let slot = match k.trim() {
            "grain" => &mut l.grain,
            "grain_size" => &mut l.grain_size,
            "fiber" => &mut l.fiber,
            "bleed" => &mut l.bleed,
            "misreg" => &mut l.misreg,
            "vignette" => &mut l.vignette,
            "sepia" => &mut l.sepia,
            "desat" => &mut l.desat,
            "halftone" => &mut l.halftone,
            "boil_fps" => &mut l.boil_fps,
            "tension_misreg" => &mut l.tension_misreg,
            "tension_grain" => &mut l.tension_grain,
            "night_vignette" => &mut l.night_vignette,
            _ => continue,
        };
        *slot = v;
    }
    l
}

/// Put the look on the camera, and re-read the file when it changes.
fn reload(
    mut commands: Commands,
    time: Res<Time>,
    mut tuning: ResMut<Tuning>,
    cams: Query<Entity, (With<Camera2d>, Without<Look>)>,
) {
    for cam in &cams {
        commands.entity(cam).insert(tuning.base);
    }
    tuning.checked += time.delta_secs();
    if tuning.checked < 1.0 && tuning.stamp.is_some() {
        return;
    }
    tuning.checked = 0.0;
    let p = path();
    let stamp = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
    if stamp.is_some() && stamp != tuning.stamp {
        if let Ok(text) = std::fs::read_to_string(&p) {
            tuning.base = parse(&text);
        }
        tuning.stamp = stamp;
    } else if tuning.stamp.is_none() {
        // No file: defaults, and don't stat it every frame.
        tuning.stamp = Some(std::time::SystemTime::UNIX_EPOCH);
    }
}

/// The county's state into the uniform: tension and night.
fn drive(time: Res<Time>, sim: Res<Sim>, tuning: Res<Tuning>, mut looks: Query<&mut Look>) {
    let w = &sim.0;
    let tension = ((w.grievance[0] + w.grievance[1]) as f32 / 200.0).clamp(0.0, 1.0);
    let night = 1.0 - w.night_light(w.day).min(1.0);
    for mut l in &mut looks {
        *l = tuning.base;
        l.time = time.elapsed_secs();
        l.tension = tension;
        l.night = night;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_overrides_the_defaults_and_ignores_the_rest() {
        let l = parse("grain = 0.3 # heavier\nmisreg=4\nnonsense = 9\n# sepia = 1\n");
        assert_eq!(l.grain, 0.3);
        assert_eq!(l.misreg, 4.0);
        assert_eq!(l.sepia, Look::default().sepia);
    }
}
