//! Bevy view over the headless sim (§21 step 2). Three scales, like the old
//! console RPGs: the county map (wide), your claim and the town streets (on
//! foot), and scenes for the moments that won't wait. Command windows in
//! place of a wall of buttons. Every rule lives in `bleeding_kansas::sim`.
//!
//! Debug knobs (env vars): BK_SEED, BK_START_DAYS (run the sim ahead before
//! showing it), BK_DAY_SECONDS (clock speed), BK_SCREEN (county, claim,
//! lawrence, franklin, lecompton), BK_PLAY (gate, door, raid, ambush, bench: start
//! in one of the action games against the Pikes).

// Bevy systems take their world as arguments; long parameter lists and
// query types are how it's written.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

mod claim;
mod cmds;
mod county;
mod duel;
mod panels;
mod raid;
mod scene;
mod scenery;
mod town;
mod ui;
mod walk;

use bevy::prelude::*;
use bleeding_kansas::sim::world::{NpcId, World};

const TILE: f32 = 12.0;

// Palette: a survey plat with some life in it. Charcoal frame, bone type,
// live greens and river blue, one oxblood accent and a little brass.
const CHARCOAL: Color = Color::srgb(0.086, 0.078, 0.071);
const OXBLOOD: Color = Color::srgb(0.56, 0.17, 0.13);
const SLATE: Color = Color::srgb(0.40, 0.55, 0.72);
const INK_GREEN: Color = Color::srgb(0.20, 0.30, 0.18);

/// Map's top-left corner in world coordinates.
const MAP_ORIGIN: Vec2 = Vec2::new(-624.0, 344.0);

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

#[derive(Resource)]
struct Sim(World);

#[derive(Resource, Default)]
struct Selection(Option<NpcId>);

#[derive(Resource)]
struct Clock {
    timer: Timer,
    paused: bool,
}

/// Which scale you're looking at.
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Screen {
    County,
    #[default]
    Claim,
    Town,
    /// Someone else's place, at night.
    Raid,
}

/// Which town, when the screen is a town.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
enum TownId {
    #[default]
    Lawrence,
    Franklin,
    Lecompton,
}

fn env_num<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn tile_to_world(t: (i32, i32)) -> Vec2 {
    MAP_ORIGIN
        + Vec2::new(
            t.0 as f32 * TILE + TILE / 2.0,
            -(t.1 as f32 * TILE + TILE / 2.0),
        )
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

fn main() {
    let mut world = World::new(env_num("BK_SEED", 1856));
    // Fast-forward with the sim running your household, then hand it over.
    world.run_days(env_num("BK_START_DAYS", 0));
    world.autopilot_player = false;
    // Four seconds a day: long enough to choose the day's work, and for
    // dusk and a moonlit night to pass over the map.
    let day_seconds: f32 = env_num("BK_DAY_SECONDS", 4.0);
    let (screen, town) = match std::env::var("BK_SCREEN").as_deref() {
        Ok("county") => (Screen::County, TownId::Lawrence),
        Ok("lawrence") => (Screen::Town, TownId::Lawrence),
        Ok("franklin") => (Screen::Town, TownId::Franklin),
        Ok("lecompton") => (Screen::Town, TownId::Lecompton),
        _ => (Screen::Claim, TownId::Lawrence),
    };
    let scenes = scene::Scenes::starting_at(world.events.len());

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
        .init_resource::<scenery::Art>()
        .insert_resource(Sim(world))
        .init_resource::<Selection>()
        .insert_resource(Clock {
            timer: Timer::from_seconds(day_seconds, TimerMode::Repeating),
            paused: false,
        })
        .insert_state(screen)
        .insert_resource(town)
        .init_resource::<ui::Menu>()
        .init_resource::<ui::Toast>()
        .init_resource::<ui::Overlay>()
        .init_resource::<walk::Bounds>()
        .init_resource::<county::CountyView>()
        .init_resource::<duel::Game>()
        .init_resource::<raid::Raid>()
        .insert_resource(scenes)
        .add_systems(
            Startup,
            (
                spawn_camera,
                scenery::setup,
                county::setup,
                ui::setup,
                scene::setup,
                duel::setup,
                debug_play,
            )
                .chain(),
        )
        .add_systems(OnEnter(Screen::County), county::enter)
        .add_systems(OnEnter(Screen::Claim), claim::enter)
        .add_systems(OnExit(Screen::Claim), walk::despawn)
        .add_systems(OnEnter(Screen::Town), town::enter)
        .add_systems(OnExit(Screen::Town), walk::despawn)
        .add_systems(OnEnter(Screen::Raid), raid::enter)
        .add_systems(OnExit(Screen::Raid), walk::despawn)
        .add_systems(
            Update,
            (
                advance_calendar,
                keyboard,
                ui::drive_menu,
                ui::draw_menu,
                ui::draw_toast,
                ui::draw_hud,
                ui::overlays,
                ui::dusk,
                scene::run,
                duel::play,
                duel::draw,
            )
                .chain(),
        )
        .add_systems(Update, raid::sneak.run_if(in_state(Screen::Raid)))
        .add_systems(
            Update,
            (
                county::control,
                county::click,
                county::spawn_figures,
                county::draw_npcs,
                county::draw_spirits,
                scenery::seasons,
                scenery::buildings,
                scenery::night,
                scenery::weather,
                scenery::smoke,
            )
                .run_if(in_state(Screen::County)),
        )
        .add_systems(
            Update,
            (
                claim::rebuild,
                claim::kin,
                walk::walk,
                walk::interact,
                walk::rest_key,
            )
                .chain()
                .run_if(in_state(Screen::Claim)),
        )
        .add_systems(
            Update,
            (town::stroll, walk::walk, walk::interact, walk::rest_key)
                .chain()
                .run_if(in_state(Screen::Town)),
        )
        .run();
}

/// BK_PLAY: drop straight into an action game, for testing by hand.
fn debug_play(
    mut sim: ResMut<Sim>,
    mut game: ResMut<duel::Game>,
    mut raid: ResMut<raid::Raid>,
    mut next: ResMut<NextState<Screen>>,
) {
    use bleeding_kansas::sim::action;
    let world = &mut sim.0;
    let Some(pike) = world
        .living()
        .find(|n| {
            n.faction == bleeding_kansas::sim::Faction::ProSlavery
                && n.family != 0
                && !world.families[n.family as usize].store
                && n.age >= 18
                && !bleeding_kansas::sim::world::is_woman(&n.name)
        })
        .map(|n| n.id)
    else {
        return;
    };
    match std::env::var("BK_PLAY").as_deref() {
        Ok("gate") => {
            action::park(
                world,
                pike,
                bleeding_kansas::sim::events::Retaliation::Arson,
                None,
            );
        }
        Ok("door") => {
            action::confront(world, pike);
        }
        Ok("raid") => {
            raid.plan = action::raid_plan(world, pike);
            next.set(Screen::Raid);
        }
        Ok("bench") => {
            game.craft(world, bleeding_kansas::sim::arms::Craft::Balls, 1.0);
        }
        Ok("ambush") => {
            if let Some(plan) = action::ambush_plan(world, pike) {
                game.ambush(world, plan, 1.0);
            }
        }
        _ => {}
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn advance_calendar(
    time: Res<Time>,
    mut clock: ResMut<Clock>,
    mut sim: ResMut<Sim>,
    menu: Res<ui::Menu>,
    overlay: Res<ui::Overlay>,
    state: Res<State<Screen>>,
) {
    // Nights out take no time on the clock: the day's already done.
    if clock.paused
        || !sim.0.player_alive()
        || ui::blocking(&menu, &overlay)
        || *state.get() == Screen::Raid
    {
        return;
    }
    clock.timer.tick(time.delta());
    if clock.timer.just_finished() {
        sim.0.advance_day();
    }
}

fn keyboard(keys: Res<ButtonInput<KeyCode>>, mut clock: ResMut<Clock>, menu: Res<ui::Menu>) {
    if keys.just_pressed(KeyCode::Space) && !menu.is_open() {
        clock.paused = !clock.paused;
    }
}
