//! Scenes: the moments the county forces on you. A knock after dark, the
//! storekeeper's petition, the muster call, election day, a bee tonight, a
//! letter, a death in the house. The screen goes dark at the edges, an
//! engraving comes up, the clock stops, and you choose.

use bevy::prelude::*;
use bleeding_kansas::sim::EventKind;
use bleeding_kansas::sim::life::Activity;
use bleeding_kansas::sim::mail::Letter;
use bleeding_kansas::sim::world::{PLAYER, World};
use bleeding_kansas::sim::{family, law};

use crate::cmds::{self, Cmd, MenuSpec, item};
use crate::duel::Approach;
use crate::scenery::Art;
use crate::ui::{LIGHT, Menu};
use crate::{Fonts, Sim, text};
use bleeding_kansas::sim::action;
use bleeding_kansas::sim::events::Retaliation;

#[derive(Resource, Default)]
pub struct Scenes {
    /// What's already been shown, so nothing asks twice.
    seen: Vec<String>,
    /// The last event looked at.
    read_to: usize,
    pub showing: bool,
}

impl Scenes {
    /// Start watching from here: what happened before you took the reins
    /// isn't news.
    pub fn starting_at(events: usize) -> Self {
        Scenes {
            read_to: events,
            ..Default::default()
        }
    }
}

/// Every moment that can have a painted plate, by the file name it looks for.
pub const PLATES: [&str; 16] = [
    "end",
    "bounty",
    "law",
    "door",
    "riders_torch",
    "riders_rifle",
    "fallen",
    "paper",
    "knock",
    "petition",
    "muster",
    "election",
    "bee",
    "letter",
    "grave",
    "wagon",
];

/// Painted plates found under `assets/plates/`: `<name>.png`, or a boil of
/// `<name>_1.png`, `<name>_2.png`, ... cycled like hand-drawn line.
#[derive(Resource)]
pub struct Plates {
    frames: Vec<(&'static str, Vec<Handle<Image>>)>,
    /// Showing now, and since when (seconds).
    showing: Option<(&'static str, f32)>,
}

impl FromWorld for Plates {
    fn from_world(world: &mut bevy::ecs::world::World) -> Self {
        let assets = world.resource::<AssetServer>();
        let mut frames = Vec::new();
        for name in PLATES {
            let mut f: Vec<Handle<Image>> = (1..=8)
                .map(|i| format!("plates/{name}_{i}.png"))
                .take_while(|p| crate::asset_exists(p))
                .map(|p| assets.load(p))
                .collect();
            if f.is_empty() && crate::asset_exists(&format!("plates/{name}.png")) {
                f.push(assets.load(format!("plates/{name}.png")));
            }
            if !f.is_empty() {
                frames.push((name, f));
            }
        }
        Plates {
            frames,
            showing: None,
        }
    }
}

impl Plates {
    fn get(&self, name: &str) -> Option<&Vec<Handle<Image>>> {
        self.frames.iter().find(|p| p.0 == name).map(|p| &p.1)
    }
}

/// The light a plate is seen by: moonlight for what comes after dark,
/// lamplight for the house, cold for the grave, paper for the rest.
pub fn plate_tint(name: &str) -> Color {
    match name {
        "knock" | "riders_torch" | "riders_rifle" | "law" | "door" | "fallen" => {
            Color::srgb(0.74, 0.80, 0.92)
        }
        "letter" | "bee" | "petition" => Color::srgb(1.0, 0.88, 0.70),
        "grave" | "end" => Color::srgb(0.80, 0.80, 0.78),
        _ => Color::srgb(0.95, 0.90, 0.80),
    }
}

/// Seconds for the slow push to reach its end; frames a second for a boil.
const PUSH_SECONDS: f32 = 24.0;
const PUSH_SCALE: f32 = 0.06;
const BOIL_FPS: f32 = 6.0;

#[derive(Component)]
pub struct ScenePlate;

#[derive(Component)]
pub struct SceneRoot;
#[derive(Component)]
pub struct SceneArt;
#[derive(Component)]
pub struct SceneTitle;

pub fn setup(mut commands: Commands, fonts: Res<Fonts>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
            GlobalZIndex(15),
            SceneRoot,
        ))
        .with_children(|s| {
            // The painted plate, under everything else in the frame.
            s.spawn((
                ImageNode {
                    color: Color::NONE,
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    ..default()
                },
                UiTransform::default(),
                ScenePlate,
            ));
            // Letterbox.
            s.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(64.0),
                    ..default()
                },
                BackgroundColor(Color::BLACK),
            ));
            s.spawn((
                Text::new(""),
                text(&fonts.display, 34.0, LIGHT),
                // Over a pale plate the title needs ink under it.
                TextShadow {
                    offset: Vec2::new(2.0, 2.0),
                    color: Color::srgba(0.05, 0.04, 0.03, 0.9),
                },
                Node {
                    margin: UiRect::top(Val::Px(18.0)),
                    ..default()
                },
                SceneTitle,
            ));
            s.spawn((
                ImageNode::default(),
                Node {
                    height: Val::Px(200.0),
                    margin: UiRect::top(Val::Px(14.0)),
                    ..default()
                },
                SceneArt,
            ));
            s.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Px(64.0),
                    ..default()
                },
                BackgroundColor(Color::BLACK),
            ));
        });
}

struct Moment {
    key: String,
    title: String,
    art: Handle<Image>,
    /// The painted plate for this moment, if one's been drawn
    /// (`assets/plates/<plate>.png`, docs/ART.md).
    plate: &'static str,
    spec: MenuSpec,
}

fn next_moment(world: &World, scenes: &mut Scenes, art: &Art) -> Option<Moment> {
    let day = world.day.0;
    let unseen = |k: &str, s: &Scenes| !s.seen.iter().any(|x| x == k);

    if !world.player_alive() {
        let key = "the end".to_string();
        if unseen(&key, scenes) {
            return Some(Moment {
                key,
                title: "Knocking on heaven's door".into(),
                art: art.cross.clone(),
                plate: "end",
                spec: MenuSpec::new(
                    "It's over for you",
                    "The county goes on without you, which it was always going to do. Somebody will tell it wrong in the paper.",
                    vec![item("Let it be told", Cmd::Close)],
                ),
            });
        }
        return None;
    }

    if let Some(s) = &world.action.standoff {
        let key = format!("standoff {} {}", s.actor, s.day.0);
        if unseen(&key, scenes) {
            let name = world.name(s.actor).to_string();
            let company = match s.riders.len() {
                0 => "alone".to_string(),
                _ => format!(
                    "with {}",
                    s.riders
                        .iter()
                        .map(|&r| world.name(r).to_string())
                        .collect::<Vec<_>>()
                        .join(" and ")
                ),
            };
            let quick = action::their_draw(world, s.actor);
            let hand = if quick < 0.36 {
                "His hand is quick. Very quick."
            } else if quick < 0.5 {
                "He knows which end of a gun is which."
            } else {
                "He's no gunman, and knows it."
            };
            let nerve = if action::nerve(world) > 0.2 {
                "Your hands are steady."
            } else {
                "Your mouth is dry."
            };
            let paper = s.serving.and_then(|i| world.warrants.list.get(i));
            let (title, body, stand) = if let Some(p) = paper {
                if s.yours {
                    (
                        "With a paper".to_string(),
                        format!(
                            "{name} comes out onto the step and sees the justice's paper in your hand. ${} on delivery, half if you bring him in over a saddle. {hand} {nerve}",
                            p.bounty
                        ),
                        "Let him be",
                    )
                } else {
                    (
                        "The law at the gate".to_string(),
                        format!(
                            "{name}, {company}, with a paper from the justice. It says you did a thing; it doesn't say whether you did. ${} on your head. {hand} {nerve}",
                            bleeding_kansas::sim::warrant::price_on(world, PLAYER)
                        ),
                        "Put your hands up",
                    )
                }
            } else if s.yours {
                (
                    format!(
                        "At the {} door",
                        world.families[world.npc(s.actor).family as usize].surname
                    ),
                    format!(
                        "{name} comes out onto the step and doesn't ask you in. {hand} {nerve}"
                    ),
                    "Lose your nerve and ride home",
                )
            } else {
                (
                    "Riders at the gate".to_string(),
                    format!(
                        "{name}, {company}, {}. {hand} {nerve}",
                        match s.method {
                            Retaliation::Arson => "and a torch burning in the damp",
                            Retaliation::Ambush => "a rifle across the saddle",
                        }
                    ),
                    "Stand aside",
                )
            };
            let plate = match (paper.is_some(), s.yours, s.method) {
                (true, true, _) => "bounty",
                (true, false, _) => "law",
                (false, true, _) => "door",
                (false, false, Retaliation::Arson) => "riders_torch",
                (false, false, Retaliation::Ambush) => "riders_rifle",
            };
            let woman = bleeding_kansas::sim::world::is_woman(&name);
            let body = crate::panels::gendered(&body, woman);
            return Some(Moment {
                key,
                title,
                art: art.gunman.clone(),
                plate,
                spec: MenuSpec::new(
                    name,
                    body,
                    vec![
                        item(
                            crate::panels::gendered("Talk him down", woman),
                            Cmd::Standoff(Approach::Talk),
                        ),
                        item(
                            crate::panels::gendered("Face him down", woman),
                            Cmd::Standoff(Approach::Face),
                        ),
                        crate::cmds::when(
                            if action::armed(world) {
                                "Go for your gun"
                            } else {
                                "Go for your gun (nothing loaded to hand)"
                            },
                            Cmd::Standoff(Approach::Draw),
                            action::armed(world),
                        ),
                        item(stand, Cmd::StandAside),
                    ],
                ),
            });
        }
    }

    // Somebody you just shot, lying there.
    if let Some((v, act, _, day)) = &world.wardrobe.lootable
        && *day == world.day
        && world.action.standoff.is_none()
    {
        let key = format!("fallen {act}");
        if unseen(&key, scenes) {
            let name = world.name(*v).to_string();
            let woman = bleeding_kansas::sim::world::is_woman(&name);
            let dead = !world.npc(*v).alive;
            let body = crate::panels::gendered(
                &format!(
                    "{name} is {} in the dirt. {} coat, {} boots, whatever's in his pockets. Anyone who sees you do it will remember it, and so will the clothes.",
                    if dead { "dead" } else { "down and bleeding" },
                    "His",
                    "his"
                ),
                woman,
            );
            return Some(Moment {
                key,
                title: "Down".into(),
                art: art.gunman.clone(),
                plate: "fallen",
                spec: MenuSpec::new(
                    name,
                    body,
                    vec![
                        item("Go through the pockets", Cmd::Loot),
                        item("Leave it lie", Cmd::Close),
                    ],
                ),
            });
        }
    }

    // A paper with your name on it, the day word reaches you.
    if let Some(&i) = bleeding_kansas::sim::warrant::wanted(world, PLAYER).last() {
        let key = format!("paper {i}");
        if unseen(&key, scenes) && family::away(world).is_none() {
            let w = &world.warrants.list[i];
            let what = bleeding_kansas::sim::chronicle::charge_line(world, w.about);
            let mut items = cmds::wanted_choices(world);
            items.insert(
                0,
                cmds::when(
                    "Ride to Lecompton and give yourself up",
                    Cmd::Surrender,
                    world.life.acted_on != Some(world.day),
                ),
            );
            items.push(item("Wait for them", Cmd::Close));
            return Some(Moment {
                key,
                title: "A paper with your name on it".into(),
                art: art.office.clone(),
                plate: "paper",
                spec: MenuSpec::new(
                    format!("Wanted: ${}", w.bounty),
                    format!(
                        "Word comes up the road before the riders do. The justice has written a paper on you for {what}, on the oaths of people who believe it. Whether you did it is not a question the paper asks."
                    ),
                    items,
                ),
            });
        }
    }

    if let Some(id) = world.railroad.at_door {
        let key = format!("knock {id}");
        if unseen(&key, scenes) {
            let s = &world.railroad.seekers[id as usize];
            return Some(Moment {
                key,
                title: "A knock after dark".into(),
                art: art.woman.clone(),
                plate: "knock",
                spec: MenuSpec::new(
                    format!("{}, from {}", s.name, s.from),
                    "Wet to the knees from the creek, and not asking for much: a loft, a day, a direction. The law of this Territory says you owe the county a name. The law says a great many things.",
                    cmds::door_choices(world),
                ),
            });
        }
    }

    if let Some(c) = world.pending_favor {
        let key = format!("petition {day}");
        if unseen(&key, scenes) {
            return Some(Moment {
                key,
                title: "Dunmore's petition".into(),
                art: art.store.clone(),
                plate: "petition",
                spec: MenuSpec::new(
                    format!("{} holds your note", world.name(c)),
                    format!(
                        "You owe the store ${}. He slides a Law and Order petition across the counter with the air of a man offering you a biscuit. Sign it and he'll forgive half. Don't, and he'll take what he can carry.",
                        world.families[0].stores.debt
                    ),
                    vec![
                        item("Sign it", Cmd::Sign(true)),
                        item("Tell him where to put it", Cmd::Sign(false)),
                    ],
                ),
            });
        }
    }

    if let Some((i, _, _)) = &world.law.muster
        && !world.law.player_answered
        && law::eligible(world, PLAYER)
    {
        let key = format!("muster {i}");
        if unseen(&key, scenes) {
            let m = &law::MUSTERS[*i];
            return Some(Moment {
                key,
                title: "The muster is called".into(),
                art: art.rifles.clone(),
                plate: "muster",
                spec: MenuSpec::new(
                    m.name,
                    "Men at the gate with rifles and a list. Your name is on the list. So is everybody's; that's what a list is for.",
                    vec![
                        item(
                            "Ride with them (ten days)",
                            Cmd::Do(Activity::Muster { join: true }),
                        ),
                        item(
                            "Stay home, and say so",
                            Cmd::Do(Activity::Muster { join: false }),
                        ),
                    ],
                ),
            });
        }
    }

    if let Some(e) = law::election_today(world)
        && world.life.ballot.is_none_or(|(d, _)| d != world.day)
    {
        let key = format!("election {day}");
        if unseen(&key, scenes) {
            let sell = e.poll == law::Poll::Territorial;
            let mut items = vec![item(
                "Cast your ballot",
                Cmd::Do(Activity::Vote { sell: false }),
            )];
            if sell {
                items.push(item(
                    "Sell it to the store's man ($3)",
                    Cmd::Do(Activity::Vote { sell: true }),
                ));
            }
            items.push(item("Stay home", Cmd::Close));
            return Some(Moment {
                key,
                title: "Election day".into(),
                art: art.ballot.clone(),
                plate: "election",
                spec: MenuSpec::new(
                    format!("The vote on {}", e.name),
                    if sell {
                        "Missourians at the polls, most of them voting early and some of them voting often."
                    } else {
                        "A Free-State poll in somebody's parlor. The other side calls it treason and says so loudly."
                    },
                    items,
                ),
            });
        }
    }

    if let Some((bee, host, _)) = &world.gatherings.today
        && family::away(world).is_none()
    {
        let key = format!("bee {day}");
        if unseen(&key, scenes) {
            return Some(Moment {
                key,
                title: format!("A {} tonight", bee.label()),
                art: art.barn.clone(),
                plate: "bee",
                spec: MenuSpec::new(
                    format!("At the {} place", world.families[*host as usize].surname),
                    "Lanterns, a fiddle, people from both sides of the line being civil on account of the food.",
                    vec![
                        item("Go", Cmd::Do(Activity::Gather)),
                        item("Stay home", Cmd::Close),
                    ],
                ),
            });
        }
    }

    // Letters and deaths come through the event log.
    let from = scenes.read_to;
    scenes.read_to = world.events.len();
    for e in &world.events[from.min(world.events.len())..] {
        match e.kind {
            EventKind::Letter {
                family: 0, letter, ..
            } => {
                let body = match letter {
                    Letter::Money(d) => format!("Folded around ${d} in banknotes of doubtful Ohio banks. Your mother's hand, and her opinions."),
                    Letter::BadNews => "Black-bordered. Someone back home is dead, and has been for a month, the mails being what they are.".into(),
                    Letter::KinComing => "Kin are coming out in the spring. They have heard Kansas is a garden. You will not correct them by post.".into(),
                    Letter::Homesick => "They ask when you are coming home. The question has no answer that fits on a page.".into(),
                };
                return Some(Moment {
                    key: format!("letter {}", e.id),
                    title: "A letter".into(),
                    art: art.letter.clone(),
                    plate: "letter",
                    spec: MenuSpec::new(
                        "From the States",
                        body,
                        vec![item("Fold it away", Cmd::Close)],
                    ),
                });
            }
            EventKind::Death { victim, .. } | EventKind::Perished { victim, .. }
                if world.npc(victim).family == 0 && victim != PLAYER =>
            {
                return Some(Moment {
                    key: format!("death {}", e.id),
                    title: "Knocking on heaven's door".into(),
                    art: art.cross.clone(),
                    plate: "grave",
                    spec: MenuSpec::new(
                        world.name(victim).to_string(),
                        "You dig in ground that doesn't want digging, and say what words you have. The prairie doesn't stop for it.",
                        vec![item("Bury them", Cmd::Close)],
                    ),
                });
            }
            _ => {}
        }
    }
    None
}

/// Look for a moment; show it; hide the frame when it's answered.
pub fn run(
    sim: Res<Sim>,
    art: Res<Art>,
    time: Res<Time>,
    mut plates: ResMut<Plates>,
    mut scenes: ResMut<Scenes>,
    mut menu: ResMut<Menu>,
    mut root: Query<&mut Node, With<SceneRoot>>,
    mut img: Query<&mut ImageNode, (With<SceneArt>, Without<ScenePlate>)>,
    mut title: Query<&mut Text, With<SceneTitle>>,
    game: Res<crate::duel::Game>,
    state: Res<State<crate::Screen>>,
) {
    let Ok(mut node) = root.single_mut() else {
        return;
    };
    if game.active() || *state.get() == crate::Screen::Raid {
        if scenes.showing && !menu.is_open() {
            scenes.showing = false;
            node.display = Display::None;
        }
        return;
    }
    if scenes.showing {
        if !menu.is_open() {
            scenes.showing = false;
            node.display = Display::None;
        }
        return;
    }
    if menu.is_open() {
        return;
    }
    let Some(m) = next_moment(&sim.0, &mut scenes, &art) else {
        return;
    };
    scenes.seen.push(m.key);
    scenes.showing = true;
    let mut spec = m.spec;
    if spec.items.is_empty() {
        spec.items.push(item("Go on", Cmd::Close));
    }
    node.display = Display::Flex;
    let painted = plates.get(m.plate).is_some();
    plates.showing = painted.then_some((m.plate, time.elapsed_secs()));
    if let Ok(mut i) = img.single_mut() {
        i.image = m.art;
        // A painted plate takes the frame; the little engraving steps aside.
        i.color = if painted {
            Color::NONE
        } else {
            Color::srgb(0.93, 0.89, 0.80)
        };
    }
    if let Ok(mut t) = title.single_mut() {
        **t = crate::panels::plain(&m.title);
    }
    menu.open_scene(spec);
}

/// The plate breathes: a slow push in, a drift, and the line boiling if the
/// plate has frames. Hidden whenever the scene is.
pub fn plate(
    time: Res<Time>,
    scenes: Res<Scenes>,
    plates: Res<Plates>,
    mut q: Query<(&mut ImageNode, &mut UiTransform), With<ScenePlate>>,
) {
    let Ok((mut img, mut tf)) = q.single_mut() else {
        return;
    };
    let Some((name, since)) = plates.showing.filter(|_| scenes.showing) else {
        img.color = Color::NONE;
        return;
    };
    let Some(frames) = plates.get(name) else {
        return;
    };
    let t = time.elapsed_secs() - since;
    let i = (t * BOIL_FPS) as usize % frames.len();
    img.image = frames[i].clone();
    img.color = plate_tint(name);
    let push = (t / PUSH_SECONDS).min(1.0);
    let ease = push * push * (3.0 - 2.0 * push);
    tf.scale = Vec2::splat(1.0 + PUSH_SCALE * ease);
    tf.translation = Val2::px(-6.0 * ease, -3.0 * ease);
}
