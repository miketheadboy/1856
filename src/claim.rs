//! Your claim, up close. Everything on it is drawn from the sim: acres under
//! the plow and what's growing on them by season, the fence as sound as it
//! is, the hay rick, the barn or its ashes, the improvements you've raised,
//! and the timber along the creek thinning to stumps as it's cut.

use bevy::prelude::*;
use bleeding_kansas::sim::geography::{TIMBER_LOADS, Terrain};
use bleeding_kansas::sim::homestead::{self, Improvement};
use bleeding_kansas::sim::psyche::LifeStage;
use bleeding_kansas::sim::world::{PLAYER, World};

use crate::scenery::{self, Art};
use crate::walk::{self, Bounds, OnFoot, Spot, SpotKind};
use crate::{Fonts, Sim};

/// The claim sits far off to the east of the county map in world space.
pub const ORIGIN: Vec2 = Vec2::new(4000.0, 300.0);
const T: f32 = 32.0;
const W: i32 = 30;
const H: i32 = 18;
const PAPER: Color = Color::srgb(0.93, 0.89, 0.80);

/// Local tile to world position (y runs down the claim).
fn at(x: f32, y: f32) -> Vec2 {
    ORIGIN + Vec2::new(x * T, -y * T)
}

/// Parts of the claim that change with the sim; rebuilt when it does.
#[derive(Component)]
pub struct Living;

#[derive(Component)]
pub struct Kin {
    id: u32,
    phase: f32,
}

pub fn enter(
    mut commands: Commands,
    art: Res<Art>,
    fonts: Res<Fonts>,
    mut bounds: ResMut<Bounds>,
    mut cam: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    bounds.0 = Rect::from_corners(at(0.5, 16.5), at(29.5, 1.0));
    bounds.1 = Rect::from_corners(at(0.0, H as f32), at(W as f32, 0.0));
    let start = at(11.0, 7.5);
    commands.spawn(walk::avatar_bundle(art.man.clone(), start));
    walk::spawn_prompt(&mut commands, &fonts);
    if let Ok((mut tf, mut proj)) = cam.single_mut() {
        tf.translation = start.extend(tf.translation.z);
        if let Projection::Orthographic(o) = proj.as_mut() {
            o.scale = 0.72;
        }
    }
    // Spots that never move.
    for (kind, x, y) in [
        (SpotKind::Cabin, 9.0, 5.0),
        (SpotKind::Barn, 16.0, 5.0),
        (SpotKind::Fields, 16.5, 11.5),
        (SpotKind::Fence, 8.3, 11.5),
        (SpotKind::Woods, 2.5, 9.0),
        (SpotKind::Creek, 14.0, 15.6),
        (SpotKind::Yard, 5.5, 6.0),
        (SpotKind::Road, 27.0, 1.8),
    ] {
        commands.spawn((
            Transform::from_translation(at(x, y).extend(0.0)),
            GlobalTransform::default(),
            Spot(kind),
            OnFoot,
        ));
    }
}

/// Redraw the claim when the day turns or the player does something.
pub fn rebuild(
    mut commands: Commands,
    sim: Res<Sim>,
    art: Res<Art>,
    mut last: Local<(u32, usize)>,
    living: Query<Entity, With<Living>>,
) {
    let world = &sim.0;
    let key = (world.day.0, world.events.len());
    if *last == key && !living.is_empty() {
        return;
    }
    *last = key;
    for e in &living {
        commands.entity(e).despawn();
    }
    draw(&mut commands, world, &art);
}

fn sprite(commands: &mut Commands, img: &Handle<Image>, pos: Vec2, size: Vec2, z: f32) {
    commands.spawn((
        Sprite {
            image: img.clone(),
            color: PAPER,
            custom_size: Some(size),
            ..default()
        },
        Transform::from_translation(pos.extend(z)),
        Living,
        OnFoot,
    ));
}

fn block(commands: &mut Commands, color: Color, pos: Vec2, size: Vec2, z: f32) {
    commands.spawn((
        Sprite {
            color,
            custom_size: Some(size),
            ..default()
        },
        Transform::from_translation(pos.extend(z)),
        Living,
        OnFoot,
    ));
}

fn draw(commands: &mut Commands, world: &World, art: &Art) {
    let farm = world.families[0].farm;
    let hh = &world.families[0].stores;
    let month = world.day.month();

    // Ground: this season's grass (scorched if it burned), a road, the creek.
    let ground = scenery::terrain_color(Terrain::Prairie, farm, world);
    for y in 0..H {
        for x in 0..W {
            let j = ((x * 7 + y * 13) % 5) as f32 * 0.008;
            let c = ground.to_srgba();
            block(
                commands,
                Color::srgb(c.red + j, c.green + j, c.blue + j * 0.5),
                at(x as f32 + 0.5, y as f32 + 0.5),
                Vec2::splat(T),
                0.0,
            );
        }
    }
    let road = scenery::terrain_color(Terrain::Road, farm, world);
    block(
        commands,
        road,
        at(15.0, 1.5),
        Vec2::new(W as f32 * T, T * 0.9),
        0.1,
    );
    let water = scenery::terrain_color(Terrain::River, farm, world);
    block(
        commands,
        water,
        at(15.0, 16.4),
        Vec2::new(W as f32 * T, T * 1.6),
        0.1,
    );
    sprite(
        commands,
        &art.sign,
        at(27.8, 2.2),
        Vec2::new(22.0, 30.0),
        2.0,
    );

    // The fields: one plot per acre, dressed for the season.
    let acres = hh.acres.min(28) as i32;
    let planted = hh.work.planted_on.is_some();
    for a in 0..acres {
        let (cx, cy) = (10.0 + (a % 14) as f32, 10.0 + (a / 14) as f32 * 1.6);
        let soil = if hh.work.plowed > 0.0 || planted {
            Color::srgb(0.36, 0.26, 0.17)
        } else {
            Color::srgb(0.46, 0.40, 0.26)
        };
        block(
            commands,
            soil,
            at(cx, cy),
            Vec2::new(T * 0.94, T * 1.4),
            0.2,
        );
        let crop = match month {
            5 | 6 if planted => Some((&art.sprout, Vec2::new(16.0, 32.0))),
            7..=9 if planted => Some((&art.corn, Vec2::new(20.0, 44.0))),
            10 | 11 if hh.work.standing > 0.0 => Some((&art.corn, Vec2::new(20.0, 44.0))),
            10..=12 if planted || hh.work.crop > 0.0 => Some((&art.shock, Vec2::new(20.0, 30.0))),
            _ => None,
        };
        if let Some((img, size)) = crop {
            for k in 0..2 {
                sprite(
                    commands,
                    img,
                    at(cx - 0.2 + k as f32 * 0.4, cy - 0.1),
                    size,
                    0.3 + k as f32 * 0.01,
                );
            }
        }
    }

    // The fence around them: as many rails standing as the fence is sound.
    let rails = homestead::has(world, 0, Improvement::RailFence);
    let spans: Vec<Vec2> = (0..15)
        .map(|i| at(9.5 + i as f32, 8.9))
        .chain((0..15).map(|i| at(9.5 + i as f32, 14.3)))
        .chain((0..4).map(|i| at(8.8, 9.7 + i as f32 * 1.2)))
        .collect();
    let sound = (hh.work.fences * spans.len() as f32).round() as usize;
    for (i, p) in spans.iter().enumerate() {
        // Broken spans scattered, not all at one end.
        let standing = (i * 7) % spans.len() < sound;
        if standing {
            sprite(
                commands,
                &art.rail,
                *p,
                Vec2::new(32.0, if rails { 12.0 } else { 9.0 }),
                0.4,
            );
        }
    }

    // Buildings: cabin, barn or its ashes, the hay rick.
    sprite(
        commands,
        &art.cabin,
        at(9.0, 4.4),
        Vec2::new(96.0, 80.0),
        1.0,
    );
    let barn = if world.families[0].barn_standing {
        &art.barn
    } else {
        &art.ruin
    };
    sprite(commands, barn, at(16.0, 4.3), Vec2::new(104.0, 88.0), 1.0);
    if hh.work.hay >= 0.5 {
        sprite(
            commands,
            &art.rick,
            at(19.5, 6.2),
            Vec2::new(48.0, 42.0),
            1.1,
        );
    }
    // Improvements where they stand; stakes where they're only planned.
    for (imp, x, y, img, size) in [
        (
            Improvement::Well,
            5.0,
            7.5,
            &art.well,
            Vec2::new(40.0, 46.0),
        ),
        (
            Improvement::Cellar,
            5.5,
            4.2,
            &art.cellar,
            Vec2::new(52.0, 32.0),
        ),
        (
            Improvement::Smokehouse,
            12.6,
            4.4,
            &art.smokehouse,
            Vec2::new(44.0, 44.0),
        ),
        (
            Improvement::Crib,
            21.5,
            5.0,
            &art.crib,
            Vec2::new(44.0, 44.0),
        ),
    ] {
        if homestead::has(world, 0, imp) {
            sprite(commands, img, at(x, y), size, 1.0);
        } else {
            block(
                commands,
                Color::srgb(0.30, 0.22, 0.14),
                at(x, y + 0.5),
                Vec2::new(3.0, 10.0),
                0.5,
            );
        }
    }

    // The timber along the creek: fewer trees as the nearest stand is cut.
    let left = world
        .map
        .nearest_timber(farm)
        .map_or(0, |s| world.map.loads_left(s)) as f32
        / TIMBER_LOADS as f32;
    for i in 0..22 {
        let (x, y) = (
            0.8 + (i % 4) as f32 * 1.0 + (i / 4 % 2) as f32 * 0.4,
            3.0 + (i / 4) as f32 * 2.2,
        );
        let standing = (i as f32 / 22.0) < left.max(0.15);
        let img = if standing { &art.tree } else { &art.stump };
        let size = if standing {
            Vec2::new(40.0, 56.0)
        } else {
            Vec2::new(24.0, 16.0)
        };
        sprite(commands, img, at(x, y), size, 1.0 - y * 0.001);
    }
    for i in 0..8 {
        sprite(
            commands,
            &art.tree,
            at(6.0 + i as f32 * 3.1, 14.6),
            Vec2::new(36.0, 50.0),
            1.2,
        );
    }

    // Kin about the place.
    for (k, n) in world
        .living()
        .filter(|n| n.family == 0 && n.id != PLAYER)
        .enumerate()
    {
        let img = scenery::figure(art, world, n.id);
        let size = if LifeStage::of(n.age) == LifeStage::Child {
            Vec2::new(20.0, 40.0)
        } else {
            Vec2::new(28.0, 56.0)
        };
        commands.spawn((
            Sprite {
                image: img,
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(at(8.0 + k as f32 * 1.5, 7.0).extend(5.0)),
            Kin {
                id: n.id,
                phase: k as f32 * 1.9,
            },
            Spot(SpotKind::Person(n.id)),
            Living,
            OnFoot,
        ));
    }
}

/// Kin potter about the yard.
pub fn kin(time: Res<Time>, mut q: Query<(&Kin, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (k, mut tf) in &mut q {
        let home = at(9.0 + (k.id % 3) as f32 * 1.4, 7.2);
        let off = Vec2::new(
            (t * 0.4 + k.phase).sin() * 50.0,
            (t * 0.3 + k.phase).cos() * 18.0,
        );
        tf.translation = (home + off).extend(5.0 - (home.y + off.y) * 0.001);
    }
}
