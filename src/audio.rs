//! The score: one piece in four stems at the same tempo and length, looped
//! together, the mix set by the county. Nobody hears the war music start;
//! they hear the county get louder the way it gets worse (docs/ART.md).
//!
//! Stems are optional: `assets/audio/stem_<name>.ogg`. Missing ones are
//! skipped. They start together only once every one is loaded, so the
//! loops stay locked to the bar. `BK_MUTE=1` silences them.

use bevy::audio::{AudioSinkPlayback, Volume};
use bevy::prelude::*;

use crate::Sim;

/// drone: always under everything. folk: the calm county. fuzz: trouble.
/// drums: the county at war.
pub const STEMS: [&str; 4] = ["drone", "folk", "fuzz", "drums"];

#[derive(Component)]
pub struct Stem(usize);

/// Handles waiting to load, then the level each stem is easing toward.
#[derive(Resource, Default)]
pub struct Score {
    pending: Vec<(usize, Handle<AudioSource>)>,
    started: bool,
    level: [f32; 4],
}

pub fn setup(mut score: ResMut<Score>, assets: Res<AssetServer>) {
    if std::env::var("BK_MUTE").is_ok_and(|v| v != "0") {
        return;
    }
    score.pending = STEMS
        .iter()
        .enumerate()
        .filter(|(_, n)| crate::asset_exists(&format!("audio/stem_{n}.ogg")))
        .map(|(i, n)| (i, assets.load(format!("audio/stem_{n}.ogg"))))
        .collect();
}

/// Start every stem on the same frame, silent, once they're all in.
pub fn start(mut commands: Commands, mut score: ResMut<Score>, assets: Res<AssetServer>) {
    if score.started || score.pending.is_empty() {
        return;
    }
    if !score
        .pending
        .iter()
        .all(|(_, h)| assets.is_loaded_with_dependencies(h))
    {
        return;
    }
    for (i, h) in score.pending.drain(..) {
        commands.spawn((
            AudioPlayer::new(h),
            PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)),
            Stem(i),
        ));
    }
    score.started = true;
}

/// What each stem should be at, from the county's state.
pub fn targets(world: &bleeding_kansas::sim::World) -> [f32; 4] {
    let smooth = |a: f32, b: f32, x: f32| {
        let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    // 0 a quiet county .. 1 the Wakarusa War.
    let tension = ((world.grievance[0] + world.grievance[1]) as f32 / 200.0).clamp(0.0, 1.2);
    // Fresh blood brings the drums in on its own, and they fade over a week.
    let since_killing = world
        .events
        .iter()
        .rev()
        .take_while(|e| world.day.0.saturating_sub(e.day.0) <= 7)
        .filter(|e| {
            matches!(
                e.kind,
                bleeding_kansas::sim::EventKind::Death {
                    killer: Some(_),
                    ..
                }
            )
        })
        .map(|e| world.day.0 - e.day.0)
        .min();
    let blood = since_killing.map_or(0.0, |d| 1.0 - d as f32 / 8.0);
    let night = 1.0 - world.night_light(world.day).min(1.0);
    [
        0.55 + 0.25 * night,
        (1.0 - smooth(0.35, 0.8, tension)) * (0.85 - 0.25 * night),
        smooth(0.45, 0.85, tension),
        smooth(0.8, 1.1, tension).max(blood * 0.8),
    ]
}

pub fn mix(
    time: Res<Time>,
    sim: Res<Sim>,
    mut score: ResMut<Score>,
    mut sinks: Query<(&Stem, &mut AudioSink)>,
) {
    if !score.started {
        return;
    }
    let want = targets(&sim.0);
    // Ease, never jump: a stem takes about four seconds to come all the way in.
    let step = time.delta_secs() / 4.0;
    for (i, w) in want.iter().enumerate() {
        let l = &mut score.level[i];
        *l += (w - *l).clamp(-step, step);
    }
    for (s, mut sink) in &mut sinks {
        sink.set_volume(Volume::Linear(score.level[s.0]));
    }
}
