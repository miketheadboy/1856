//! What the county looks like: engraved buildings and figures, seasons on
//! the grass, weather over the map, smoke from burning barns, and night.
//! The view reads the sim; nothing here changes it.

use bevy::prelude::*;
use bleeding_kansas::sim::civic::Project;
use bleeding_kansas::sim::geography::{self, HEIGHT, Terrain, WIDTH};
use bleeding_kansas::sim::nations::NationId;
use bleeding_kansas::sim::psyche::LifeStage;
use bleeding_kansas::sim::world::{FamilyId, is_woman};
use bleeding_kansas::sim::{Day, World};

use crate::{Clock, MAP_ORIGIN, Sim, TILE, tile_to_world};

/// Paper, as the engravings are printed on.
const PAPER: Color = Color::srgb(0.93, 0.89, 0.80);

#[derive(Resource)]
pub struct Art {
    pub cabin: Handle<Image>,
    pub barn: Handle<Image>,
    pub ruin: Handle<Image>,
    pub rick: Handle<Image>,
    pub school: Handle<Image>,
    pub church: Handle<Image>,
    pub lyceum: Handle<Image>,
    pub bridge: Handle<Image>,
    pub man: Handle<Image>,
    pub woman: Handle<Image>,
    pub child: Handle<Image>,
    pub glow: Handle<Image>,
    pub puff: Handle<Image>,
}

impl FromWorld for Art {
    fn from_world(world: &mut bevy::ecs::world::World) -> Self {
        let a = world.resource::<AssetServer>();
        let l = |n: &str| a.load(format!("sprites/{n}.png"));
        Art {
            cabin: l("cabin"),
            barn: l("barn"),
            ruin: l("ruin"),
            rick: l("rick"),
            school: l("school"),
            church: l("church"),
            lyceum: l("lyceum"),
            bridge: l("bridge"),
            man: l("man"),
            woman: l("woman"),
            child: l("child"),
            glow: l("glow"),
            puff: l("puff"),
        }
    }
}

#[derive(Component)]
pub struct Tile {
    terrain: Terrain,
    at: (i32, i32),
}

#[derive(Component)]
pub struct Barn(pub FamilyId);

#[derive(Component)]
pub struct Rick(FamilyId);

#[derive(Component)]
pub struct Lamp(FamilyId);

#[derive(Component)]
pub struct Civic(Project);

#[derive(Component)]
pub struct Night;

#[derive(Component)]
pub struct Drop {
    snow: bool,
    seed: f32,
}

#[derive(Component)]
pub struct Smoke(usize);

/// The figure for a person: a man, a woman in a bonnet, a child.
pub fn figure(art: &Art, world: &World, id: u32) -> Handle<Image> {
    let n = world.npc(id);
    if LifeStage::of(n.age) == LifeStage::Child {
        art.child.clone()
    } else if is_woman(&n.name) {
        art.woman.clone()
    } else {
        art.man.clone()
    }
}

/// A little hand-inked unevenness, deterministic per tile.
fn jitter(tile: (i32, i32)) -> f32 {
    let h = (tile.0 as u32).wrapping_mul(73_856_093) ^ (tile.1 as u32).wrapping_mul(19_349_663);
    ((h % 1000) as f32 / 1000.0 - 0.5) * 0.06
}

/// Green in spring, deep in summer, gold in the fall, dun in winter, and
/// white after a blizzard.
pub fn terrain_color(t: Terrain, tile: (i32, i32), world: &World) -> Color {
    let day = world.day;
    let (_, m, d) = day.date();
    let snow = (0..10).any(|k| day.0 >= k && world.weather_on(Day(day.0 - k)).blizzard);
    let autumn = matches!((m, d), (9, 20..) | (10, _) | (11, _));
    let (r, g, b) = match t {
        Terrain::River if snow => (0.62, 0.72, 0.82),
        Terrain::River => (0.24, 0.50, 0.74),
        Terrain::Road if snow => (0.80, 0.78, 0.74),
        Terrain::Road => (0.55, 0.44, 0.32),
        Terrain::Town => (0.85, 0.81, 0.72),
        _ if snow => match t {
            Terrain::Timber => (0.52, 0.56, 0.54),
            _ => (0.90, 0.91, 0.92),
        },
        Terrain::Timber => match m {
            12 | 1 | 2 | 3 => (0.34, 0.30, 0.24),
            _ if autumn => (0.62, 0.28, 0.12),
            4 | 5 => (0.30, 0.50, 0.25),
            _ => (0.22, 0.40, 0.21),
        },
        Terrain::Prairie | Terrain::Reserve(_) => {
            let base = match m {
                12 | 1 | 2 | 3 => (0.66, 0.60, 0.46),
                _ if autumn => (0.74, 0.62, 0.32),
                4 | 5 => (0.52, 0.70, 0.34),
                6 | 7 => (0.50, 0.63, 0.32),
                _ => (0.62, 0.62, 0.34),
            };
            match t {
                Terrain::Reserve(NationId::Delaware) => {
                    (base.0 + 0.05, base.1 + 0.03, base.2 + 0.06)
                }
                Terrain::Reserve(_) => (base.0 + 0.08, base.1 + 0.02, base.2 + 0.08),
                _ => base,
            }
        }
    };
    let j = jitter(tile);
    let j = if t == Terrain::River { j * 0.5 } else { j };
    Color::srgb(r + j, g + j, b + j * 0.5)
}

pub fn setup(mut commands: Commands, sim: Res<Sim>, art: Res<Art>) {
    let world = &sim.0;
    for ty in 0..HEIGHT {
        for tx in 0..WIDTH {
            let terrain = world.map.at((tx, ty));
            commands.spawn((
                Sprite {
                    color: terrain_color(terrain, (tx, ty), world),
                    custom_size: Some(Vec2::splat(TILE)),
                    ..default()
                },
                Transform::from_translation(tile_to_world((tx, ty)).extend(0.0)),
                Tile {
                    terrain,
                    at: (tx, ty),
                },
            ));
        }
    }

    // Each claim: a cabin, a barn, a hay rick, and a lamp for the night.
    for f in &world.families {
        let at = tile_to_world(f.farm);
        let ink = |img: &Handle<Image>, size: Vec2| Sprite {
            image: img.clone(),
            color: PAPER,
            custom_size: Some(size),
            ..default()
        };
        commands.spawn((
            ink(&art.cabin, Vec2::new(16.0, 13.0)),
            Transform::from_translation((at + Vec2::new(-5.0, 2.0)).extend(1.2)),
        ));
        commands.spawn((
            ink(&art.barn, Vec2::new(17.0, 14.0)),
            Transform::from_translation((at + Vec2::new(9.0, 3.0)).extend(1.1)),
            Barn(f.id),
        ));
        commands.spawn((
            ink(&art.rick, Vec2::new(9.0, 8.0)),
            Transform::from_translation((at + Vec2::new(17.0, -3.0)).extend(1.15)),
            Rick(f.id),
        ));
        commands.spawn((
            Sprite {
                image: art.glow.clone(),
                color: Color::srgba(1.0, 0.78, 0.40, 0.0),
                custom_size: Some(Vec2::splat(14.0)),
                ..default()
            },
            Transform::from_translation((at + Vec2::new(-5.0, 1.0)).extend(6.0)),
            Lamp(f.id),
        ));
    }

    // Lawrence's church, and the county's projects once they're raised.
    let place = |name: &str| {
        geography::PLACES
            .iter()
            .find(|p| p.name == name)
            .map(|p| tile_to_world(geography::to_tile(p.at)))
            .unwrap_or(Vec2::ZERO)
    };
    let lawrence = place("Lawrence");
    commands.spawn((
        Sprite {
            image: art.church.clone(),
            color: PAPER,
            custom_size: Some(Vec2::new(14.0, 19.0)),
            ..default()
        },
        Transform::from_translation((lawrence + Vec2::new(-16.0, -2.0)).extend(1.2)),
    ));
    // The bridge goes where the Clinton road meets the Wakarusa.
    let clinton = geography::to_tile(
        geography::PLACES
            .iter()
            .find(|p| p.name == "Clinton")
            .map_or((12.0, 9.8), |p| p.at),
    );
    let mut crossing = clinton;
    let mut best = f32::MAX;
    for ty in 0..HEIGHT {
        for tx in 0..WIDTH {
            if world.map.at((tx, ty)) == Terrain::River {
                let d = ((tx - clinton.0).pow(2) + (ty - clinton.1).pow(2)) as f32;
                if d < best && (tx, ty) != clinton {
                    best = d;
                    crossing = (tx, ty);
                }
            }
        }
    }
    for (p, img, pos, size) in [
        (
            Project::Schoolhouse,
            &art.school,
            place("Blanton's Bridge") + Vec2::new(-24.0, -22.0),
            Vec2::new(15.0, 15.0),
        ),
        (
            Project::Bridge,
            &art.bridge,
            tile_to_world(crossing) + Vec2::new(0.0, 2.0),
            Vec2::new(22.0, 9.0),
        ),
        (
            Project::Lyceum,
            &art.lyceum,
            lawrence + Vec2::new(18.0, -6.0),
            Vec2::new(17.0, 15.0),
        ),
    ] {
        commands.spawn((
            Sprite {
                image: img.clone(),
                color: PAPER,
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(pos.extend(1.2)),
            Visibility::Hidden,
            Civic(p),
        ));
    }

    // Night lies over the whole map.
    let size = Vec2::new(WIDTH as f32 * TILE, HEIGHT as f32 * TILE);
    commands.spawn((
        Sprite {
            color: Color::srgba(0.02, 0.03, 0.08, 0.0),
            custom_size: Some(size),
            ..default()
        },
        Transform::from_translation((MAP_ORIGIN + Vec2::new(size.x, -size.y) / 2.0).extend(5.0)),
        Night,
    ));

    // Rain and snow, pooled.
    for i in 0..160 {
        commands.spawn((
            Sprite {
                color: Color::srgba(0.8, 0.85, 0.95, 0.0),
                custom_size: Some(Vec2::new(1.0, 7.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 7.0))
                .with_rotation(Quat::from_rotation_z(-0.35)),
            Drop {
                snow: i % 2 == 1,
                seed: i as f32 * 12.9898,
            },
        ));
    }
    // Smoke, pooled.
    for i in 0..48 {
        commands.spawn((
            Sprite {
                image: art.puff.clone(),
                color: Color::srgba(0.25, 0.23, 0.22, 0.0),
                custom_size: Some(Vec2::splat(10.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, 6.5)),
            Smoke(i),
        ));
    }
}

/// Recolor the grass when the day turns.
pub fn seasons(
    sim: Res<Sim>,
    mut last: Local<Option<u32>>,
    mut tiles: Query<(&Tile, &mut Sprite)>,
) {
    let world = &sim.0;
    if *last == Some(world.day.0) {
        return;
    }
    *last = Some(world.day.0);
    for (t, mut s) in &mut tiles {
        s.color = terrain_color(t.terrain, t.at, world);
    }
}

/// 0 full day .. 1 deep night, over the course of one sim day.
pub fn darkness(fraction: f32) -> f32 {
    match fraction {
        f if f < 0.55 => 0.0,
        f if f < 0.68 => (f - 0.55) / 0.13,
        f if f < 0.92 => 1.0,
        f => 1.0 - (f - 0.92) / 0.08,
    }
}

pub fn night(
    sim: Res<Sim>,
    clock: Res<Clock>,
    mut sky: Query<&mut Sprite, With<Night>>,
    mut lamps: Query<(&Lamp, &mut Sprite), Without<Night>>,
) {
    let world = &sim.0;
    let dark = darkness(clock.timer.fraction());
    let moon = world.day.moonlight();
    for mut s in &mut sky {
        s.color = Color::srgba(0.02, 0.03, 0.09, dark * (0.78 - 0.45 * moon));
    }
    for (l, mut s) in &mut lamps {
        let f = &world.families[l.0 as usize];
        let home = world.head_of(l.0).is_some() && f.barn_standing;
        let a = if home { 0.8 * dark } else { 0.0 };
        s.color = Color::srgba(1.0, 0.78, 0.40, a);
    }
}

/// Barn sprites, kept apart from ricks and county buildings.
type BarnQuery<'w, 's> =
    Query<'w, 's, (&'static Barn, &'static mut Sprite), (Without<Rick>, Without<Civic>)>;

pub fn buildings(
    sim: Res<Sim>,
    art: Res<Art>,
    mut barns: BarnQuery,
    mut ricks: Query<(&Rick, &mut Visibility), Without<Civic>>,
    mut civic: Query<(&Civic, &mut Visibility), Without<Rick>>,
) {
    let world = &sim.0;
    for (b, mut s) in &mut barns {
        let standing = world.families[b.0 as usize].barn_standing;
        let want = if standing { &art.barn } else { &art.ruin };
        if &s.image != want {
            s.image = want.clone();
        }
    }
    for (r, mut v) in &mut ricks {
        let hay = world.families[r.0 as usize].stores.work.hay;
        *v = if hay >= 0.5 && world.head_of(r.0).is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (c, mut v) in &mut civic {
        *v = if world.civic.built(c.0) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

pub fn weather(
    time: Res<Time>,
    sim: Res<Sim>,
    mut drops: Query<(&Drop, &mut Transform, &mut Sprite)>,
) {
    let world = &sim.0;
    let w = world.weather_on(world.day);
    let winter = matches!(world.day.month(), 12 | 1 | 2);
    let snowing = w.blizzard || (winter && w.rain);
    let raining = w.rain && !snowing;
    let heavy = if w.storm || w.blizzard { 1.0 } else { 0.55 };
    let t = time.elapsed_secs();
    let (width, height) = (WIDTH as f32 * TILE, HEIGHT as f32 * TILE);
    for (d, mut tf, mut s) in &mut drops {
        let on = (d.snow && snowing) || (!d.snow && raining);
        let active = on && (d.seed * 0.618).fract() < heavy;
        if !active {
            s.color = s.color.with_alpha(0.0);
            continue;
        }
        let x0 = (d.seed * 43.758).fract() * width;
        let (speed, drift) = if d.snow {
            (28.0, 6.0 * (t * 0.8 + d.seed).sin())
        } else {
            (260.0, 0.0)
        };
        let y = ((d.seed * 7.31).fract() * height + t * speed) % height;
        let x = (x0 + if d.snow { drift } else { y * 0.36 }) % width;
        tf.translation = Vec3::new(MAP_ORIGIN.x + x, MAP_ORIGIN.y - y, 7.0);
        if d.snow {
            s.custom_size = Some(Vec2::splat(2.0));
            tf.rotation = Quat::IDENTITY;
            s.color = Color::srgba(0.97, 0.97, 1.0, 0.85);
        } else {
            s.custom_size = Some(Vec2::new(1.0, 7.0));
            tf.rotation = Quat::from_rotation_z(-0.35);
            s.color = Color::srgba(0.75, 0.82, 0.95, 0.5);
        }
    }
}

/// Smoke over barns that burned in the last few days.
pub fn smoke(
    time: Res<Time>,
    sim: Res<Sim>,
    mut puffs: Query<(&Smoke, &mut Transform, &mut Sprite)>,
) {
    let world = &sim.0;
    let fires: Vec<Vec2> = world
        .families
        .iter()
        .filter(|f| f.barn_burned_on.is_some_and(|d| world.day.0 <= d.0 + 2))
        .map(|f| tile_to_world(f.farm) + Vec2::new(9.0, 6.0))
        .collect();
    let t = time.elapsed_secs();
    let wind = world.weather_on(world.day).wind;
    for (p, mut tf, mut s) in &mut puffs {
        let Some(&at) = fires
            .get(p.0 % 8)
            .filter(|_| p.0 / 8 < 6 && p.0 % 8 < fires.len())
        else {
            s.color = s.color.with_alpha(0.0);
            continue;
        };
        let life = ((t * 0.35) + (p.0 / 8) as f32 / 6.0).fract();
        let rise = life * 46.0;
        let lean = life * life * 30.0 * wind;
        tf.translation = (at + Vec2::new(lean + (t + p.0 as f32).sin() * 2.0, rise)).extend(6.5);
        s.custom_size = Some(Vec2::splat(6.0 + 16.0 * life));
        s.color = Color::srgba(0.22, 0.20, 0.19, 0.55 * (1.0 - life));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn night_comes_and_goes() {
        assert_eq!(darkness(0.2), 0.0);
        assert_eq!(darkness(0.8), 1.0);
        assert!(darkness(0.62) > 0.0 && darkness(0.62) < 1.0);
        assert!(darkness(0.97) < 1.0);
    }
}
