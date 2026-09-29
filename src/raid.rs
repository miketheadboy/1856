//! A neighbor's place at night (PLAN Phase B.2). Lanterns sweep the yard
//! from the ones sitting up; a dog walks its round; the moon decides how far
//! a lantern carries. Crawl (Shift) to keep low. Walk up to what you came
//! for and hold E. Then get back to the road.
//!
//! Whoever's light finds you long enough has seen you, and that goes to the
//! sim as a true belief. Nobody else knows a thing; they'll guess, and
//! they'll guess whoever they already hate.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::*;
use bleeding_kansas::sim::action::{self, Objective, RaidPlan};
use bleeding_kansas::sim::world::NpcId;

use crate::scenery::Art;
use crate::ui::{LIGHT, Toast};
use crate::walk::OnFoot;
use crate::{Fonts, OXBLOOD, Screen, Sim, text};

pub const ORIGIN: Vec2 = Vec2::new(12000.0, 0.0);
const SIZE: Vec2 = Vec2::new(900.0, 560.0);
const WALK: f32 = 125.0;
const CRAWL: f32 = 52.0;
const REACH: f32 = 42.0;
const SCALE: f32 = 0.78;
/// The way you came.
const EXIT: Vec2 = Vec2::new(40.0, 30.0);
const CABIN: Vec2 = Vec2::new(450.0, 400.0);
const DOOR: Vec2 = Vec2::new(450.0, 345.0);

#[derive(Resource, Default)]
pub struct Raid {
    pub plan: Option<RaidPlan>,
    alarm: f32,
    spotted_by: Vec<NpcId>,
    roused: bool,
    done: Option<Objective>,
    work: Option<(Objective, f32)>,
    t: f32,
    fire_at: Option<f32>,
    over: bool,
}

#[derive(Component)]
pub struct Sneak;
#[derive(Component)]
pub struct Cone {
    who: NpcId,
    reach: f32,
    base: f32,
    angle: f32,
    phase: f32,
}
#[derive(Component)]
pub struct Dog {
    angle: f32,
}
#[derive(Component)]
pub struct Flames;
#[derive(Component)]
pub struct Alarm;
#[derive(Component)]
pub struct RaidText;
#[derive(Component)]
pub struct Goal(Objective, Vec2);

const HALF: f32 = 0.42;

fn at(p: Vec2) -> Vec3 {
    (ORIGIN + p).extend(2.0 - p.y * 0.001)
}

pub fn enter(
    mut commands: Commands,
    art: Res<Art>,
    fonts: Res<Fonts>,
    sim: Res<Sim>,
    mut raid: ResMut<Raid>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut cam: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    let Some(plan) = raid.plan.clone() else {
        return;
    };
    *raid = Raid {
        plan: Some(plan.clone()),
        ..default()
    };
    let world = &sim.0;
    let moon = plan.moon;
    let dim = 0.22 + 0.55 * moon;
    let paper = Color::srgb(0.93 * dim, 0.89 * dim, 0.80 * dim);

    // Night grass, the yard, the lane.
    commands.spawn((
        Sprite {
            color: Color::srgb(0.03 + 0.05 * moon, 0.06 + 0.07 * moon, 0.05 + 0.08 * moon),
            custom_size: Some(SIZE + Vec2::splat(800.0)),
            ..default()
        },
        Transform::from_translation((ORIGIN + SIZE / 2.0).extend(0.0)),
        OnFoot,
    ));
    commands.spawn((
        Sprite {
            color: Color::srgb(0.05 + 0.05 * moon, 0.06 + 0.05 * moon, 0.05 + 0.04 * moon),
            custom_size: Some(Vec2::new(300.0, 120.0)),
            ..default()
        },
        Transform::from_translation((ORIGIN + Vec2::new(450.0, 300.0)).extend(0.1)),
        OnFoot,
    ));
    commands.spawn((
        Sprite {
            color: Color::srgb(0.10 + 0.05 * moon, 0.08 + 0.05 * moon, 0.06 + 0.04 * moon),
            custom_size: Some(Vec2::new(440.0, 22.0)),
            ..default()
        },
        Transform::from_translation((ORIGIN + Vec2::new(200.0, 30.0)).extend(0.1))
            .with_rotation(Quat::from_rotation_z(0.25)),
        OnFoot,
    ));

    let mut put = |img: &Handle<Image>, p: Vec2, size: Vec2| {
        commands.spawn((
            Sprite {
                image: img.clone(),
                color: paper,
                custom_size: Some(size),
                ..default()
            },
            Transform::from_translation(at(p)),
            OnFoot,
        ));
    };
    put(&art.cabin, CABIN, Vec2::new(150.0, 125.0));
    put(
        if plan.barn { &art.barn } else { &art.ruin },
        Vec2::new(740.0, 380.0),
        Vec2::new(160.0, 135.0),
    );
    if plan.well {
        put(&art.well, Vec2::new(280.0, 300.0), Vec2::new(42.0, 48.0));
    }
    if plan.hay {
        put(&art.rick, Vec2::new(830.0, 190.0), Vec2::new(72.0, 63.0));
    }
    for i in 0..plan.cattle {
        put(
            &art.cow,
            Vec2::new(600.0 + 48.0 * (i % 3) as f32, 130.0 + 30.0 * (i / 3) as f32),
            Vec2::new(44.0, 28.0),
        );
    }
    // The fence along the south line, with gaps where it's down.
    let rails = (14.0 * plan.fences).round() as i32;
    for i in 0..14 {
        if i < rails {
            put(
                &art.rail,
                Vec2::new(150.0 + i as f32 * 50.0, 70.0),
                Vec2::new(48.0, 16.0),
            );
        }
    }
    put(
        &art.sign,
        EXIT + Vec2::new(-10.0, 30.0),
        Vec2::new(24.0, 32.0),
    );
    put(&art.tree, Vec2::new(90.0, 420.0), Vec2::new(60.0, 84.0));
    put(&art.tree, Vec2::new(150.0, 480.0), Vec2::new(50.0, 70.0));
    put(&art.tree, Vec2::new(860.0, 500.0), Vec2::new(56.0, 78.0));

    // Lit windows: somebody's up.
    let awake = plan.watchers.iter().filter(|w| w.awake).count();
    commands.spawn((
        Sprite {
            image: art.glow.clone(),
            color: Color::srgba(1.0, 0.78, 0.4, if awake > 0 { 0.85 } else { 0.25 }),
            custom_size: Some(Vec2::splat(90.0)),
            ..default()
        },
        Transform::from_translation((ORIGIN + CABIN + Vec2::new(-20.0, -10.0)).extend(3.0)),
        OnFoot,
    ));

    // What you could do here, where.
    let goals = [
        (Objective::Burn, Vec2::new(740.0, 320.0)),
        (Objective::Foul, Vec2::new(280.0, 280.0)),
        (Objective::Shoot, Vec2::new(620.0, 115.0)),
        (Objective::Drive, Vec2::new(690.0, 120.0)),
        (Objective::Cut, Vec2::new(300.0, 70.0)),
        (Objective::Wet, Vec2::new(830.0, 160.0)),
        (Objective::Listen, Vec2::new(525.0, 385.0)),
        (Objective::Steal, Vec2::new(430.0, 395.0)),
    ];
    for (o, p) in goals {
        if plan.objectives.contains(&o) {
            commands.spawn((Transform::default(), Goal(o, p), OnFoot));
        }
    }

    // Lanterns: one cone per watcher; the sleeping ones light theirs when
    // the house wakes.
    let mesh = meshes.add(CircularSector::new(1.0, HALF));
    for (i, w) in plan.watchers.iter().enumerate() {
        let reach = (120.0 + 170.0 * w.alertness) * (0.55 + 0.8 * moon);
        commands.spawn((
            Mesh2d(mesh.clone()),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::srgba(
                1.0, 0.85, 0.45, 0.16,
            )))),
            Transform::from_translation(at(DOOR).with_z(4.0)).with_scale(Vec3::splat(reach)),
            Visibility::Hidden,
            Cone {
                who: w.id,
                reach,
                base: -FRAC_PI_2 + (i as f32 - 1.0) * 0.5,
                angle: -FRAC_PI_2,
                phase: i as f32 * 2.1,
            },
            OnFoot,
        ));
    }
    if plan.dog {
        commands.spawn((
            Sprite {
                image: art.dog.clone(),
                color: paper,
                custom_size: Some(Vec2::new(32.0, 20.0)),
                ..default()
            },
            Transform::from_translation(at(CABIN)),
            Dog { angle: 0.0 },
            OnFoot,
        ));
    }
    commands.spawn((
        Sprite {
            image: art.glow.clone(),
            color: Color::srgba(1.0, 0.45, 0.1, 0.0),
            custom_size: Some(Vec2::splat(260.0)),
            ..default()
        },
        Transform::from_translation((ORIGIN + Vec2::new(740.0, 390.0)).extend(6.0)),
        Flames,
        OnFoot,
    ));

    // You.
    commands.spawn((
        Sprite {
            image: art.man.clone(),
            color: Color::srgb(0.55, 0.55, 0.6),
            custom_size: Some(Vec2::new(30.0, 60.0)),
            ..default()
        },
        Transform::from_translation(at(EXIT + Vec2::new(30.0, 20.0))),
        Sneak,
        OnFoot,
    ));

    // The meter and the word.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(40.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-160.0)),
                width: Val::Px(320.0),
                height: Val::Px(10.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(LIGHT.with_alpha(0.5)),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            GlobalZIndex(12),
            OnFoot,
        ))
        .with_child((
            Node {
                width: Val::Percent(0.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(OXBLOOD),
            Alarm,
        ));
    commands.spawn((
        Text::new(""),
        text(&fonts.body, 16.0, LIGHT),
        TextLayout::new_with_justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(56.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            ..default()
        },
        GlobalZIndex(12),
        RaidText,
        OnFoot,
    ));
    if let Ok((mut tf, mut proj)) = cam.single_mut() {
        tf.translation = (ORIGIN + EXIT).extend(tf.translation.z);
        if let Projection::Orthographic(o) = proj.as_mut() {
            o.scale = SCALE;
        }
    }
    let _ = world;
}

fn in_cone(from: Vec2, angle: f32, reach: f32, p: Vec2) -> bool {
    let d = p - from;
    if d.length() > reach {
        return false;
    }
    let a = d.y.atan2(d.x);
    let mut diff = (a - angle).rem_euclid(2.0 * PI);
    if diff > PI {
        diff -= 2.0 * PI;
    }
    diff.abs() < HALF
}

#[allow(clippy::too_many_arguments)]
pub fn sneak(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut raid: ResMut<Raid>,
    mut sim: ResMut<Sim>,
    mut toast: ResMut<Toast>,
    mut next: ResMut<NextState<Screen>>,
    mut me: Query<(&mut Transform, &mut Sprite), With<Sneak>>,
    mut cones: Query<(&mut Cone, &mut Transform, &mut Visibility), Without<Sneak>>,
    mut dogs: Query<(&mut Dog, &mut Transform), (Without<Sneak>, Without<Cone>)>,
    mut flames: Query<&mut Sprite, (With<Flames>, Without<Sneak>)>,
    goals: Query<&Goal>,
    mut cam: Query<
        (&mut Transform, &mut Projection),
        (With<Camera2d>, Without<Sneak>, Without<Cone>, Without<Dog>),
    >,
    mut meter: Query<&mut Node, With<Alarm>>,
    mut words: Query<&mut Text, With<RaidText>>,
) {
    let Some(plan) = raid.plan.clone() else {
        return;
    };
    if raid.over {
        return;
    }
    let dt = time.delta_secs();
    raid.t += dt;
    let t = raid.t;
    let Ok((mut tf, mut sprite)) = me.single_mut() else {
        return;
    };
    let crawling = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let working = keys.pressed(KeyCode::KeyE);
    let mut pos = tf.translation.truncate() - ORIGIN;

    // Feet.
    let mut d = Vec2::ZERO;
    if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        d.x -= 1.0;
    }
    if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        d.x += 1.0;
    }
    if keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        d.y += 1.0;
    }
    if keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        d.y -= 1.0;
    }
    let driving = raid.done == Some(Objective::Drive);
    let mut speed = if crawling { CRAWL } else { WALK };
    if driving {
        speed *= 0.6;
    }
    if d != Vec2::ZERO && !working {
        pos = (pos + d.normalize() * speed * dt).clamp(Vec2::ZERO, SIZE);
        if d.x != 0.0 {
            sprite.flip_x = d.x < 0.0;
        }
    }
    tf.translation = at(pos).with_z(10.0);
    tf.scale = Vec3::new(1.0, if crawling { 0.45 } else { 1.0 }, 1.0);
    // Low in the grass you're a darker thing.
    sprite.color = if crawling {
        Color::srgb(0.35, 0.36, 0.38)
    } else {
        Color::srgb(0.6, 0.6, 0.65)
    };

    // The house.
    let fire_lit = raid.fire_at.is_some_and(|f| t - f > 1.2);
    if fire_lit || raid.alarm > 0.55 {
        if !raid.roused {
            toast.say("A light moves inside. The house is waking.");
        }
        raid.roused = true;
    }
    let exposure = if crawling { 0.45 } else { 1.2 }
        * (1.0 - 0.5 * plan.stealth)
        * if working { 1.4 } else { 1.0 };
    let mut seen_now: Option<NpcId> = None;
    for (mut c, mut ctf, mut vis) in &mut cones {
        let watcher = plan.watchers.iter().find(|w| w.id == c.who);
        // The ones asleep still get up: to see to the stock, to the privy,
        // to a noise. The alert get up more. Learn the rhythm.
        let stirring = watcher.is_some_and(|w| {
            let period = 10.0 + 9.0 * (1.0 - w.alertness);
            (t + 5.0 + c.phase * 3.0).rem_euclid(period) < 4.5
        });
        let up = watcher.is_some_and(|w| w.awake) || stirring || raid.roused;
        *vis = if up {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if !up {
            continue;
        }
        let from = if raid.roused {
            DOOR + Vec2::new(0.0, -60.0) + Vec2::new(c.phase.sin() * 60.0, 0.0)
        } else {
            DOOR
        };
        let sweep = c.base + 0.9 * (t * 0.55 + c.phase).sin();
        // Roused, they swing toward where they think you are.
        let goal = if raid.roused {
            let to = pos - from;
            to.y.atan2(to.x)
        } else {
            sweep
        };
        let turn = if raid.roused { 0.9 } else { 3.0 };
        let mut diff = (goal - c.angle).rem_euclid(2.0 * PI);
        if diff > PI {
            diff -= 2.0 * PI;
        }
        c.angle += diff.clamp(-turn * dt, turn * dt);
        let reach = c.reach * if raid.roused { 1.25 } else { 1.0 };
        ctf.translation = at(from).with_z(4.0);
        ctf.rotation = Quat::from_rotation_z(c.angle - FRAC_PI_2);
        ctf.scale = Vec3::splat(reach);
        if in_cone(from, c.angle, reach, pos) {
            seen_now = Some(c.who);
        }
    }
    let mut barking = false;
    for (mut dog, mut dtf) in &mut dogs {
        let dp = dtf.translation.truncate() - ORIGIN;
        let next_p = if raid.roused || raid.alarm > 0.3 {
            dp + (pos - dp).normalize_or_zero() * 150.0 * dt
        } else {
            dog.angle += 0.45 * dt;
            CABIN + Vec2::new(dog.angle.cos() * 190.0, dog.angle.sin() * 150.0 - 60.0)
        };
        let nose = if crawling { 45.0 } else { 80.0 };
        if next_p.distance(pos) < nose {
            barking = true;
        }
        dtf.translation = at(next_p).with_z(9.0);
    }
    if let Some(who) = seen_now {
        raid.alarm += exposure * dt;
        if raid.alarm >= 1.0 {
            if !raid.spotted_by.contains(&who) {
                raid.spotted_by.push(who);
                toast.say(format!(
                    "\"Who's there?\" {} has seen you plain.",
                    sim.0.name(who)
                ));
            }
            raid.alarm = 0.6;
            raid.roused = true;
        }
    } else {
        raid.alarm = (raid.alarm - 0.25 * dt).max(0.0);
    }
    if barking {
        // The dog can't name you, but it wakes the ones who can.
        raid.alarm = (raid.alarm + 0.9 * dt).min(0.99);
        if !raid.roused {
            toast.say("The dog's on you, barking fit to raise the dead.");
        }
        raid.roused = true;
    }

    // The work.
    let near = goals
        .iter()
        .filter(|g| raid.done.is_none() && g.1.distance(pos) < REACH)
        .min_by(|a, b| a.1.distance(pos).total_cmp(&b.1.distance(pos)))
        .map(|g| g.0);
    let mut line = String::new();
    match (near, working) {
        (Some(o), true) => {
            let p = match raid.work {
                Some((w, p)) if w == o => p + dt,
                _ => dt,
            };
            raid.work = Some((o, p));
            if p >= o.seconds() {
                raid.done = Some(o);
                raid.work = None;
                match o {
                    Objective::Burn => raid.fire_at = Some(t),
                    Objective::Shoot => {
                        raid.roused = true;
                        toast.say(
                            "The shot rolls across the bottoms. Every lamp in the house comes up.",
                        );
                    }
                    Objective::Listen => {
                        toast.say("You heard enough through the chinking to hang a man, or at least embarrass him.");
                    }
                    _ => {}
                }
            } else {
                line = format!("{}... {:.0}%", o.label(), 100.0 * p / o.seconds());
            }
        }
        (Some(o), false) => {
            line = format!("Hold E: {}", o.label());
        }
        _ => {}
    }
    if line.is_empty() {
        line = match raid.done {
            Some(o) => format!(
                "Done: {}. Get back to the road.{}",
                o.label(),
                if raid.spotted_by.is_empty() {
                    ""
                } else {
                    " They saw you."
                }
            ),
            None => format!(
                "The {} place. Shift crawls. {}",
                plan.surname,
                if raid.roused {
                    "They're up. Get out or get it done."
                } else {
                    "Stay out of the light."
                }
            ),
        };
    }
    if let Ok(mut w) = words.single_mut()
        && **w != line
    {
        **w = line;
    }
    if let Ok(mut n) = meter.single_mut() {
        n.width = Val::Percent(100.0 * raid.alarm.min(1.0));
    }
    if let Ok(mut f) = flames.single_mut() {
        let a = raid.fire_at.map_or(0.0, |f| {
            ((t - f) * 0.5).min(0.85) * (0.85 + 0.15 * (t * 11.0).sin())
        });
        f.color = Color::srgba(1.0, 0.45, 0.1, a);
    }

    // Home by the road.
    if pos.distance(EXIT) < 36.0 && raid.t > 1.0 && (raid.done.is_some() || !working) {
        let leaving = raid.done.is_some() || keys.just_pressed(KeyCode::KeyE);
        if leaving || raid.roused {
            finish(&mut raid, &mut sim, &mut toast, &plan);
            next.set(Screen::Claim);
            return;
        }
        if raid.done.is_none()
            && let Ok(mut w) = words.single_mut()
        {
            **w = "The road home. E to go with nothing done.".into();
        }
    }

    // The camera stays with you.
    if let Ok((mut c, mut proj)) = cam.single_mut() {
        if let Projection::Orthographic(o) = proj.as_mut()
            && o.scale != SCALE
        {
            o.scale = SCALE;
        }
        let half = Vec2::new(640.0, 360.0) * SCALE;
        let goal = (ORIGIN + pos).clamp(
            ORIGIN + half - Vec2::splat(60.0),
            ORIGIN + SIZE - half + Vec2::splat(60.0),
        );
        let now = c.translation.truncate().lerp(goal, (5.0 * dt).min(1.0));
        c.translation.x = now.x;
        c.translation.y = now.y;
    }
}

fn finish(raid: &mut Raid, sim: &mut Sim, toast: &mut Toast, plan: &RaidPlan) {
    raid.over = true;
    let world = &mut sim.0;
    let before = world.events.len();
    action::raid(world, plan.target, raid.done, raid.spotted_by.clone());
    let seen = if raid.spotted_by.is_empty() {
        "Nobody saw your face.".to_string()
    } else {
        format!(
            "{} saw you.",
            raid.spotted_by
                .iter()
                .map(|&w| world.name(w).to_string())
                .collect::<Vec<_>>()
                .join(" and ")
        )
    };
    let what = match raid.done {
        Some(o) => format!("You got it done: {}.", o.label()),
        None => "You came home with nothing but wet knees.".into(),
    };
    let _ = before;
    toast.say(format!("{what} {seen}"));
    raid.plan = None;
}
