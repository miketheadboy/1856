//! Bevy view over the headless sim (§21 step 2). This file draws the world
//! and forwards clicks; every rule lives in `bleeding_kansas::sim`.
//!
//! Debug knobs (env vars): BK_SEED, BK_START_DAYS (run the sim ahead before
//! showing it), BK_DAY_SECONDS (clock speed).

use bevy::prelude::*;
use bleeding_kansas::sim::character;
use bleeding_kansas::sim::chronicle::{self, suspect_label};
use bleeding_kansas::sim::civic::Project;
use bleeding_kansas::sim::economy::Choice;
use bleeding_kansas::sim::events::{Cruelty, Source};
use bleeding_kansas::sim::family::{self, Errand};
use bleeding_kansas::sim::farmwork;
use bleeding_kansas::sim::geography::{self, HEIGHT, PLACES, Terrain, WIDTH};
use bleeding_kansas::sim::intrigue;
use bleeding_kansas::sim::law;
use bleeding_kansas::sim::life::{self, Activity, Skill};
use bleeding_kansas::sim::market::Good;
use bleeding_kansas::sim::nations::NationId;
use bleeding_kansas::sim::psyche::{self, Condition};
use bleeding_kansas::sim::world::{Faction, FamilyId, NpcId, PLAYER, World};

const TILE: f32 = 12.0;

// Palette: a survey plat with some life in it. Charcoal frame, bone type,
// live greens and river blue, one oxblood accent and a little brass.
const CHARCOAL: Color = Color::srgb(0.086, 0.078, 0.071);
const PANEL: Color = Color::srgba(0.118, 0.106, 0.090, 0.97);
const BONE: Color = Color::srgb(0.914, 0.882, 0.800);
const DIM: Color = Color::srgb(0.66, 0.62, 0.53);
const BRASS: Color = Color::srgb(0.72, 0.57, 0.35);
const OXBLOOD: Color = Color::srgb(0.56, 0.17, 0.13);
const SLATE: Color = Color::srgb(0.40, 0.55, 0.72);
const INK_GREEN: Color = Color::srgb(0.20, 0.30, 0.18);

#[derive(Resource)]
struct Fonts {
    /// IM Fell English: 17th-century type, digitized with all its grit.
    display: Handle<Font>,
    /// EB Garamond: the body text that has to stay readable.
    body: Handle<Font>,
}

impl FromWorld for Fonts {
    fn from_world(world: &mut bevy::ecs::world::World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            display: assets.load("fonts/IMFellEnglish.ttf"),
            body: assets.load("fonts/EBGaramond.ttf"),
        }
    }
}
/// Map's top-left corner in world coordinates (screen is 1280x720, centered).
const MAP_ORIGIN: Vec2 = Vec2::new(-624.0, 344.0);
const LOG_LINES: usize = 14;

fn env_num<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

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
    phase: f32,
}

#[derive(Component)]
struct BarnSprite(FamilyId);

/// A pale light over a restless grave. Pooled; the sim decides where.
#[derive(Component)]
struct Wisp(usize);

#[derive(Component)]
enum Label {
    Header,
    Tension,
    Inspect,
    Household,
    Market,
    Nations,
    Log,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Action {
    Kill,
    Burn,
    Steal,
    Tavern,
    Pause,
    Choose(Choice),
    Buy(Good),
    Sell(Good),
    Sign(bool),
    Leave(Errand),
    Broker,
    Do(Activity),
    /// Activities aimed at the selected neighbor.
    Visit,
    Court,
    Baptize,
    Sue,
    BuyClaim,
    /// Aimed at the selected neighbor, after dark.
    Dark(Dark),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dark {
    Blackmail,
    Expose,
    Slander,
    Harm(Cruelty),
}

fn main() {
    let mut world = World::new(env_num("BK_SEED", 1856));
    // Fast-forward with the sim running your household, then hand it over.
    world.run_days(env_num("BK_START_DAYS", 0));
    world.autopilot_player = false;
    let day_seconds: f32 = env_num("BK_DAY_SECONDS", 1.2);

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bleeding Kansas".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(CHARCOAL))
        .init_resource::<Fonts>()
        .insert_resource(Sim(world))
        .init_resource::<Selection>()
        .insert_resource(Clock {
            timer: Timer::from_seconds(day_seconds, TimerMode::Repeating),
            paused: false,
        })
        .add_systems(Startup, (setup_map, setup_ui))
        .add_systems(
            Update,
            (
                advance_calendar,
                keyboard,
                select_npc,
                handle_actions,
                draw_npcs,
                draw_barns,
                draw_spirits,
                update_panels,
            )
                .chain(),
        )
        .run();
}

fn tile_to_world(t: (i32, i32)) -> Vec2 {
    MAP_ORIGIN
        + Vec2::new(
            t.0 as f32 * TILE + TILE / 2.0,
            -(t.1 as f32 * TILE + TILE / 2.0),
        )
}

fn terrain_color(t: Terrain, tile: (i32, i32)) -> Color {
    // A little hand-inked unevenness, deterministic per tile.
    let h = (tile.0 as u32).wrapping_mul(73_856_093) ^ (tile.1 as u32).wrapping_mul(19_349_663);
    let j = ((h % 1000) as f32 / 1000.0 - 0.5) * 0.06;
    let (r, g, b) = match t {
        Terrain::Prairie => (0.56, 0.64, 0.36),
        Terrain::Timber => (0.24, 0.42, 0.23),
        Terrain::River => (0.24, 0.50, 0.74),
        Terrain::Road => (0.66, 0.54, 0.36),
        Terrain::Town => (0.85, 0.81, 0.72),
        Terrain::Reserve(NationId::Delaware) => (0.62, 0.68, 0.43),
        Terrain::Reserve(_) => (0.66, 0.66, 0.45),
    };
    let j = if t == Terrain::River { j * 0.5 } else { j };
    Color::srgb(r + j, g + j, b + j * 0.5)
}

fn setup_map(mut commands: Commands, sim: Res<Sim>, fonts: Res<Fonts>) {
    commands.spawn(Camera2d);
    let world = &sim.0;

    for ty in 0..HEIGHT {
        for tx in 0..WIDTH {
            commands.spawn((
                Sprite {
                    color: terrain_color(world.map.at((tx, ty)), (tx, ty)),
                    custom_size: Some(Vec2::splat(TILE)),
                    ..default()
                },
                Transform::from_translation(tile_to_world((tx, ty)).extend(0.0)),
            ));
        }
    }

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
        commands.spawn((
            Sprite {
                color: edge,
                custom_size: Some(Vec2::splat(TILE + 4.0)),
                ..default()
            },
            Transform::from_translation(at.extend(1.0)),
        ));
        commands.spawn((
            Sprite {
                color: Color::srgb(0.45, 0.26, 0.16),
                custom_size: Some(Vec2::splat(TILE)),
                ..default()
            },
            Transform::from_translation(at.extend(1.1)),
            BarnSprite(f.id),
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
            Transform::from_translation((at + Vec2::new(0.0, -13.0)).extend(3.0)),
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

    for (i, npc) in world.npcs.iter().enumerate() {
        commands.spawn((
            Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::splat(if npc.id == PLAYER { 7.0 } else { 5.0 })),
                ..default()
            },
            Transform::from_translation(tile_to_world(world.farm_of(npc.id)).extend(2.0)),
            NpcSprite {
                id: npc.id,
                phase: i as f32 * 2.399,
            },
        ));
    }
}

fn text(font: &Handle<Font>, size: f32, color: Color) -> (TextFont, TextColor) {
    (
        TextFont {
            font: font.clone(),
            font_size: size,
            ..default()
        },
        TextColor(color),
    )
}

fn button_row(
    panel: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    buttons: &[(Action, &str, Color)],
) {
    panel
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(5.0),
            row_gap: Val::Px(5.0),
            ..default()
        })
        .with_children(|row| {
            for (action, label, color) in buttons {
                row.spawn((
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(3.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(BRASS.with_alpha(0.5)),
                    BackgroundColor(*color),
                    *action,
                ))
                .with_child((Text::new(*label), text(font, 13.0, BONE)));
            }
        });
}

fn setup_ui(mut commands: Commands, fonts: Res<Fonts>) {
    let (display, body) = (&fonts.display, &fonts.body);
    let dim = DIM;
    let red = OXBLOOD;
    let brown = Color::srgb(0.36, 0.24, 0.12);
    let blue = Color::srgb(0.16, 0.26, 0.36);
    let green = Color::srgb(0.19, 0.31, 0.17);
    let gray = Color::srgb(0.22, 0.20, 0.18);

    // Right panel.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(800.0),
                top: Val::Px(8.0),
                width: Val::Px(472.0),
                height: Val::Px(704.0),
                padding: UiRect::all(Val::Px(12.0)),
                border: UiRect::all(Val::Px(1.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(BRASS.with_alpha(0.35)),
        ))
        .with_children(|panel| {
            panel.spawn((Text::new("Bleeding Kansas"), text(display, 30.0, BONE)));
            panel.spawn((Text::new(""), text(display, 16.0, BRASS), Label::Header));
            panel.spawn((Text::new(""), text(body, 13.0, dim), Label::Tension));
            panel.spawn((
                Text::new("Click a neighbor"),
                text(body, 14.0, BONE),
                Node {
                    min_height: Val::Px(150.0),
                    ..default()
                },
                Label::Inspect,
            ));
            button_row(
                panel,
                body,
                &[
                    (Action::Kill, "KILL", red),
                    (Action::Burn, "BURN BARN", brown),
                    (Action::Steal, "STEAL FROM", brown),
                    (Action::Choose(Choice::Beg), "BEG", blue),
                    (Action::Tavern, "BE SEEN", blue),
                    (Action::Broker, "BROKER PEACE", blue),
                    (Action::Pause, "PAUSE", gray),
                ],
            );
            button_row(
                panel,
                body,
                &[
                    (Action::Do(Activity::Chores), "CHORES", green),
                    (Action::Do(Activity::Fish), "FISH", green),
                    (Action::Do(Activity::Roam), "ROAM", green),
                    (Action::Visit, "VISIT", blue),
                    (Action::Court, "COURT", blue),
                    (Action::Do(Activity::Drink), "DRINK", brown),
                    (Action::Do(Activity::Preach), "PREACH", blue),
                    (Action::Baptize, "BAPTIZE", blue),
                    (
                        Action::Do(Activity::Speech { calm: true }),
                        "SPEAK: PEACE",
                        blue,
                    ),
                    (
                        Action::Do(Activity::Speech { calm: false }),
                        "SPEAK: FIRE",
                        red,
                    ),
                    (
                        Action::Do(Activity::Build(Project::Schoolhouse)),
                        "BUILD",
                        green,
                    ),
                    (Action::Do(Activity::FileClaim), "FILE CLAIM", gray),
                    (Action::Do(Activity::Study), "STUDY", gray),
                    (Action::Do(Activity::Trap), "TRAP", green),
                    (Action::Do(Activity::Camp), "CAMP", green),
                    (Action::Do(Activity::WriteHome), "WRITE HOME", gray),
                    (Action::Sue, "SUE", gray),
                    (Action::Do(Activity::Vote { sell: false }), "VOTE", blue),
                    (
                        Action::Do(Activity::Vote { sell: true }),
                        "SELL VOTE",
                        brown,
                    ),
                    (Action::Do(Activity::Muster { join: true }), "MUSTER", red),
                    (
                        Action::Do(Activity::Muster { join: false }),
                        "STAY HOME",
                        gray,
                    ),
                    (Action::Do(Activity::Gather), "GO TO THE BEE", green),
                    (
                        Action::Do(Activity::Church { north: true }),
                        "MEETING: NORTH",
                        blue,
                    ),
                    (
                        Action::Do(Activity::Church { north: false }),
                        "MEETING: SOUTH",
                        brown,
                    ),
                    (Action::Do(Activity::BuyLot), "BUY TOWN LOT", blue),
                    (Action::Do(Activity::SellLot), "SELL TOWN LOT", blue),
                    (Action::BuyClaim, "BUY THEIR CLAIM", brown),
                    (Action::Dark(Dark::Blackmail), "BLACKMAIL", red),
                    (Action::Dark(Dark::Expose), "EXPOSE", red),
                    (Action::Dark(Dark::Slander), "SLANDER", red),
                    (
                        Action::Dark(Dark::Harm(Cruelty::KillStock)),
                        "SHOOT A COW",
                        red,
                    ),
                    (
                        Action::Dark(Dark::Harm(Cruelty::FoulWell)),
                        "FOUL WELL",
                        red,
                    ),
                    (
                        Action::Dark(Dark::Harm(Cruelty::CutFence)),
                        "CUT FENCE",
                        red,
                    ),
                    (
                        Action::Dark(Dark::Harm(Cruelty::SpoilHay)),
                        "WET THE HAY",
                        red,
                    ),
                    (Action::Leave(Errand::Buffalo), "LEAVE A WHILE", gray),
                ],
            );
            button_row(
                panel,
                body,
                &[
                    (Action::Choose(Choice::Hunt), "HUNT", green),
                    (Action::Choose(Choice::GoWest), "BUFFALO RANGE", green),
                    (Action::Choose(Choice::EatSeed), "EAT SEED", green),
                    (Action::Choose(Choice::SpareCow), "BUTCHER COW", green),
                    (Action::Choose(Choice::Ox), "BUTCHER OX", green),
                    (Action::Choose(Choice::Borrow), "CREDIT", green),
                    (Action::Buy(Good::Corn), "BUY CORN", blue),
                    (Action::Sell(Good::Corn), "SELL CORN", blue),
                    (Action::Buy(Good::Powder), "BUY POWDER", blue),
                    (Action::Sign(true), "SIGN PETITION", red),
                    (Action::Sign(false), "REFUSE", gray),
                ],
            );
            panel.spawn((
                Text::new("What people are saying"),
                text(display, 17.0, BRASS),
            ));
            panel.spawn((Text::new(""), text(body, 12.5, BONE), Label::Log));
        });

    // Bottom panel: household, market, nations.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                top: Val::Px(504.0),
                width: Val::Px(768.0),
                height: Val::Px(208.0),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(1.0)),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(BRASS.with_alpha(0.35)),
        ))
        .with_children(|bottom| {
            bottom.spawn((
                Text::new(""),
                text(body, 13.0, BONE),
                Node {
                    width: Val::Px(270.0),
                    ..default()
                },
                Label::Household,
            ));
            bottom.spawn((
                Text::new(""),
                text(body, 13.0, BONE),
                Node {
                    width: Val::Px(250.0),
                    ..default()
                },
                Label::Market,
            ));
            bottom.spawn((Text::new(""), text(body, 13.0, BONE), Label::Nations));
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

fn keyboard(keys: Res<ButtonInput<KeyCode>>, mut clock: ResMut<Clock>) {
    if keys.just_pressed(KeyCode::Space) {
        clock.paused = !clock.paused;
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
    // Only clicks on the map select.
    if cursor.x > 790.0 || cursor.y > 496.0 {
        return;
    }
    let Ok((camera, camera_transform)) = camera.single() else {
        return;
    };
    let Ok(point) = camera.viewport_to_world_2d(camera_transform, cursor) else {
        return;
    };
    selection.0 = sprites
        .iter()
        .filter(|(_, s)| s.id != PLAYER && sim.0.npc(s.id).alive)
        .map(|(t, s)| (t.translation().truncate().distance(point), s.id))
        .filter(|(d, _)| *d < 14.0)
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
        let w = &mut sim.0;
        match (*action, selection.0) {
            (Action::Kill, Some(t)) => {
                w.player_kill(t);
            }
            (Action::Burn, Some(t)) => {
                w.player_burn(t);
            }
            (Action::Steal, Some(t)) => {
                w.player_steal(t);
            }
            (Action::Tavern, _) => w.player_go_to_tavern(),
            (Action::Pause, _) => clock.paused = !clock.paused,
            (Action::Choose(c), _) => {
                w.player_choose(c);
            }
            (Action::Buy(g), _) => {
                w.player_buy(g, 10.0);
            }
            (Action::Sell(g), _) => {
                w.player_sell(g, 10.0);
            }
            (Action::Sign(yes), _) => w.player_answer_favor(yes),
            (Action::Leave(e), _) => w.player_leave(e),
            (Action::Do(Activity::Build(_)), _) => {
                // Whatever the county is building next.
                if let Some(p) = Project::ALL.into_iter().find(|&p| !w.civic.built(p)) {
                    w.player_do(Activity::Build(p));
                }
            }
            (Action::Do(a), _) => {
                w.player_do(a);
            }
            (Action::Visit, Some(t)) => {
                w.player_do(Activity::Visit(t));
            }
            (Action::Court, Some(t)) => {
                w.player_do(Activity::Court(t));
            }
            (Action::Baptize, Some(t)) => {
                w.player_do(Activity::Baptize(t));
            }
            (Action::Dark(d), Some(t)) => {
                w.player_do(match d {
                    Dark::Blackmail => Activity::Blackmail(t),
                    Dark::Expose => Activity::Expose(t),
                    Dark::Slander => Activity::Slander(t),
                    Dark::Harm(c) => Activity::Sabotage(t, c),
                });
            }
            (Action::BuyClaim, Some(t)) => {
                let f = w.npc(t).family;
                w.player_do(Activity::BuyClaim(f));
            }
            (Action::Sue, Some(t)) => {
                w.player_do(Activity::Sue(t));
            }
            (Action::Broker, Some(t)) => {
                // Between the selected family and whoever it's feuding with.
                let fam = w.npc(t).family;
                let other = w
                    .feuds
                    .iter()
                    .find_map(|&(a, b)| (a == fam).then_some(b).or((b == fam).then_some(a)));
                if let Some(other) = other {
                    w.player_broker_peace(fam, other);
                }
            }
            _ => {}
        }
    }
}

fn draw_npcs(
    time: Res<Time>,
    sim: Res<Sim>,
    selection: Res<Selection>,
    mut sprites: Query<(&NpcSprite, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let world = &sim.0;
    let t = time.elapsed_secs();
    for (s, mut transform, mut sprite, mut vis) in &mut sprites {
        let npc = world.npc(s.id);
        if npc.adopted_by.is_some() {
            *vis = Visibility::Hidden;
            continue;
        }
        let home = tile_to_world(world.farm_of(s.id));
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
fn draw_spirits(
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
                tf.translation = (tile_to_world(h.site) + drift).extend(4.0);
                // Brighter under a full moon.
                let glow = 0.5 + 0.3 * world.day.moonlight() + 0.15 * (t * 1.3 + phase).sin();
                sprite.color = Color::srgba(0.92, 0.97, 1.0, glow.clamp(0.3, 0.95));
            }
            None => sprite.color = Color::srgba(0.85, 0.92, 1.0, 0.0),
        }
    }
}

fn draw_barns(sim: Res<Sim>, mut barns: Query<(&BarnSprite, &mut Sprite)>) {
    for (barn, mut sprite) in &mut barns {
        sprite.color = if sim.0.families[barn.0 as usize].barn_standing {
            Color::srgb(0.45, 0.26, 0.16)
        } else {
            Color::srgb(0.06, 0.05, 0.04)
        };
    }
}

/// The default font has no em dashes or arrows.
fn plain(text: &str) -> String {
    text.replace('—', "-").replace(['→', '⟶'], "->")
}

fn bar(v: f32, max: f32) -> String {
    let n = ((v / max) * 10.0).clamp(0.0, 10.0) as usize;
    format!("{}{}", "#".repeat(n), ".".repeat(10 - n))
}

fn stance(v: f32) -> &'static str {
    match v {
        v if v <= -0.6 => "fire-eating Pro-Slavery",
        v if v <= -0.2 => "Pro-Slavery",
        v if v < 0.2 => "keeps quiet on the question",
        v if v < 0.6 => "Free-State",
        _ => "hard Free-State",
    }
}

fn seems(world: &World, id: NpcId) -> String {
    let e = world.npc(id).emotions;
    let mut out = Vec::new();
    for (v, word) in [
        (e.fear, "afraid"),
        (e.anger, "angry"),
        (e.grief, "grieving"),
        (e.zeal, "zealous"),
    ] {
        if v > 25.0 {
            out.push(word);
        }
    }
    if out.is_empty() {
        "steady".into()
    } else {
        out.join(", ")
    }
}

fn inspect(world: &World, id: NpcId) -> String {
    let npc = world.npc(id);
    if !npc.alive {
        return format!("{} (dead)", npc.name);
    }
    let known: Vec<&str> = character::archetypes(npc)
        .into_iter()
        .map(|a| a.label())
        .collect();
    let belief = npc
        .memories
        .iter()
        .filter(|m| m.confidence >= 20)
        .max_by_key(|m| m.day)
        .map(|m| {
            let line = chronicle::debug_line(world, &world.events[m.event as usize], false);
            let what = line
                .split_once("] ")
                .map(|(_, r)| r)
                .unwrap_or(&line)
                .to_string();
            let how = match m.source {
                Source::Witnessed => "saw it",
                Source::Victim => "their loss",
                Source::Bystander => "was nearby",
                Source::Told(_) => "heard it",
                Source::Newspaper(_) => "read it in the paper",
            };
            format!(
                "\n\"{}.\" Blames {} ({}%, {})",
                what,
                suspect_label(world, m.believed),
                m.confidence,
                how
            )
        })
        .unwrap_or_default();
    plain(&format!(
        "{}, {}  ({})\n{}. Health {}. Seems {}.\nSays: {}\nKnown as: {}\nOpinion of you: {}{}",
        npc.name,
        npc.age,
        npc.faction.label(),
        psyche::condition(world, id).label(),
        npc.health,
        seems(world, id),
        stance(npc.ideology.public),
        if known.is_empty() {
            "nothing in particular".into()
        } else {
            known.join(", ")
        },
        world.opinion(id, PLAYER),
        belief
    ))
}

fn household(world: &World) -> String {
    let f = &world.families[0];
    let hh = &f.stores;
    let people = world.living().filter(|n| n.family == 0).count().max(1) as f32;
    let kin: Vec<String> = world
        .npcs
        .iter()
        .filter(|n| n.family == 0)
        .map(|n| {
            format!(
                "{} {} ({})",
                if n.id == PLAYER { "You" } else { &n.name },
                n.health,
                psyche::condition(world, n.id).label()
            )
        })
        .collect();
    let mut s = format!(
        "YOUR HOUSEHOLD\n{}\n\nFood: {:.0} days ({:.0} each)\nSeed corn: {} bu   Acres: {}\nCattle: {}   Oxen: {}\nCash: ${}   Debt: ${}\nSalt {:.0}  Powder {:.0}  Timber {:.0}  Hides {:.0}\nBarn: {}",
        kin.join("\n"),
        hh.food,
        hh.food / people,
        hh.seed,
        hh.acres,
        hh.cattle,
        hh.oxen,
        hh.cash,
        hh.debt,
        hh.goods[Good::Salt.index()],
        hh.goods[Good::Powder.index()],
        hh.goods[Good::Timber.index()],
        hh.goods[Good::Hides.index()],
        if f.barn_standing {
            "standing"
        } else {
            "burned"
        },
    );
    s.push('\n');
    s.push_str(&farmwork::status(world));
    if world.pending_favor.is_some() {
        s.push_str("\n\nDUNMORE WANTS YOUR NAME ON HIS PETITION.");
    }
    s
}

/// The Jones panel: spirits, goals, skills, and what the county calls you.
fn your_life(world: &World) -> String {
    let l = &world.life;
    let mut s = format!("YOUR LIFE   spirits {}\n", bar(l.spirits, 100.0));
    let goals: Vec<String> = life::goals(world)
        .iter()
        .map(|(name, v)| format!("{name} {:.0}%", v * 100.0))
        .collect();
    s.push_str(&goals.join("   "));
    s.push('\n');
    let mut skills: Vec<(f32, &str)> = Skill::ALL
        .iter()
        .map(|&k| (l.skill(k), k.label()))
        .filter(|(v, _)| *v > 0.0)
        .collect();
    skills.sort_by(|a, b| b.0.total_cmp(&a.0));
    if !skills.is_empty() {
        let top: Vec<String> = skills
            .iter()
            .take(4)
            .map(|(v, k)| format!("{k} {:.0}", v * 10.0))
            .collect();
        s.push_str(&top.join(", "));
        s.push('\n');
    }
    s.push_str(&format!(
        "Lawrence lots ${:.0}   you hold {}\n",
        world.land.lot_price, world.land.lots
    ));
    let leverage: Vec<&str> = intrigue::leverage(world)
        .into_iter()
        .map(|n| world.name(n))
        .collect();
    if !leverage.is_empty() {
        s.push_str(&format!("You know things about: {}\n", leverage.join(", ")));
    }
    let paths = life::paths(world);
    if !paths.is_empty() {
        s.push_str(&format!("They call you: {}\n", paths.join(", ")));
    }
    if let Some(e) = family::away(world) {
        s.push_str(&format!("You are {}.\n", e.label()));
    } else if l.acted_on == Some(world.day) {
        s.push_str("Your day is spent.\n");
    }
    if let Some((bee, host, _)) = &world.gatherings.today {
        s.push_str(&format!(
            "Tonight: a {} at the {} place.\n",
            bee.label(),
            world.families[*host as usize].surname
        ));
    }
    if let Some(e) = law::election_today(world) {
        s.push_str(&format!("ELECTION DAY: {}.\n", e.name));
    }
    if let Some((i, _, _)) = &world.law.muster
        && !world.law.player_answered
    {
        s.push_str(&format!(
            "THE MUSTER IS CALLED: {}.\n",
            law::MUSTERS[*i].name
        ));
    }
    s.push_str("Click a neighbor to visit, court, baptize, or sue.");
    s
}

fn market_and_nations(world: &World) -> String {
    let m = &world.market;
    let mut s = String::from("DUNMORE'S, FRANKLIN\n");
    for g in Good::ALL {
        let arrow = match m.pressure(g) {
            p if p >= 1.5 => " ^",
            p if p <= 0.7 => " v",
            _ => "",
        };
        s.push_str(&format!(
            "{:<14} ${:>5.2}/{:<6} stock {:>4.0}{}\n",
            g.label(),
            m.price(g),
            g.unit(),
            m.stock(g),
            arrow
        ));
    }
    if m.blockade {
        s.push_str("The roads from Westport are closed\n");
    }
    s
}

fn nations(world: &World) -> String {
    let herd = world.bison.abundance();
    let mut s = format!(
        "BUFFALO RANGE, WEST\n{} {}\n\nNATIONS\npressure, trust F-S / P-S\n",
        bar(herd, 1.0),
        if herd > 0.6 {
            "herds plenty"
        } else if herd > 0.3 {
            "herds thinning"
        } else {
            "bones on the prairie"
        }
    );
    for n in &world.nations {
        s.push_str(&format!(
            "{:<9} {}  {:.0} / {:.0}\n",
            n.id.name(),
            bar(n.land_pressure, 100.0),
            n.trust[Faction::FreeState.index()],
            n.trust[Faction::ProSlavery.index()],
        ));
    }
    s
}

fn update_panels(
    sim: Res<Sim>,
    selection: Res<Selection>,
    clock: Res<Clock>,
    mut labels: Query<(&Label, &mut Text)>,
) {
    if !sim.is_changed() && !selection.is_changed() && !clock.is_changed() {
        return;
    }
    let world = &sim.0;
    for (label, mut t) in &mut labels {
        let content = match label {
            Label::Header => {
                let w = world.weather_on(world.day);
                let sky = if w.blizzard {
                    "blizzard"
                } else if w.storm {
                    "a hard rain"
                } else if w.rain {
                    "rain"
                } else {
                    "clear"
                };
                let status = if !world.player_alive() {
                    "   Knocking on heaven's door"
                } else if clock.paused {
                    "   (paused)"
                } else {
                    ""
                };
                format!(
                    "{}   {}, {}, wind {:.0}%{}",
                    world.day,
                    sky,
                    world.day.moon_name(),
                    w.wind * 100.0,
                    status
                )
            }
            Label::Tension => format!(
                "Free-State grievance  {} {}\nPro-Slavery grievance {} {}{}",
                bar(world.grievance[Faction::FreeState.index()] as f32, 150.0),
                world.grievance[Faction::FreeState.index()],
                bar(world.grievance[Faction::ProSlavery.index()] as f32, 150.0),
                world.grievance[Faction::ProSlavery.index()],
                if world.pacified_until.is_some_and(|d| world.day < d) {
                    "\nFederal dragoons patrol the roads"
                } else {
                    ""
                }
            ),
            Label::Inspect => match selection.0 {
                None => your_life(world),
                Some(id) => inspect(world, id),
            },
            Label::Household => household(world),
            Label::Market => market_and_nations(world),
            Label::Nations => nations(world),
            Label::Log => {
                let lines = chronicle::chronicle(world, false);
                let start = lines.len().saturating_sub(LOG_LINES);
                lines[start..].join("\n")
            }
        };
        **t = plain(&content);
    }
}
