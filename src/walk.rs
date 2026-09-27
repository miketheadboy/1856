//! On foot: the close views (your claim, a town street). You walk; the things
//! around you are spots; the nearest one within reach answers to E (or a
//! click) with a command window of what it lets you do today.

use bevy::prelude::*;
use bleeding_kansas::sim::civic::Project;
use bleeding_kansas::sim::economy::Choice;
use bleeding_kansas::sim::institutions::Venue;
use bleeding_kansas::sim::life::Activity;
use bleeding_kansas::sim::world::{NpcId, PLAYER, World};

use crate::cmds::{self, Cmd, MenuSpec, Sub, item, when};
use crate::ui::{LIGHT, Menu, Overlay, blocking};
use crate::{Fonts, Screen, Sim};

/// How far away you can reach a thing.
const REACH: f32 = 46.0;
const SPEED: f32 = 150.0;
/// Camera scale on foot.
pub const CLOSE: f32 = 0.62;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpotKind {
    Cabin,
    Barn,
    Fields,
    Fence,
    Woods,
    Creek,
    Yard,
    Road,
    Person(NpcId),
    Church,
    Hotel,
    PostOffice,
    Barbershop,
    LandAgent,
    Lyceum,
    Store,
    Saloon,
    LandOffice,
    Justice,
    Polls,
    TownEdge,
}

impl SpotKind {
    pub fn name(self, world: &World) -> String {
        match self {
            SpotKind::Cabin => "the cabin".into(),
            SpotKind::Barn => "the barn".into(),
            SpotKind::Fields => "the fields".into(),
            SpotKind::Fence => "the fence".into(),
            SpotKind::Woods => "the timber".into(),
            SpotKind::Creek => "the creek".into(),
            SpotKind::Yard => "the building ground".into(),
            SpotKind::Road => "the road".into(),
            SpotKind::Person(id) => world.name(id).to_string(),
            SpotKind::Church => "Plymouth Church".into(),
            SpotKind::Hotel => "the Free State Hotel".into(),
            SpotKind::PostOffice => "the post office".into(),
            SpotKind::Barbershop => "the barbershop".into(),
            SpotKind::LandAgent => "the land agent".into(),
            SpotKind::Lyceum => "the lyceum".into(),
            SpotKind::Store => "Dunmore's".into(),
            SpotKind::Saloon => "the Jack of Hearts".into(),
            SpotKind::LandOffice => "the land office".into(),
            SpotKind::Justice => "the justice of the peace".into(),
            SpotKind::Polls => "the polls".into(),
            SpotKind::TownEdge => "the road out".into(),
        }
    }
}

#[derive(Component)]
pub struct Spot(pub SpotKind);

#[derive(Component)]
pub struct Avatar;

/// Everything spawned for a close view; despawned on the way out.
#[derive(Component)]
pub struct OnFoot;

#[derive(Component)]
pub struct Prompt;

/// Where you can walk, and the whole scene the camera may show, in world
/// coordinates.
#[derive(Resource, Default)]
pub struct Bounds(pub Rect, pub Rect);

pub fn spawn_prompt(commands: &mut Commands, fonts: &Fonts) {
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: fonts.body.clone(),
            font_size: 15.0,
            ..default()
        },
        TextColor(LIGHT),
        Transform::from_xyz(0.0, 0.0, 30.0),
        Prompt,
        OnFoot,
    ));
}

pub fn despawn(mut commands: Commands, q: Query<Entity, With<OnFoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

#[allow(clippy::too_many_arguments)]
pub fn walk(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    menu: Res<Menu>,
    overlay: Res<Overlay>,
    bounds: Res<Bounds>,
    mut avatar: Query<(&mut Transform, &mut Sprite), With<Avatar>>,
    mut cam: Query<(&mut Transform, &mut Projection), (With<Camera2d>, Without<Avatar>)>,
) {
    let Ok((mut tf, mut sprite)) = avatar.single_mut() else {
        return;
    };
    if !blocking(&menu, &overlay) {
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
        if d != Vec2::ZERO {
            let p = tf.translation.truncate() + d.normalize() * SPEED * time.delta_secs();
            let b = bounds.0;
            let p = p.clamp(b.min, b.max);
            tf.translation.x = p.x;
            tf.translation.y = p.y;
            if d.x != 0.0 {
                sprite.flip_x = d.x < 0.0;
            }
        }
        // Figures further down the screen stand in front.
        tf.translation.z = 10.0 - tf.translation.y * 0.001;
    }
    if let Ok((mut c, mut proj)) = cam.single_mut() {
        // On foot the camera sits close.
        if let Projection::Orthographic(o) = proj.as_mut()
            && o.scale != CLOSE
        {
            o.scale = CLOSE;
        }
        // Keep the frame inside the scene.
        let half = Vec2::new(640.0, 360.0) * CLOSE;
        let v = bounds.1;
        let goal = tf.translation.truncate();
        let goal = Vec2::new(
            if v.width() > half.x * 2.0 {
                goal.x.clamp(v.min.x + half.x, v.max.x - half.x)
            } else {
                v.center().x
            },
            if v.height() > half.y * 2.0 {
                goal.y.clamp(v.min.y + half.y, v.max.y - half.y)
            } else {
                v.center().y
            },
        );
        let at = c
            .translation
            .truncate()
            .lerp(goal, (6.0 * time.delta_secs()).min(1.0));
        c.translation.x = at.x;
        c.translation.y = at.y;
    }
}

fn nearest(
    avatar: Vec2,
    spots: &Query<(&GlobalTransform, &Spot)>,
    within: f32,
) -> Option<SpotKind> {
    spots
        .iter()
        .map(|(t, s)| (t.translation().truncate().distance(avatar), s.0))
        .filter(|(d, _)| *d < within)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, k)| k)
}

/// E (or a click) on whatever is in reach.
#[allow(clippy::too_many_arguments)]
pub fn interact(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform)>,
    avatar: Query<&Transform, With<Avatar>>,
    spots: Query<(&GlobalTransform, &Spot)>,
    mut prompt: Query<(&mut Text2d, &mut Transform), (With<Prompt>, Without<Avatar>)>,
    mut menu: ResMut<Menu>,
    overlay: Res<Overlay>,
    sim: Res<Sim>,
    mut next: ResMut<NextState<Screen>>,
) {
    let Ok(me) = avatar.single() else {
        return;
    };
    let here = me.translation.truncate();
    let near = nearest(here, &spots, REACH);
    if let Ok((mut t, mut tf)) = prompt.single_mut() {
        t.0 = match near {
            Some(k) if !blocking(&menu, &overlay) => format!("E  {}", k.name(&sim.0)),
            _ => String::new(),
        };
        tf.translation = (here + Vec2::new(0.0, 46.0)).extend(30.0);
    }
    if blocking(&menu, &overlay) {
        return;
    }
    if keys.just_pressed(KeyCode::KeyM) {
        next.set(Screen::County);
        return;
    }
    let mut pick = if keys.any_just_pressed([KeyCode::KeyE, KeyCode::Enter]) {
        near
    } else {
        None
    };
    if mouse.just_pressed(MouseButton::Left)
        && let (Ok(w), Ok((c, ct))) = (windows.single(), camera.single())
        && let Some(cur) = w.cursor_position()
        && cur.y > 32.0
        && let Ok(p) = c.viewport_to_world_2d(ct, cur)
    {
        pick = nearest(p, &spots, 40.0);
    }
    if let Some(k) = pick {
        menu.open(spot_menu(&sim.0, k));
    }
}

/// R: turn in early.
pub fn rest_key(
    keys: Res<ButtonInput<KeyCode>>,
    menu: Res<Menu>,
    overlay: Res<Overlay>,
    mut sim: ResMut<Sim>,
    mut clock: ResMut<crate::Clock>,
    mut toast: ResMut<crate::ui::Toast>,
) {
    if keys.just_pressed(KeyCode::KeyR) && !blocking(&menu, &overlay) && sim.0.player_alive() {
        sim.0.advance_day();
        clock.timer.reset();
        toast.say("Morning comes the way it does.");
    }
}

fn free(world: &World) -> bool {
    world.player_alive()
        && world.life.acted_on != Some(world.day)
        && bleeding_kansas::sim::family::away(world).is_none()
}

fn sunday(world: &World) -> bool {
    world.day.0 % 7 == 3
}

/// What a spot offers today.
pub fn spot_menu(world: &World, k: SpotKind) -> MenuSpec {
    let f = free(world);
    let task = cmds::season_task(world);
    let month = world.day.month();
    let winter = matches!(month, 11 | 12 | 1 | 2 | 3);
    let hh = &world.families[0].stores;
    let back = item("Leave it", Cmd::Close);
    match k {
        SpotKind::Cabin => {
            let mut items = vec![
                item("Rest till morning", Cmd::Rest),
                when("Study by the lamp", Cmd::Do(Activity::Study), f),
                when("Write home", Cmd::Do(Activity::WriteHome), f),
            ];
            items.extend(cmds::door_choices(world));
            items.push(item("The workbench...", Cmd::Open(Sub::Bench)));
            items.push(item("Hard times...", Cmd::Open(Sub::HardTimes)));
            items.push(item("Take stock", Cmd::Status));
            items.push(back);
            MenuSpec::new(
                "The cabin",
                format!(
                    "{} in the house. Food for {:.0} days.",
                    world.living().filter(|n| n.family == 0).count(),
                    hh.food / world.living().filter(|n| n.family == 0).count().max(1) as f32
                ),
                items,
            )
        }
        SpotKind::Barn => MenuSpec::new(
            "The barn",
            format!(
                "{} cattle, {} oxen. Hay {:.1} tons.{}",
                hh.cattle,
                hh.oxen,
                hh.work.hay,
                if world.families[0].barn_standing {
                    ""
                } else {
                    " What's left of it."
                }
            ),
            vec![
                when(
                    format!("The season's work: {task}"),
                    Cmd::Do(Activity::Chores),
                    f,
                ),
                item("Hard times...", Cmd::Open(Sub::HardTimes)),
                back,
            ],
        ),
        SpotKind::Fields => MenuSpec::new(
            "The fields",
            format!(
                "{} acres under the plow. The season asks for {task}.",
                hh.acres
            ),
            vec![
                when(
                    format!("Work the fields: {task}"),
                    Cmd::Do(Activity::Chores),
                    f,
                ),
                when(
                    "Burn off the pasture",
                    Cmd::Do(Activity::BurnPasture),
                    f && matches!(month, 3 | 4),
                ),
                back,
            ],
        ),
        SpotKind::Fence => MenuSpec::new(
            "The fence",
            format!(
                "The rails are {:.0}% sound. Stock goes through what's down.",
                hh.work.fences * 100.0
            ),
            vec![when("Mend fence", Cmd::Do(Activity::Mend), f), back],
        ),
        SpotKind::Woods => MenuSpec::new(
            "The timber",
            format!(
                "The nearest stand is {:.1} miles off. You have {:.0} loads cut.",
                world.families[0].timber_miles,
                hh.goods[bleeding_kansas::sim::market::Good::Timber.index()]
            ),
            vec![
                when("Cut a load of timber", Cmd::Do(Activity::CutTimber), f),
                when("Hunt", Cmd::Choose(Choice::Hunt), f),
                when("Walk the timber", Cmd::Do(Activity::Roam), f),
                when("Camp out tonight", Cmd::Do(Activity::Camp), f),
                back,
            ],
        ),
        SpotKind::Creek => MenuSpec::new(
            "The creek",
            if winter {
                "Ice at the edges. Muskrat sign in the mud."
            } else {
                "Catfish under the cutbank, if they're biting."
            },
            vec![
                when("Fish", Cmd::Do(Activity::Fish), f),
                when("Run a trapline", Cmd::Do(Activity::Trap), f && winter),
                back,
            ],
        ),
        SpotKind::Yard => cmds::menu(world, Sub::Build),
        SpotKind::Road | SpotKind::TownEdge => {
            let mut m = cmds::menu(world, Sub::Travel);
            m.items
                .insert(0, item("Ride home", Cmd::Go(Screen::Claim, None)));
            m
        }
        SpotKind::Person(id) => cmds::neighbor(world, id),
        SpotKind::Church => {
            let mut items = vec![when(
                if sunday(world) {
                    "Preach to the meeting"
                } else {
                    "Study the Book"
                },
                Cmd::Do(Activity::Preach),
                f,
            )];
            if world.gatherings.split.is_some() && world.gatherings.side_of(PLAYER).is_none() {
                items.push(item(
                    "Sit with the northern meeting",
                    Cmd::Do(Activity::Church { north: true }),
                ));
                items.push(item(
                    "Sit with the southern meeting",
                    Cmd::Do(Activity::Church { north: false }),
                ));
            }
            if let Some(p) = Project::ALL.into_iter().find(|&p| !world.civic.built(p)) {
                items.push(when(
                    format!("Give a day to the {}", p.label()),
                    Cmd::Do(Activity::Build(p)),
                    f,
                ));
            }
            items.push(back);
            MenuSpec::new(
                "Plymouth Church",
                if world.gatherings.split.is_some() {
                    "Congregational. The union meeting is split now; they pass each other on the steps."
                } else {
                    "Congregational. On Sundays it holds both sides, for now."
                },
                items,
            )
        }
        SpotKind::Hotel => {
            if !world.institutions.open[Venue::FreeStateHotel.index()] {
                return MenuSpec::new(
                    "The Free State Hotel",
                    "A shell since May. Cannon, then kerosene. Men still stop and look at it.",
                    vec![back],
                );
            }
            MenuSpec::new(
                "The Free State Hotel",
                "Stone, three stories, built like a fort because it was meant to be one.",
                vec![
                    when(
                        "Speak for patience",
                        Cmd::Do(Activity::Speech { calm: true }),
                        f,
                    ),
                    when(
                        "Speak with fire",
                        Cmd::Do(Activity::Speech { calm: false }),
                        f,
                    ),
                    item("Be seen in the lobby", Cmd::BeSeen),
                    back,
                ],
            )
        }
        SpotKind::PostOffice => MenuSpec::new(
            "The post office",
            "Mail comes Wednesdays, when it comes at all. The Emigrant Aid men leave a catalogue of hymnals on the counter, and wink.",
            vec![
                when("Write home", Cmd::Do(Activity::WriteHome), f),
                when(
                    format!(
                        "Send east for a crate of books (${})",
                        bleeding_kansas::sim::arms::RIFLE_PRICE
                    ),
                    Cmd::OrderRifles,
                    world.families[0].stores.arms.on_order.is_none()
                        && world.families[0].stores.cash >= bleeding_kansas::sim::arms::RIFLE_PRICE,
                ),
                back,
            ],
        ),
        SpotKind::Barbershop => MenuSpec::new(
            "The barbershop",
            "A shave, a bath, and everybody's version of everything.",
            vec![item("A shave and the talk", Cmd::BeSeen), back],
        ),
        SpotKind::LandAgent => MenuSpec::new(
            "The land agent",
            format!(
                "Town lots at ${:.0}. You hold {}.",
                world.land.lot_price, world.land.lots
            ),
            vec![
                when("Buy a lot", Cmd::Do(Activity::BuyLot), f),
                when(
                    "Sell a lot",
                    Cmd::Do(Activity::SellLot),
                    f && world.land.lots > 0,
                ),
                back,
            ],
        ),
        SpotKind::Lyceum => MenuSpec::new(
            "The lyceum",
            "Debates on winter nights. Both sides come to shout at each other politely.",
            vec![
                when(
                    "Take the floor",
                    Cmd::Do(Activity::Speech { calm: true }),
                    f,
                ),
                back,
            ],
        ),
        SpotKind::Store => cmds::menu(world, Sub::Store),
        SpotKind::Saloon => MenuSpec::new(
            "The Jack of Hearts",
            "Franklin's groggery. Whiskey, cards, and men who remember what you said.",
            vec![
                when("Drink", Cmd::Do(Activity::Drink), f),
                item("What you know...", Cmd::Open(Sub::Leverage)),
                item("Be seen at the bar", Cmd::BeSeen),
                back,
            ],
        ),
        SpotKind::LandOffice => MenuSpec::new(
            "The land office",
            if world.civic.filed.first().copied().unwrap_or(false) {
                "Your preemption is on file. The register nods at you, which is worse."
            } else {
                "The register is a pro-slavery man. Free-State papers have a way of getting lost."
            },
            vec![
                when(
                    "File your claim ($2)",
                    Cmd::Do(Activity::FileClaim),
                    f && !world.civic.filed.first().copied().unwrap_or(false),
                ),
                back,
            ],
        ),
        SpotKind::Justice => cmds::menu(world, Sub::Sue),
        SpotKind::Polls => match cmds::election(world) {
            Some(e) => MenuSpec::new(
                "The polls",
                format!("The vote on {}.", e.name),
                vec![
                    item("Cast your ballot", Cmd::Do(Activity::Vote { sell: false })),
                    item(
                        "Sell it to the store's man",
                        Cmd::Do(Activity::Vote { sell: true }),
                    ),
                    back,
                ],
            ),
            None => MenuSpec::new("The polls", "No election today.", vec![back]),
        },
    }
}

pub fn avatar_bundle(image: Handle<Image>, at: Vec2) -> impl Bundle {
    (
        Sprite {
            image,
            custom_size: Some(Vec2::new(30.0, 60.0)),
            ..default()
        },
        Transform::from_translation(at.extend(10.0)),
        Avatar,
        OnFoot,
    )
}
