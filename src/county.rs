//! The county: the whole of Douglas County at half a mile to the tile, the
//! wide view. Zoom with the wheel, drag to pan; click a neighbor to deal with
//! them, your own claim to go home, a town to ride in.

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;
use bleeding_kansas::sim::geography::{self, PLACES};
use bleeding_kansas::sim::psyche::{self, Condition};
use bleeding_kansas::sim::world::{Faction, NpcId, PLAYER};

use crate::cmds;
use crate::scenery;
use crate::ui::{Menu, Overlay, blocking};
use crate::{
    Fonts, INK_GREEN, OXBLOOD, SLATE, Screen, Selection, Sim, TILE, TownId, tile_to_world,
};

/// The map's center, and how far out the camera starts.
pub const MAP_CENTER: Vec2 = Vec2::new(-240.0, 104.0);
pub const WIDE: f32 = 0.68;

#[derive(Component)]
pub struct NpcSprite {
    pub id: NpcId,
    pub phase: f32,
}

/// A pale light over a restless grave. Pooled; the sim decides where.
#[derive(Component)]
pub struct Wisp(pub usize);

pub fn setup(mut commands: Commands, sim: Res<Sim>, fonts: Res<Fonts>) {
    let world = &sim.0;

    for p in PLACES {
        let at = tile_to_world(geography::to_tile(p.at));
        let name = if p.approximate {
            format!("{} (approx.)", p.name)
        } else {
            p.name.to_string()
        };
        commands.spawn((
            Text2d::new(name),
            TextFont {
                font: fonts.display.clone(),
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(0.10, 0.09, 0.07)),
            Transform::from_translation((at + Vec2::new(0.0, 10.0)).extend(3.0)),
        ));
    }
    commands.spawn((
        Text2d::new("DELAWARE LANDS"),
        TextFont {
            font: fonts.display.clone(),
            font_size: 14.0,
            ..default()
        },
        TextColor(INK_GREEN),
        Transform::from_translation(tile_to_world((30, 1)).extend(3.0)),
    ));
    commands.spawn((
        Text2d::new("Kansas River"),
        TextFont {
            font: fonts.display.clone(),
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.10, 0.22, 0.40)),
        Transform::from_translation(tile_to_world((52, 5)).extend(3.0)),
    ));
    commands.spawn((
        Text2d::new("Santa Fe Trail"),
        TextFont {
            font: fonts.display.clone(),
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.30, 0.20, 0.10)),
        Transform::from_translation(tile_to_world((8, 36)).extend(3.0)),
    ));

    for f in &world.families {
        let at = tile_to_world(f.farm);
        let edge = match f.faction {
            Faction::FreeState => SLATE,
            Faction::ProSlavery => OXBLOOD,
        };
        // The claim's side, as a rule under its name.
        commands.spawn((
            Sprite {
                color: edge,
                custom_size: Some(Vec2::new(18.0, 2.0)),
                ..default()
            },
            Transform::from_translation((at + Vec2::new(0.0, -22.0)).extend(3.0)),
        ));
        let label = if f.id == 0 {
            "You".to_string()
        } else if f.store {
            "Dunmore's".to_string()
        } else {
            f.surname.to_string()
        };
        commands.spawn((
            Text2d::new(label),
            TextFont {
                font: fonts.body.clone(),
                font_size: 11.0,
                ..default()
            },
            TextColor(Color::srgb(0.08, 0.07, 0.06)),
            Transform::from_translation((at + Vec2::new(0.0, -15.0)).extend(3.0)),
        ));
    }

    for i in 0..16 {
        commands.spawn((
            Sprite {
                color: Color::srgba(0.85, 0.92, 1.0, 0.0),
                custom_size: Some(Vec2::splat(13.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 4.0)),
            Wisp(i),
        ));
    }
}

/// Everyone gets a figure, including kin who arrive and babies born later.
pub fn spawn_figures(
    mut commands: Commands,
    sim: Res<Sim>,
    art: Res<scenery::Art>,
    mut spawned: Local<usize>,
) {
    let world = &sim.0;
    while *spawned < world.npcs.len() {
        let i = *spawned;
        let id = world.npcs[i].id;
        commands.spawn((
            Sprite {
                image: scenery::figure(&art, world, id),
                color: Color::WHITE,
                custom_size: Some(if id == PLAYER {
                    Vec2::new(8.0, 16.0)
                } else {
                    Vec2::new(6.0, 12.0)
                }),
                ..default()
            },
            Transform::from_translation(tile_to_world(world.farm_of(id)).extend(2.0)),
            NpcSprite {
                id,
                phase: i as f32 * 2.399,
            },
        ));
        *spawned += 1;
    }
}

pub fn draw_npcs(
    time: Res<Time>,
    sim: Res<Sim>,
    art: Res<scenery::Art>,
    selection: Res<Selection>,
    mut sprites: Query<(&NpcSprite, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let world = &sim.0;
    let t = time.elapsed_secs();
    for (s, mut transform, mut sprite, mut vis) in &mut sprites {
        let npc = world.npc(s.id);
        if npc.adopted_by.is_some() || npc.departed {
            *vis = Visibility::Hidden;
            continue;
        }
        // Children grow into men and women.
        let img = scenery::figure(&art, world, s.id);
        if sprite.image != img {
            sprite.image = img;
        }
        let home = tile_to_world(world.farm_of(s.id)) + Vec2::new(0.0, -4.0);
        let wander = if npc.alive {
            Vec2::new((t * 0.15 + s.phase).sin(), (t * 0.11 + s.phase * 1.7).cos()) * TILE * 0.9
        } else {
            Vec2::ZERO
        };
        transform.translation = (home + wander).extend(2.0);
        sprite.color = if !npc.alive {
            Color::srgb(0.1, 0.1, 0.1)
        } else if selection.0 == Some(s.id) {
            Color::srgb(1.0, 1.0, 0.3)
        } else if s.id == PLAYER {
            Color::WHITE
        } else if matches!(
            psyche::condition(world, s.id),
            Condition::Starving | Condition::Grieving | Condition::Wounded
        ) {
            Color::srgb(0.95, 0.30, 0.20)
        } else {
            // Opinion of you: cold blue to warm tan.
            let o = (world.opinion(s.id, PLAYER) as f32 + 100.0) / 200.0;
            Color::srgb(0.35 + 0.6 * o, 0.45 + 0.35 * o, 0.85 - 0.5 * o)
        };
    }
}

/// Graves where people were killed; wisps over the ones still restless.
pub fn draw_spirits(
    mut commands: Commands,
    time: Res<Time>,
    sim: Res<Sim>,
    fonts: Res<Fonts>,
    mut graves: Local<usize>,
    mut wisps: Query<(&Wisp, &mut Transform, &mut Sprite)>,
) {
    let world = &sim.0;
    let haunts = &world.ghosts.haunts;
    while *graves < haunts.len() {
        let h = &haunts[*graves];
        let jitter = Vec2::new((*graves % 3) as f32 * 4.0 - 4.0, 6.0);
        commands.spawn((
            Text2d::new("\u{2020}"),
            TextFont {
                font: fonts.body.clone(),
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::srgb(0.12, 0.05, 0.04)),
            Transform::from_translation((tile_to_world(h.site) + jitter).extend(3.5)),
        ));
        *graves += 1;
    }
    let t = time.elapsed_secs();
    let restless: Vec<_> = haunts.iter().filter(|h| h.restless).collect();
    for (w, mut tf, mut sprite) in &mut wisps {
        match restless.get(w.0) {
            Some(h) => {
                let phase = w.0 as f32 * 1.7;
                let drift = Vec2::new(
                    (t * 0.4 + phase).sin() * 9.0,
                    (t * 0.7 + phase).cos() * 5.0 + 8.0,
                );
                tf.translation = (tile_to_world(h.site) + drift).extend(6.2);
                // Brighter under a full moon.
                let glow = 0.5 + 0.3 * world.day.moonlight() + 0.15 * (t * 1.3 + phase).sin();
                sprite.color = Color::srgba(0.92, 0.97, 1.0, glow.clamp(0.3, 0.95));
            }
            None => sprite.color = Color::srgba(0.85, 0.92, 1.0, 0.0),
        }
    }
}

/// Where the county camera was, so coming back puts you where you left.
#[derive(Resource)]
pub struct CountyView {
    pub at: Vec2,
    pub scale: f32,
}

impl Default for CountyView {
    fn default() -> Self {
        CountyView {
            at: MAP_CENTER,
            scale: WIDE,
        }
    }
}

pub fn enter(
    view: Res<CountyView>,
    mut cam: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    if let Ok((mut tf, mut proj)) = cam.single_mut() {
        tf.translation = view.at.extend(tf.translation.z);
        if let Projection::Orthographic(o) = proj.as_mut() {
            o.scale = view.scale;
        }
    }
}

/// Wheel to zoom, right-drag or WASD to pan; left click to pick.
#[allow(clippy::too_many_arguments)]
pub fn control(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    wheel: Res<AccumulatedMouseScroll>,
    motion: Res<AccumulatedMouseMotion>,
    time: Res<Time>,
    menu: Res<Menu>,
    overlay: Res<Overlay>,
    mut view: ResMut<CountyView>,
    mut cam: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    let Ok((mut tf, mut proj)) = cam.single_mut() else {
        return;
    };
    tf.translation = view.at.extend(tf.translation.z);
    if let Projection::Orthographic(o) = proj.as_mut() {
        o.scale = view.scale;
    }
    if blocking(&menu, &overlay) {
        return;
    }
    if wheel.delta.y != 0.0 {
        view.scale = (view.scale * (1.0 - 0.12 * wheel.delta.y.signum())).clamp(0.22, 0.9);
    }
    let mut pan = Vec2::ZERO;
    if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        pan.x -= 1.0;
    }
    if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        pan.x += 1.0;
    }
    if keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        pan.y += 1.0;
    }
    if keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        pan.y -= 1.0;
    }
    let scale = view.scale;
    view.at += pan * 400.0 * scale * time.delta_secs();
    if mouse.pressed(MouseButton::Right) {
        view.at += Vec2::new(-motion.delta.x, motion.delta.y) * scale;
    }
    view.at = view.at.clamp(
        MAP_CENTER - Vec2::new(400.0, 260.0),
        MAP_CENTER + Vec2::new(400.0, 260.0),
    );
    tf.translation = view.at.extend(tf.translation.z);
    if let Projection::Orthographic(o) = proj.as_mut() {
        o.scale = view.scale;
    }
    if keys.just_pressed(KeyCode::Home) {
        *view = CountyView::default();
    }
}

/// A click on the map: a neighbor, your claim, or a town.
#[allow(clippy::too_many_arguments)]
pub fn click(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    sprites: Query<(&GlobalTransform, &NpcSprite)>,
    sim: Res<Sim>,
    mut selection: ResMut<Selection>,
    mut menu: ResMut<Menu>,
    overlay: Res<Overlay>,
    mut next: ResMut<NextState<Screen>>,
    mut town: ResMut<TownId>,
) {
    if !buttons.just_pressed(MouseButton::Left) || blocking(&menu, &overlay) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if cursor.y < 32.0 {
        return; // the top bar
    }
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };
    let Ok(point) = camera.viewport_to_world_2d(camera_transform, cursor) else {
        return;
    };
    let world = &sim.0;
    let who = sprites
        .iter()
        .filter(|(_, s)| s.id != PLAYER && world.npc(s.id).alive && !world.npc(s.id).departed)
        .map(|(t, s)| (t.translation().truncate().distance(point), s.id))
        .filter(|(d, _)| *d < 10.0)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, id)| id);
    if let Some(id) = who {
        selection.0 = Some(id);
        menu.open(cmds::neighbor(world, id));
        return;
    }
    if tile_to_world(world.families[0].farm).distance(point) < 16.0 {
        next.set(Screen::Claim);
        return;
    }
    for (t, name) in [
        (TownId::Lawrence, "Lawrence"),
        (TownId::Franklin, "Franklin"),
        (TownId::Lecompton, "Lecompton"),
    ] {
        if let Some(p) = PLACES.iter().find(|p| p.name == name)
            && tile_to_world(geography::to_tile(p.at)).distance(point) < 18.0
        {
            *town = t;
            next.set(Screen::Town);
            return;
        }
    }
    selection.0 = None;
}
