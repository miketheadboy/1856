//! Bevy view over the headless sim (§21 step 2). This file draws the world
//! and forwards clicks; every rule lives in `bleeding_kansas::sim`.

use bevy::prelude::*;
use bleeding_kansas::sim::chronicle::{self, suspect_label};
use bleeding_kansas::sim::psyche::{self, Condition};
use bleeding_kansas::sim::world::{FamilyId, NpcId, PLAYER, World};

const SEED: u64 = 1856;
const GRID_SIZE: i32 = 30;
const TILE_SIZE: f32 = 22.0;
const MAP_CENTER: Vec2 = Vec2::new(-190.0, 0.0);
const SECONDS_PER_DAY: f32 = 1.2;
const LOG_LINES: usize = 16;

#[derive(Resource)]
struct Sim(World);

#[derive(Resource, Default)]
struct Selection(Option<NpcId>);

#[derive(Resource)]
struct Clock {
    timer: Timer,
    paused: bool,
}

#[derive(Component)]
struct NpcSprite {
    id: NpcId,
    /// Where on the farm they are wandering, in tiles.
    offset: Vec2,
    phase: f32,
}

#[derive(Component)]
struct BarnSprite(FamilyId);

#[derive(Component)]
struct HeaderLabel;

#[derive(Component)]
struct InspectionLabel;

#[derive(Component)]
struct LogLabel;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Action {
    Kill,
    Burn,
    Tavern,
    Pause,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bleeding Kansas".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Sim(World::new(SEED)))
        .init_resource::<Selection>()
        .insert_resource(Clock {
            timer: Timer::from_seconds(SECONDS_PER_DAY, TimerMode::Repeating),
            paused: false,
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                advance_calendar,
                select_npc,
                handle_actions,
                wander,
                draw_npcs,
                draw_barns,
                update_panel,
            )
                .chain(),
        )
        .run();
}

fn tile_to_world(tile: (i32, i32)) -> Vec2 {
    let half = GRID_SIZE as f32 * TILE_SIZE / 2.0;
    MAP_CENTER
        + Vec2::new(
            tile.0 as f32 * TILE_SIZE - half + TILE_SIZE / 2.0,
            half - tile.1 as f32 * TILE_SIZE - TILE_SIZE / 2.0,
        )
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    sim: Res<Sim>,
) {
    commands.spawn(Camera2d);

    let size = GRID_SIZE as f32 * TILE_SIZE;
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(size, size))),
        MeshMaterial2d(materials.add(Color::srgb(0.30, 0.34, 0.20))),
        Transform::from_translation(MAP_CENTER.extend(0.0)),
    ));

    let world = &sim.0;
    for family in &world.families {
        commands.spawn((
            Sprite {
                color: Color::srgb(0.45, 0.26, 0.16),
                custom_size: Some(Vec2::splat(TILE_SIZE * 1.4)),
                ..default()
            },
            Transform::from_translation(tile_to_world(family.farm).extend(0.5)),
            BarnSprite(family.id),
        ));
    }

    for (i, npc) in world.npcs.iter().enumerate() {
        let angle = i as f32 * 2.399;
        commands.spawn((
            Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::splat(if npc.id == PLAYER { 14.0 } else { 11.0 })),
                ..default()
            },
            Transform::from_translation(tile_to_world(world.farm_of(npc.id)).extend(1.0)),
            NpcSprite {
                id: npc.id,
                offset: Vec2::new(angle.cos(), angle.sin()) * 1.5,
                phase: angle,
            },
        ));
    }

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                top: Val::Px(16.0),
                width: Val::Px(440.0),
                height: Val::Percent(95.0),
                padding: UiRect::all(Val::Px(14.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.05, 0.04, 0.94)),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("BLEEDING KANSAS"),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::srgb(0.88, 0.72, 0.42)),
            ));
            panel.spawn((
                Text::new(""),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.75, 0.66)),
                HeaderLabel,
            ));
            panel.spawn((
                Text::new("Click a neighbor"),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                // Fixed height so the buttons below don't jump around.
                Node {
                    min_height: Val::Px(84.0),
                    ..default()
                },
                InspectionLabel,
            ));
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|row| {
                    for (action, label, color) in [
                        (Action::Kill, "KILL", Color::srgb(0.40, 0.10, 0.08)),
                        (Action::Burn, "BURN BARN", Color::srgb(0.45, 0.25, 0.06)),
                        (Action::Tavern, "BE SEEN", Color::srgb(0.16, 0.25, 0.30)),
                        (Action::Pause, "PAUSE", Color::srgb(0.22, 0.22, 0.22)),
                    ] {
                        row.spawn((
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(10.0), Val::Px(7.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(color),
                            action,
                        ))
                        .with_child((
                            Text::new(label),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    }
                });
            panel.spawn((
                Text::new("WHAT PEOPLE ARE SAYING"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.72, 0.72, 0.62)),
            ));
            panel.spawn((
                Text::new(""),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.70, 0.76, 0.70)),
                LogLabel,
            ));
        });
}

fn advance_calendar(time: Res<Time>, mut clock: ResMut<Clock>, mut sim: ResMut<Sim>) {
    if clock.paused || !sim.0.player_alive() {
        return;
    }
    clock.timer.tick(time.delta());
    if clock.timer.just_finished() {
        sim.0.advance_day();
    }
}

fn select_npc(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    sprites: Query<(&GlobalTransform, &NpcSprite)>,
    sim: Res<Sim>,
    mut selection: ResMut<Selection>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };
    let Ok(point) = camera.viewport_to_world_2d(camera_transform, cursor) else {
        return;
    };
    // Clicks on the UI panel should not clear the selection.
    if point.x > MAP_CENTER.x + GRID_SIZE as f32 * TILE_SIZE / 2.0 {
        return;
    }
    selection.0 = sprites
        .iter()
        .filter(|(_, s)| s.id != PLAYER && sim.0.npc(s.id).alive)
        .map(|(t, s)| (t.translation().truncate().distance(point), s.id))
        .filter(|(d, _)| *d < 16.0)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, id)| id);
}

fn handle_actions(
    buttons: Query<(&Interaction, &Action), Changed<Interaction>>,
    selection: Res<Selection>,
    mut clock: ResMut<Clock>,
    mut sim: ResMut<Sim>,
) {
    for (interaction, action) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match (action, selection.0) {
            (Action::Kill, Some(target)) => {
                sim.0.player_kill(target);
            }
            (Action::Burn, Some(target)) => {
                sim.0.player_burn(target);
            }
            (Action::Tavern, _) => sim.0.player_go_to_tavern(),
            (Action::Pause, _) => clock.paused = !clock.paused,
            _ => {}
        }
    }
}

fn wander(time: Res<Time>, mut sprites: Query<&mut NpcSprite>) {
    let t = time.elapsed_secs();
    for mut s in &mut sprites {
        let p = s.phase;
        s.offset = Vec2::new((t * 0.07 + p).sin() * 1.6, (t * 0.05 + p * 1.7).cos() * 1.6);
    }
}

fn draw_npcs(
    sim: Res<Sim>,
    selection: Res<Selection>,
    mut sprites: Query<(&NpcSprite, &mut Transform, &mut Sprite)>,
) {
    let world = &sim.0;
    for (s, mut transform, mut sprite) in &mut sprites {
        let npc = world.npc(s.id);
        let home = tile_to_world(world.farm_of(s.id));
        let offset = if npc.alive { s.offset } else { Vec2::ZERO };
        transform.translation = (home + offset * TILE_SIZE).extend(1.0);
        sprite.color = if !npc.alive {
            Color::srgb(0.15, 0.15, 0.15)
        } else if selection.0 == Some(s.id) {
            Color::srgb(1.0, 1.0, 0.6)
        } else if s.id == PLAYER {
            Color::srgb(0.95, 0.95, 0.95)
        } else if matches!(
            psyche::condition(world, s.id),
            Condition::Starving | Condition::Grieving | Condition::Wounded
        ) {
            Color::srgb(0.88, 0.34, 0.22)
        } else {
            // Opinion of you, from cold blue to warm tan.
            let o = (world.opinion(s.id, PLAYER) as f32 + 100.0) / 200.0;
            Color::srgb(0.35 + 0.5 * o, 0.45 + 0.25 * o, 0.75 - 0.4 * o)
        };
    }
}

fn draw_barns(sim: Res<Sim>, mut barns: Query<(&BarnSprite, &mut Sprite)>) {
    for (barn, mut sprite) in &mut barns {
        sprite.color = if sim.0.families[barn.0 as usize].barn_standing {
            Color::srgb(0.45, 0.26, 0.16)
        } else {
            Color::srgb(0.08, 0.07, 0.06)
        };
    }
}

/// The default font has no em dashes or arrows.
fn plain(text: &str) -> String {
    text.replace('—', "-").replace(['→', '⟶'], "->")
}

#[allow(clippy::type_complexity)]
fn update_panel(
    sim: Res<Sim>,
    selection: Res<Selection>,
    clock: Res<Clock>,
    mut header: Query<
        &mut Text,
        (
            With<HeaderLabel>,
            Without<InspectionLabel>,
            Without<LogLabel>,
        ),
    >,
    mut inspect: Query<
        &mut Text,
        (
            With<InspectionLabel>,
            Without<HeaderLabel>,
            Without<LogLabel>,
        ),
    >,
    mut log: Query<
        &mut Text,
        (
            With<LogLabel>,
            Without<HeaderLabel>,
            Without<InspectionLabel>,
        ),
    >,
) {
    if !sim.is_changed() && !selection.is_changed() && !clock.is_changed() {
        return;
    }
    let world = &sim.0;

    if let Ok(mut text) = header.single_mut() {
        let w = world.weather_on(world.day);
        let sky = if w.storm {
            "storm"
        } else if w.rain {
            "rain"
        } else {
            "clear"
        };
        let status = if !world.player_alive() {
            "  — YOU ARE DEAD"
        } else if clock.paused {
            "  — paused"
        } else {
            ""
        };
        **text = plain(&format!(
            "{}   {}, wind {:.0}%, dryness {:.0}%{}",
            world.day,
            sky,
            w.wind * 100.0,
            world.dryness * 100.0,
            status
        ));
    }

    if let Ok(mut text) = inspect.single_mut() {
        **text = match selection.0 {
            None => "Click a neighbor".into(),
            Some(id) if !world.npc(id).alive => format!("{} (dead)", world.npc(id).name),
            Some(id) => {
                let npc = world.npc(id);
                let belief = npc
                    .memories
                    .iter()
                    .filter(|m| m.confidence >= 20)
                    .max_by_key(|m| m.day)
                    .map(|m| {
                        // "[FIRE] The Webb barn burned" -> "The Webb barn burned"
                        let line =
                            chronicle::debug_line(world, &world.events[m.event as usize], false);
                        let what = line.split_once("] ").map(|(_, rest)| rest).unwrap_or(&line);
                        format!(
                            "\n\"{}.\" - blames {} ({}%)",
                            what,
                            suspect_label(world, m.believed),
                            m.confidence
                        )
                    })
                    .unwrap_or_default();
                plain(&format!(
                    "{}  ({})\n{} - health {}   Opinion of you: {}{}",
                    npc.name,
                    npc.faction.label(),
                    psyche::condition(world, id).label(),
                    npc.health,
                    world.opinion(id, PLAYER),
                    belief
                ))
            }
        };
    }

    if sim.is_changed()
        && let Ok(mut text) = log.single_mut()
    {
        let lines = chronicle::chronicle(world, false);
        let start = lines.len().saturating_sub(LOG_LINES);
        **text = plain(&lines[start..].join("\n"));
    }
}
