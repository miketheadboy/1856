//! Commands: every choice in a command window, what it's called, whether it
//! applies right now, and what it does to the sim. The view never decides an
//! outcome; it forwards a verb and reports what the chronicle says happened.

use bleeding_kansas::sim::chronicle;
use bleeding_kansas::sim::civic::Project;
use bleeding_kansas::sim::economy::Choice;
use bleeding_kansas::sim::events::Cruelty;
use bleeding_kansas::sim::family;
use bleeding_kansas::sim::farmwork;
use bleeding_kansas::sim::homestead::{self, Improvement};
use bleeding_kansas::sim::intrigue;
use bleeding_kansas::sim::law;
use bleeding_kansas::sim::life::Activity;
use bleeding_kansas::sim::market::Good;
use bleeding_kansas::sim::psyche::LifeStage;
use bleeding_kansas::sim::railroad::{Answer, Status};
use bleeding_kansas::sim::world::{NpcId, World};

use crate::{Screen, TownId};

/// Which submenu to open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sub {
    AfterDark(NpcId),
    HardTimes,
    Build,
    Store,
    StoreSell,
    Travel,
    Leverage,
    Sue,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cmd {
    Do(Activity),
    Choose(Choice),
    Buy(Good, f32),
    Sell(Good, f32),
    Sign(bool),
    Kill(NpcId),
    Burn(NpcId),
    Steal(NpcId),
    Broker(NpcId),
    BeSeen,
    /// Sleep till morning: the day ends now.
    Rest,
    Open(Sub),
    Go(Screen, Option<TownId>),
    /// Status pages.
    Status,
    Close,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub label: String,
    pub cmd: Cmd,
    pub enabled: bool,
}

pub fn item(label: impl Into<String>, cmd: Cmd) -> Item {
    Item {
        label: label.into(),
        cmd,
        enabled: true,
    }
}

pub fn when(label: impl Into<String>, cmd: Cmd, enabled: bool) -> Item {
    Item {
        label: label.into(),
        cmd,
        enabled,
    }
}

#[derive(Clone, Debug, Default)]
pub struct MenuSpec {
    pub title: String,
    pub body: String,
    pub items: Vec<Item>,
}

impl MenuSpec {
    pub fn new(title: impl Into<String>, body: impl Into<String>, items: Vec<Item>) -> Self {
        MenuSpec {
            title: title.into(),
            body: body.into(),
            items,
        }
    }
}

/// What running a command did.
#[derive(Default)]
pub struct Outcome {
    pub toast: Option<String>,
    pub next: Option<MenuSpec>,
    pub goto: Option<(Screen, Option<TownId>)>,
    pub status: bool,
    /// The clock should roll to morning.
    pub rest: bool,
}

/// Lines the chronicle wrote since `from`, the ones a settler would know.
fn news_since(world: &World, from: usize) -> Option<String> {
    let lines: Vec<String> = world.events[from..]
        .iter()
        .filter(|e| chronicle::is_notable(world, e, false))
        .map(|e| chronicle::debug_line(world, e, false))
        .map(|l| l.split_once("] ").map(|(_, r)| r.to_string()).unwrap_or(l))
        .take(3)
        .collect();
    (!lines.is_empty()).then(|| lines.join("\n"))
}

pub fn run(world: &mut World, cmd: Cmd) -> Outcome {
    let before = world.events.len();
    let mut out = Outcome::default();
    let acted = match cmd {
        Cmd::Do(a) => {
            let a = match a {
                Activity::Build(_) => {
                    match Project::ALL.into_iter().find(|&p| !world.civic.built(p)) {
                        Some(p) => Activity::Build(p),
                        None => return toast("Nothing left for the county to raise."),
                    }
                }
                a => a,
            };
            world.player_do(a)
        }
        Cmd::Choose(c) => world.player_choose(c),
        Cmd::Buy(g, q) => world.player_buy(g, q) > 0.0,
        Cmd::Sell(g, q) => world.player_sell(g, q) > 0,
        Cmd::Sign(yes) => {
            world.player_answer_favor(yes);
            true
        }
        Cmd::Kill(t) => {
            world.player_kill(t);
            true
        }
        Cmd::Burn(t) => {
            world.player_burn(t);
            true
        }
        Cmd::Steal(t) => world.player_steal(t),
        Cmd::Broker(t) => {
            let fam = world.npc(t).family;
            let other = world
                .feuds
                .iter()
                .find_map(|&(a, b)| (a == fam).then_some(b).or((b == fam).then_some(a)));
            match other {
                Some(o) => world.player_broker_peace(fam, o),
                None => return toast("They're at war with nobody you could name."),
            }
        }
        Cmd::BeSeen => {
            world.player_go_to_tavern();
            return toast("You made sure you were seen. Half the county can swear to it.");
        }
        Cmd::Rest => {
            out.rest = true;
            out.toast = Some("You turn in. The house settles.".into());
            return out;
        }
        Cmd::Open(sub) => {
            out.next = Some(menu(world, sub));
            return out;
        }
        Cmd::Go(screen, town) => {
            out.goto = Some((screen, town));
            return out;
        }
        Cmd::Status => {
            out.status = true;
            return out;
        }
        Cmd::Close => return out,
    };
    out.toast = news_since(world, before).or_else(|| {
        Some(if acted {
            "Done.".into()
        } else if world.life.acted_on == Some(world.day) {
            "Your day is spent. Rest, or wait for morning.".into()
        } else {
            "That won't go today.".into()
        })
    });
    out
}

fn toast(s: &str) -> Outcome {
    Outcome {
        toast: Some(s.into()),
        ..Default::default()
    }
}

fn day_free(world: &World) -> bool {
    world.player_alive() && world.life.acted_on != Some(world.day) && family::away(world).is_none()
}

/// A neighbor, face to face.
pub fn neighbor(world: &World, t: NpcId) -> MenuSpec {
    let n = world.npc(t);
    let grown = LifeStage::of(n.age) != LifeStage::Child;
    let free = day_free(world);
    let mut items = vec![
        when("Sit a spell", Cmd::Do(Activity::Visit(t)), free),
        when("Court", Cmd::Do(Activity::Court(t)), free && grown),
        when(
            "Baptize in the Wakarusa",
            Cmd::Do(Activity::Baptize(t)),
            free && !world.life.baptized.contains(&t),
        ),
    ];
    let fam = n.family;
    if world.feuds.iter().any(|&(a, b)| a == fam || b == fam) {
        items.push(item("Broker a peace", Cmd::Broker(t)));
    }
    if world
        .law
        .disputes
        .iter()
        .any(|d| d.plaintiff == 0 && d.defendant == t)
    {
        items.push(when("Take them to law", Cmd::Do(Activity::Sue(t)), free));
    }
    if bleeding_kansas::sim::land::would_sell(world, fam) {
        items.push(when(
            format!(
                "Offer for their claim (${})",
                bleeding_kansas::sim::land::claim_price(world, fam)
            ),
            Cmd::Do(Activity::BuyClaim(fam)),
            free,
        ));
    }
    items.push(item("After dark...", Cmd::Open(Sub::AfterDark(t))));
    items.push(item("Leave them be", Cmd::Close));
    MenuSpec::new(n.name.clone(), crate::panels::inspect(world, t), items)
}

fn after_dark(world: &World, t: NpcId) -> MenuSpec {
    let free = day_free(world);
    let leverage = intrigue::held(world, t).is_some();
    let items = vec![
        when(
            "Put their barn to the torch",
            Cmd::Burn(t),
            world.player_alive(),
        ),
        when("Drive off a cow", Cmd::Steal(t), world.player_alive()),
        when(
            "Shoot a cow and leave it",
            Cmd::Do(Activity::Sabotage(t, Cruelty::KillStock)),
            free,
        ),
        when(
            "A carcass down the well",
            Cmd::Do(Activity::Sabotage(t, Cruelty::FoulWell)),
            free,
        ),
        when(
            "Pull the fence rails",
            Cmd::Do(Activity::Sabotage(t, Cruelty::CutFence)),
            free,
        ),
        when(
            "Wet the haystack",
            Cmd::Do(Activity::Sabotage(t, Cruelty::SpoilHay)),
            free,
        ),
        when("Whisper against them", Cmd::Do(Activity::Slander(t)), free),
        when(
            "Ask for money to keep quiet",
            Cmd::Do(Activity::Blackmail(t)),
            free && leverage,
        ),
        when(
            "Tell the county",
            Cmd::Do(Activity::Expose(t)),
            free && leverage,
        ),
        when(
            "Lie in wait with a rifle",
            Cmd::Kill(t),
            world.player_alive(),
        ),
        item("Think better of it", Cmd::Close),
    ];
    MenuSpec::new(
        "After dark",
        "The moon's what it is tonight. Whatever you do, somebody may be awake.",
        items,
    )
}

pub fn menu(world: &World, sub: Sub) -> MenuSpec {
    let free = day_free(world);
    match sub {
        Sub::AfterDark(t) => after_dark(world, t),
        Sub::HardTimes => MenuSpec::new(
            "Hard times",
            crate::panels::household(world),
            vec![
                item("Butcher a cow", Cmd::Choose(Choice::SpareCow)),
                item("Butcher the last cow", Cmd::Choose(Choice::LastCow)),
                item("Butcher an ox", Cmd::Choose(Choice::Ox)),
                item("Eat the seed corn", Cmd::Choose(Choice::EatSeed)),
                item("Beg at a neighbor's door", Cmd::Choose(Choice::Beg)),
                item("Go hungry today", Cmd::Close),
            ],
        ),
        Sub::Build => {
            let timber = world.families[0].stores.goods[Good::Timber.index()];
            let items = Improvement::ALL
                .into_iter()
                .map(|imp| {
                    let (days, wood) = imp.cost();
                    let built = homestead::has(world, 0, imp);
                    let label = if built {
                        format!("{} (standing)", imp.label())
                    } else {
                        format!("{}: {days:.0} days, {wood:.0} loads", imp.label())
                    };
                    when(label, Cmd::Do(Activity::Improve(imp)), free && !built)
                })
                .chain([item("Not today", Cmd::Close)])
                .collect();
            MenuSpec::new(
                "Build",
                format!("Timber on hand: {timber:.0} loads. Cut more in the woods."),
                items,
            )
        }
        Sub::Store => {
            let m = &world.market;
            let cash = world.families[0].stores.cash;
            let line = |g: Good| format!("{} ${:.2}", g.label(), m.price(g));
            MenuSpec::new(
                "Dunmore's",
                format!(
                    "Dunmore sets his prices by what's on the shelf and who's asking. You have ${cash}."
                ),
                vec![
                    item(
                        format!("Buy corn: {}", line(Good::Corn)),
                        Cmd::Buy(Good::Corn, 10.0),
                    ),
                    item(
                        format!("Buy seed: {}", line(Good::SeedCorn)),
                        Cmd::Buy(Good::SeedCorn, 5.0),
                    ),
                    item(
                        format!("Buy salt: {}", line(Good::Salt)),
                        Cmd::Buy(Good::Salt, 2.0),
                    ),
                    item(
                        format!("Buy powder and lead: {}", line(Good::Powder)),
                        Cmd::Buy(Good::Powder, 3.0),
                    ),
                    item(
                        format!("Buy timber: {}", line(Good::Timber)),
                        Cmd::Buy(Good::Timber, 1.0),
                    ),
                    item(
                        format!("Buy whiskey: {}", line(Good::Whiskey)),
                        Cmd::Buy(Good::Whiskey, 1.0),
                    ),
                    item("Sell...", Cmd::Open(Sub::StoreSell)),
                    item("Put it on the book", Cmd::Choose(Choice::Borrow)),
                    item("Walk out", Cmd::Close),
                ],
            )
        }
        Sub::StoreSell => MenuSpec::new(
            "Sell to Dunmore",
            "He buys low. He always buys low.",
            vec![
                item("Sell corn", Cmd::Sell(Good::Corn, 10.0)),
                item("Sell hides and pelts", Cmd::Sell(Good::Hides, 5.0)),
                item("Sell a cow", Cmd::Sell(Good::Cattle, 1.0)),
                item("Sell timber", Cmd::Sell(Good::Timber, 1.0)),
                item("Back", Cmd::Open(Sub::Store)),
            ],
        ),
        Sub::Travel => MenuSpec::new(
            "The road",
            "Lawrence to the northeast, Franklin east on the Wakarusa, Lecompton up the Kaw.",
            vec![
                item(
                    "Ride to Lawrence",
                    Cmd::Go(Screen::Town, Some(TownId::Lawrence)),
                ),
                item(
                    "Ride to Franklin",
                    Cmd::Go(Screen::Town, Some(TownId::Franklin)),
                ),
                item(
                    "Ride to Lecompton",
                    Cmd::Go(Screen::Town, Some(TownId::Lecompton)),
                ),
                item("Look over the county", Cmd::Go(Screen::County, None)),
                when(
                    "West to the buffalo range (weeks)",
                    Cmd::Choose(Choice::GoWest),
                    free,
                ),
                item("Stay home", Cmd::Close),
            ],
        ),
        Sub::Leverage => {
            let mut items: Vec<Item> = intrigue::leverage(world)
                .into_iter()
                .map(|t| {
                    when(
                        format!("Lean on {}", world.name(t)),
                        Cmd::Do(Activity::Blackmail(t)),
                        free,
                    )
                })
                .collect();
            if items.is_empty() {
                return MenuSpec::new(
                    "Nothing to lean on",
                    "You know nothing about anybody. Drink more, and listen.",
                    vec![item("Fair enough", Cmd::Close)],
                );
            }
            items.push(item("Keep it to yourself", Cmd::Close));
            MenuSpec::new("What you know", "Silence has a price. So does talk.", items)
        }
        Sub::Sue => {
            let mut items: Vec<Item> = world
                .law
                .disputes
                .iter()
                .filter(|d| d.plaintiff == 0)
                .map(|d| {
                    when(
                        format!("Sue {} over {} acres", world.name(d.defendant), d.acres),
                        Cmd::Do(Activity::Sue(d.defendant)),
                        free,
                    )
                })
                .collect();
            if items.is_empty() {
                return MenuSpec::new(
                    "No cause",
                    "The justice looks at you over his spectacles. You have no case.",
                    vec![item("Leave", Cmd::Close)],
                );
            }
            items.push(item("Leave", Cmd::Close));
            MenuSpec::new(
                "Before the justice",
                "Justice of the peace, appointed by the bogus legislature. Fees are three dollars.",
                items,
            )
        }
    }
}

/// The door: what someone at your house can do right now.
pub fn door_choices(world: &World) -> Vec<Item> {
    let mut items = Vec::new();
    if world.railroad.at_door.is_some() {
        items.push(item(
            "Hide them",
            Cmd::Do(Activity::Answer(Answer::Shelter)),
        ));
        items.push(item(
            "Turn them away",
            Cmd::Do(Activity::Answer(Answer::TurnAway)),
        ));
        items.push(item(
            "Turn them in",
            Cmd::Do(Activity::Answer(Answer::Betray)),
        ));
    }
    if world
        .railroad
        .seekers
        .iter()
        .any(|s| s.status == Status::Hidden(0))
    {
        items.push(when(
            "Guide them north",
            Cmd::Do(Activity::Guide),
            day_free(world),
        ));
    }
    items
}

pub fn season_task(world: &World) -> String {
    farmwork::due(world.day).label().to_string()
}

pub fn election(world: &World) -> Option<&'static law::ElectionDay> {
    law::election_today(world)
}
