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
use crate::scenery::Art;
use crate::ui::{LIGHT, Menu};
use crate::{Fonts, Sim, text};

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
                Node {
                    margin: UiRect::top(Val::Px(18.0)),
                    ..default()
                },
                SceneTitle,
            ));
            s.spawn((
                ImageNode::default(),
                Node {
                    width: Val::Px(240.0),
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
                spec: MenuSpec::new(
                    "It's over for you",
                    "The county goes on without you, which it was always going to do. Somebody will tell it wrong in the paper.",
                    vec![item("Let it be told", Cmd::Close)],
                ),
            });
        }
        return None;
    }

    if let Some(id) = world.railroad.at_door {
        let key = format!("knock {id}");
        if unseen(&key, scenes) {
            let s = &world.railroad.seekers[id as usize];
            return Some(Moment {
                key,
                title: "A knock after dark".into(),
                art: art.woman.clone(),
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
    mut scenes: ResMut<Scenes>,
    mut menu: ResMut<Menu>,
    mut root: Query<&mut Node, With<SceneRoot>>,
    mut img: Query<&mut ImageNode, With<SceneArt>>,
    mut title: Query<&mut Text, With<SceneTitle>>,
) {
    let Ok(mut node) = root.single_mut() else {
        return;
    };
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
    if let Ok(mut i) = img.single_mut() {
        i.image = m.art;
        i.color = Color::srgb(0.93, 0.89, 0.80);
    }
    if let Ok(mut t) = title.single_mut() {
        **t = crate::panels::plain(&m.title);
    }
    menu.open_scene(spec);
}
