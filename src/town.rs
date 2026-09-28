//! A town street: Lawrence, Franklin, or Lecompton. Facades along a muddy
//! road; each door is a spot. The people on the street are the ones the sim
//! says were seen in town today (the alibis walk around where you can see
//! them).

use bevy::prelude::*;
use bleeding_kansas::sim::civic::Project;
use bleeding_kansas::sim::institutions::Venue;
use bleeding_kansas::sim::psyche::LifeStage;
use bleeding_kansas::sim::world::{Faction, PLAYER};

use crate::scenery::{self, Art};
use crate::walk::{self, Bounds, OnFoot, Spot, SpotKind};
use crate::{Fonts, Sim, TownId};

pub const ORIGIN: Vec2 = Vec2::new(8000.0, 0.0);
const PAPER: Color = Color::srgb(0.93, 0.89, 0.80);
const SPACING: f32 = 170.0;

#[derive(Component)]
pub struct Walker {
    phase: f32,
    home: f32,
}

impl TownId {
    pub fn name(self) -> &'static str {
        match self {
            TownId::Lawrence => "Lawrence",
            TownId::Franklin => "Franklin",
            TownId::Lecompton => "Lecompton",
        }
    }

    /// Whose town it is.
    fn side(self) -> Faction {
        match self {
            TownId::Lawrence => Faction::FreeState,
            _ => Faction::ProSlavery,
        }
    }
}

pub fn enter(
    mut commands: Commands,
    art: Res<Art>,
    fonts: Res<Fonts>,
    sim: Res<Sim>,
    town: Res<TownId>,
    mut bounds: ResMut<Bounds>,
    mut cam: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    let world = &sim.0;
    let open = |v: Venue| world.institutions.open[v.index()];
    let mut row: Vec<(SpotKind, &Handle<Image>, &str, Vec2)> = match *town {
        TownId::Lawrence => vec![
            (
                SpotKind::Church,
                &art.church,
                "PLYMOUTH CHURCH",
                Vec2::new(120.0, 164.0),
            ),
            (
                SpotKind::Hotel,
                if open(Venue::FreeStateHotel) {
                    &art.hotel
                } else {
                    &art.ruin
                },
                "FREE STATE HOTEL",
                if open(Venue::FreeStateHotel) {
                    Vec2::new(150.0, 209.0)
                } else {
                    Vec2::new(150.0, 127.0)
                },
            ),
            (
                SpotKind::PostOffice,
                &art.office,
                "POST OFFICE",
                Vec2::new(130.0, 98.0),
            ),
            (
                SpotKind::Barbershop,
                &art.store,
                "BARBER & BATHS",
                Vec2::new(130.0, 98.0),
            ),
            (
                SpotKind::LandAgent,
                &art.office,
                "LAND AGENT",
                Vec2::new(130.0, 98.0),
            ),
        ],
        TownId::Franklin => vec![
            (
                SpotKind::Store,
                &art.store,
                "DUNMORE'S",
                Vec2::new(160.0, 120.0),
            ),
            (
                SpotKind::Saloon,
                &art.saloon,
                "JACK OF HEARTS",
                Vec2::new(160.0, 120.0),
            ),
        ],
        TownId::Lecompton => vec![
            (
                SpotKind::LandOffice,
                &art.office,
                "U.S. LAND OFFICE",
                Vec2::new(150.0, 113.0),
            ),
            (
                SpotKind::Justice,
                &art.office,
                "JUSTICE OF THE PEACE",
                Vec2::new(130.0, 98.0),
            ),
        ],
    };
    if *town == TownId::Lawrence && world.civic.built(Project::Lyceum) {
        row.push((
            SpotKind::Lyceum,
            &art.lyceum,
            "LYCEUM HALL",
            Vec2::new(150.0, 138.0),
        ));
    }
    if *town == TownId::Lecompton && crate::cmds::election(world).is_some() {
        row.push((
            SpotKind::Polls,
            &art.ballot,
            "THE POLLS",
            Vec2::new(80.0, 67.0),
        ));
    }

    let width = (row.len() as f32 + 1.0) * SPACING;
    bounds.0 = Rect::from_corners(
        ORIGIN + Vec2::new(0.0, -110.0),
        ORIGIN + Vec2::new(width, -40.0),
    );
    // Tall enough for the hotel's three stories, low enough for the street.
    bounds.1 = Rect::from_corners(
        ORIGIN + Vec2::new(-200.0, -170.0),
        ORIGIN + Vec2::new(width + 200.0, 270.0),
    );

    // Sky, street, boardwalk.
    let ground = scenery::terrain_color(
        bleeding_kansas::sim::geography::Terrain::Road,
        world.families[0].farm,
        world,
    );
    let grass = scenery::terrain_color(
        bleeding_kansas::sim::geography::Terrain::Prairie,
        world.families[0].farm,
        world,
    );
    for (color, y, h) in [
        (grass, 150.0, 300.0),
        (ground, -75.0, 150.0),
        (Color::srgb(0.40, 0.30, 0.20), -12.0, 14.0),
        (grass, -230.0, 160.0),
    ] {
        commands.spawn((
            Sprite {
                color,
                custom_size: Some(Vec2::new(width + 800.0, h)),
                ..default()
            },
            Transform::from_translation((ORIGIN + Vec2::new(width / 2.0, y)).extend(0.0)),
            OnFoot,
        ));
    }
    for (i, (kind, img, sign, size)) in row.iter().enumerate() {
        let x = SPACING * (i as f32 + 1.0);
        let size = *size;
        commands.spawn((
            Sprite {
                image: (*img).clone(),
                color: PAPER,
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation((ORIGIN + Vec2::new(x, size.y / 2.0 - 6.0)).extend(1.0)),
            OnFoot,
        ));
        commands.spawn((
            Text2d::new(*sign),
            TextFont {
                font: fonts.display.clone(),
                font_size: 15.0,
                ..default()
            },
            TextColor(Color::srgb(0.10, 0.08, 0.06)),
            Transform::from_translation((ORIGIN + Vec2::new(x, size.y + 8.0)).extend(2.0)),
            OnFoot,
        ));
        commands.spawn((
            Transform::from_translation((ORIGIN + Vec2::new(x, -30.0)).extend(0.0)),
            GlobalTransform::default(),
            Spot(*kind),
            OnFoot,
        ));
    }
    // The way out.
    commands.spawn((
        Sprite {
            image: art.sign.clone(),
            color: PAPER,
            custom_size: Some(Vec2::new(28.0, 38.0)),
            ..default()
        },
        Transform::from_translation((ORIGIN + Vec2::new(30.0, -20.0)).extend(1.0)),
        Spot(SpotKind::TownEdge),
        OnFoot,
    ));

    // Who's in town today: the people with an alibi, the town's own first.
    let mut folk: Vec<_> = world
        .living()
        .filter(|n| n.id != PLAYER && LifeStage::of(n.age) != LifeStage::Child)
        .filter(|n| n.alibi == Some(world.day) || about_town(n.id, world.day.0))
        .map(|n| (n.faction == town.side(), n.id))
        .collect();
    folk.sort();
    folk.reverse();
    for (i, (_, id)) in folk.iter().take(7).enumerate() {
        commands.spawn((
            Sprite {
                image: scenery::figure(&art, world, *id),
                custom_size: Some(Vec2::new(28.0, 56.0)),
                ..default()
            },
            Transform::from_translation(
                (ORIGIN + Vec2::new(120.0 + i as f32 * 130.0, -70.0)).extend(5.0),
            ),
            Walker {
                phase: i as f32 * 2.3,
                home: 120.0 + i as f32 * 130.0,
            },
            Spot(SpotKind::Person(*id)),
            OnFoot,
        ));
    }

    let start = ORIGIN + Vec2::new(60.0, -70.0);
    commands.spawn(walk::avatar_bundle(art.man.clone(), start));
    walk::spawn_prompt(&mut commands, &fonts);
    if let Ok((mut tf, mut proj)) = cam.single_mut() {
        tf.translation = start.extend(tf.translation.z);
        if let Projection::Orthographic(o) = proj.as_mut() {
            o.scale = 0.72;
        }
    }
}

/// Some folks are just about town on a given day. A view-side hash, not the
/// sim's RNG: it decides who you see, never what happens.
fn about_town(id: u32, day: u32) -> bool {
    (id.wrapping_mul(2_654_435_761) ^ day.wrapping_mul(40_503)).is_multiple_of(5)
}

/// People stroll the street.
pub fn stroll(time: Res<Time>, mut q: Query<(&Walker, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (w, mut tf) in &mut q {
        tf.translation.x = ORIGIN.x + w.home + (t * 0.25 + w.phase).sin() * 60.0;
        tf.translation.y = ORIGIN.y - 70.0 + (t * 0.2 + w.phase).cos() * 18.0;
        tf.translation.z = 5.0 - tf.translation.y * 0.001;
    }
}
