//! Commands: every choice in a command window, what it's called, whether it
//! applies right now, and what it does to the sim. The view never decides an
//! outcome; it forwards a verb and reports what the chronicle says happened.

use bleeding_kansas::sim::action;
use bleeding_kansas::sim::arms::{self, Craft, Hide};
use bleeding_kansas::sim::chronicle;
use bleeding_kansas::sim::civic::Project;
use bleeding_kansas::sim::economy::Choice;
use bleeding_kansas::sim::family;
use bleeding_kansas::sim::farmwork;
use bleeding_kansas::sim::hands::{self, COMPANIES, Task};
use bleeding_kansas::sim::homestead::{self, Improvement};
use bleeding_kansas::sim::intrigue;
use bleeding_kansas::sim::law;
use bleeding_kansas::sim::life::Activity;
use bleeding_kansas::sim::market::Good;
use bleeding_kansas::sim::psyche::LifeStage;
use bleeding_kansas::sim::railroad::{Answer, Status};
use bleeding_kansas::sim::sickness::{self, Remedy};
use bleeding_kansas::sim::wardrobe::{self, ITEMS};
use bleeding_kansas::sim::warrant;
use bleeding_kansas::sim::world::{NpcId, World};

use crate::duel::Approach;
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
    /// The workbench: cast, roll, split, and where the guns go.
    Bench,
    /// Who's doing what on the place.
    Hands,
    /// Set one person to a job.
    Assign(NpcId),
    /// Who'd take your wages.
    Hire,
    /// Companies that ride for pay. `true`: the Free-State ones.
    Guns(bool),
    /// Whose place a company would ride on.
    GunsOn(usize),
    /// The justice's desk: papers, suits, and giving yourself up.
    Justice,
    /// Papers anyone could take.
    Papers,
    /// What you're wearing and what's in the trunk.
    Dress,
    /// Dunmore's dry goods.
    Clothes,
    /// Dunmore's physic shelf.
    Medicines,
    /// Who's sick in the house, and what can be done.
    Sick,
    /// One sick person: what to give them.
    Tend(NpcId),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cmd {
    Do(Activity),
    Choose(Choice),
    Buy(Good, f32),
    Sell(Good, f32),
    Sign(bool),
    Broker(NpcId),
    /// Meet the man at the gate (or at his door) this way.
    Standoff(Approach),
    /// Stand aside and let him do what he came to do.
    StandAside,
    /// Ride to their door and have it out.
    Confront(NpcId),
    /// Slip onto their place tonight.
    Raid(NpcId),
    /// Lie in wait on their road tonight.
    Ambush(NpcId),
    /// An evening at the bench.
    Craft(Craft),
    /// Put the guns somewhere.
    Hide(Hide),
    /// Send east for a crate of books.
    OrderRifles,
    BuyGun,
    BuyLead,
    /// Set someone to a job.
    Assign(NpcId, Task),
    Hire(NpcId),
    LetGo(NpcId),
    /// A company to sit on your place, three nights.
    HireGuard(usize),
    /// A company to ride on someone tonight.
    HireRaid(usize, NpcId),
    /// Take a paper for the bounty.
    TakePaper(usize),
    /// Go after the man on your paper.
    RideAfter(usize),
    /// Ride with the sheriff's men for a share.
    RideWith(usize),
    Surrender,
    Run,
    LieLow,
    /// Put on the piece at this place in the trunk.
    Wear(usize),
    /// Take off what's in this slot.
    TakeOff(usize),
    BuyItem(u8),
    BuyMed(Remedy),
    Dose(NpcId, Remedy),
    /// Sit up with them all day.
    Nurse(NpcId),
    /// Send for the doctor.
    Doctor,
    /// Bark and herbs off the creek bottoms.
    GatherHerbs,
    Delouse,
    Quarantine(bool),
    /// A blanket out of a lousy house.
    FoulBlanket(NpcId),
    /// Go through the fallen man's pockets.
    Loot,
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

/// A game to play by hand.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Play {
    Standoff(Approach),
    Raid(NpcId),
    Ambush(NpcId),
    Craft(Craft),
}

/// What running a command did.
#[derive(Default)]
pub struct Outcome {
    pub play: Option<Play>,
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
        Cmd::Standoff(a) => {
            out.play = Some(Play::Standoff(a));
            return out;
        }
        Cmd::StandAside => {
            action::back_down(world);
            true
        }
        Cmd::Confront(t) => {
            if !action::confront(world, t) {
                return toast("Not today.");
            }
            return out;
        }
        Cmd::Raid(t) | Cmd::Ambush(t) => {
            if !action::night_free(world) {
                return toast("You've been out once tonight. Once is plenty.");
            }
            out.play = Some(match cmd {
                Cmd::Raid(_) => Play::Raid(t),
                _ => Play::Ambush(t),
            });
            return out;
        }
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
        Cmd::Craft(c) => {
            if !arms::can_craft(world, c) {
                return toast("Nothing to work with, or you've had your evening at the bench.");
            }
            out.play = Some(Play::Craft(c));
            return out;
        }
        Cmd::Hide(h) => {
            arms::hide(world, 0, h);
            return toast(&format!("The guns are {} now.", h.label()));
        }
        Cmd::OrderRifles => {
            return toast(if arms::order(world, 0, 1) {
                "The letter goes east with twenty dollars in it. Books, it says. Two weeks, three, if the river's open."
            } else {
                "Twenty dollars you haven't got, or a crate already coming."
            });
        }
        Cmd::BuyGun => {
            return toast(if arms::buy_gun(world, 0) {
                "An old fowling piece. Dunmore charged you the Yankee price and smiled doing it."
            } else {
                "Not with what's in your purse."
            });
        }
        Cmd::BuyLead => {
            return toast(if arms::buy_lead(world, 0) {
                "Five pounds of bar lead, heavy as a conscience."
            } else {
                "Not with what's in your purse."
            });
        }
        Cmd::Assign(id, t) => {
            hands::assign(world, id, t);
            out.next = Some(menu(world, Sub::Hands));
            out.toast = Some(format!(
                "{} will be {} from tomorrow.",
                world.name(id),
                t.label()
            ));
            return out;
        }
        Cmd::Hire(id) => {
            if !hands::hire(world, id) {
                return toast("They've thought better of it.");
            }
            out.next = Some(menu(world, Sub::Hands));
            out.toast = news_since(world, before);
            return out;
        }
        Cmd::LetGo(id) => {
            hands::let_go(world, id);
            out.next = Some(menu(world, Sub::Hands));
            out.toast = news_since(world, before);
            return out;
        }
        Cmd::HireGuard(c) => {
            return toast(if hands::hire_guard(world, c, 3) {
                "They'll be in the barn by dark: three nights, rifles, and your whiskey."
            } else {
                "Not with what's in your purse, or they're already here."
            });
        }
        Cmd::HireRaid(c, t) => {
            if !hands::hire_raid(world, c, t) {
                return toast("Not tonight.");
            }
            out.toast = news_since(world, before)
                .or_else(|| Some("They rode out after dark. You didn't go.".into()));
            return out;
        }
        Cmd::TakePaper(i) => {
            if !warrant::take(world, i) {
                return toast("The justice won't give you that one.");
            }
            out.next = Some(menu(world, Sub::Papers));
            out.toast = Some(
                "The justice signs it over. Dead or alive pays, he says; alive pays better.".into(),
            );
            return out;
        }
        Cmd::RideAfter(i) => {
            if !warrant::ride_after(world, i) {
                return toast("Not today.");
            }
            if world.action.standoff.is_none() {
                out.toast = news_since(world, before);
            }
            return out;
        }
        Cmd::RideWith(i) => warrant::ride_with(world, i),
        Cmd::Surrender => warrant::surrender(world),
        Cmd::Run => warrant::run(world),
        Cmd::LieLow => {
            if !warrant::lie_low(world) {
                return toast("Not today.");
            }
            return toast(
                "A week in the timber, a blanket and cold meat. Your people say they haven't seen you.",
            );
        }
        Cmd::Wear(i) => {
            wardrobe::wear(world, i);
            out.next = Some(menu(world, Sub::Dress));
            return out;
        }
        Cmd::TakeOff(s) => {
            wardrobe::take_off(world, s);
            out.next = Some(menu(world, Sub::Dress));
            return out;
        }
        Cmd::BuyItem(i) => {
            if !wardrobe::buy(world, i) {
                return toast("Not with what's in your purse.");
            }
            out.next = Some(menu(world, Sub::Clothes));
            out.toast = Some(format!(
                "Dunmore wraps the {} in brown paper. It's in your trunk.",
                ITEMS[i as usize].name
            ));
            return out;
        }
        Cmd::BuyMed(r) => {
            if !sickness::buy(world, r) {
                return toast("Not with what's in your purse.");
            }
            out.next = Some(menu(world, Sub::Medicines));
            out.toast = Some(format!("A dose of {} for the chest.", r.label()));
            return out;
        }
        Cmd::Dose(id, r) => {
            if !sickness::dose(world, id, r) {
                return toast("None left in the chest.");
            }
            out.next = Some(menu(world, Sub::Sick));
            out.toast = Some(format!("{} gets the {}.", world.name(id), r.label()));
            return out;
        }
        Cmd::Nurse(id) => sickness::nurse(world, id),
        Cmd::Doctor => {
            if !sickness::doctor(world, 0) {
                return toast("Three dollars you haven't got.");
            }
            out.toast = news_since(world, before);
            return out;
        }
        Cmd::GatherHerbs => {
            if !day_free(world) {
                return toast("Your day is spent.");
            }
            world.life.acted_on = Some(world.day);
            let found = sickness::gather(world);
            return toast(&if found.is_empty() {
                "A day in the bottoms and nothing you'd trust.".to_string()
            } else {
                format!(
                    "Back with {}.",
                    found
                        .iter()
                        .map(|r| r.label())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            });
        }
        Cmd::Delouse => {
            if !sickness::delouse(world) {
                return toast("Not today.");
            }
            return toast(
                "Every blanket in the kettle, the ticks burned. The house smells of lye and it's clean.",
            );
        }
        Cmd::Quarantine(on) => {
            sickness::quarantine(world, on);
            out.next = Some(menu(world, Sub::Sick));
            return out;
        }
        Cmd::FoulBlanket(t) => {
            if !sickness::foul_blanket(world, t) {
                return toast("Not today.");
            }
            return toast(
                "They thanked you for it. Their youngest wrapped up in it before you'd left the yard.",
            );
        }
        Cmd::Loot => {
            let took = wardrobe::loot_fallen(world);
            return toast(&if took.is_empty() {
                "Nothing worth the taking.".to_string()
            } else {
                format!("You took {}.", took.join(", "))
            });
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
    items.push(when(
        "Have it out with them",
        Cmd::Confront(t),
        free && grown && world.action.standoff.is_none(),
    ));
    items.push(item("After dark...", Cmd::Open(Sub::AfterDark(t))));
    items.push(item("Leave them be", Cmd::Close));
    MenuSpec::new(n.name.clone(), crate::panels::inspect(world, t), items)
}

fn after_dark(world: &World, t: NpcId) -> MenuSpec {
    let free = day_free(world);
    let night = action::night_free(world);
    let leverage = intrigue::held(world, t).is_some();
    let plan = action::raid_plan(world, t);
    let what = plan
        .as_ref()
        .map(|p| {
            p.objectives
                .iter()
                .map(|o| o.label())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let items = vec![
        when(
            "Slip onto their place",
            Cmd::Raid(t),
            night && plan.is_some(),
        ),
        when(
            if action::armed(world) {
                "Lie in wait on their road"
            } else {
                "Lie in wait on their road (nothing loaded)"
            },
            Cmd::Ambush(t),
            night && action::armed(world),
        ),
        when("Whisper against them", Cmd::Do(Activity::Slander(t)), free),
        when(
            "Bring them a blanket (from your lousy bedding)",
            Cmd::FoulBlanket(t),
            free && world.sickness.lousy.contains(&0),
        ),
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
        item("Think better of it", Cmd::Close),
    ];
    let moon = world.night_light(world.day);
    MenuSpec::new(
        "After dark",
        format!(
            "{} On their place tonight: {what}.",
            if moon > 0.6 {
                "A bright moon. You'll see well, and be seen."
            } else if moon > 0.25 {
                "Some moon, some cloud."
            } else {
                "Dark of the moon. Good for the wicked and the careful."
            }
        ),
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
                    item(
                        format!("Buy an old shotgun: ${}", arms::gun_price(world, 0)),
                        Cmd::BuyGun,
                    ),
                    item(
                        format!("Buy bar lead, 5 lb: ${}", arms::LEAD_PRICE),
                        Cmd::BuyLead,
                    ),
                    item("Dry goods...", Cmd::Open(Sub::Clothes)),
                    item("Physic...", Cmd::Open(Sub::Medicines)),
                    item("Sell...", Cmd::Open(Sub::StoreSell)),
                    item("Put it on the book", Cmd::Choose(Choice::Borrow)),
                    item("Walk out", Cmd::Close),
                ],
            )
        }
        Sub::Bench => {
            let h = &world.families[0].stores;
            let a = &h.arms;
            let mut items: Vec<Item> = Craft::ALL
                .into_iter()
                .map(|c| {
                    let need = match c {
                        Craft::Balls => "a pound of lead",
                        Craft::Cartridges => "ten balls and a measure of powder",
                        Craft::Rails => "a load of timber",
                    };
                    when(
                        format!("{} ({need})", capitalize(c.label())),
                        Cmd::Craft(c),
                        arms::can_craft(world, c),
                    )
                })
                .collect();
            for place in Hide::ALL {
                items.push(when(
                    format!("Keep the guns {}", place.label()),
                    Cmd::Hide(place),
                    a.hide != place,
                ));
            }
            items.push(item("Leave it", Cmd::Close));
            MenuSpec::new(
                "The workbench",
                format!(
                    "{} Sharps, {} old gun{}. {} balls, {} cartridges, {:.0} lb lead, {} rails. The guns are {}.",
                    a.rifles,
                    a.guns,
                    if a.guns == 1 { "" } else { "s" },
                    a.balls,
                    a.cartridges,
                    a.lead,
                    a.rails,
                    a.hide.label()
                ),
                items,
            )
        }
        Sub::Hands => hands_menu(world),
        Sub::Assign(id) => {
            let now = hands::task(world, id);
            let mut items: Vec<Item> = Task::ALL
                .into_iter()
                .map(|t| when(capitalize(t.label()), Cmd::Assign(id, t), t != now))
                .collect();
            if world.hands.is_hired(id) {
                items.push(item("Let them go", Cmd::LetGo(id)));
            }
            items.push(item("Back", Cmd::Open(Sub::Hands)));
            MenuSpec::new(
                world.name(id).to_string(),
                format!(
                    "{} now. A watch at night sees riders before they reach the gate; whoever sits up is no use in the field.",
                    capitalize(now.label())
                ),
                items,
            )
        }
        Sub::Hire => {
            let mut items: Vec<Item> = hands::candidates(world)
                .into_iter()
                .map(|id| {
                    let n = world.npc(id);
                    let hungry = world.families[n.family as usize].stores.desperate();
                    item(
                        format!(
                            "{}{}",
                            n.name,
                            if hungry {
                                " (the family is going hungry)"
                            } else {
                                ""
                            }
                        ),
                        Cmd::Hire(id),
                    )
                })
                .collect();
            if items.is_empty() {
                return MenuSpec::new(
                    "Nobody's asking",
                    "Nobody in the county wants your wages this month, or you've hands enough.",
                    vec![item("Back", Cmd::Open(Sub::Hands))],
                );
            }
            items.push(item("Back", Cmd::Open(Sub::Hands)));
            MenuSpec::new(
                "Hire a hand",
                format!(
                    "${} a month and board, paid on the first. Miss a month and they walk, and talk.",
                    hands::WAGE
                ),
                items,
            )
        }
        Sub::Guns(free_state) => {
            let side = if free_state {
                bleeding_kansas::sim::Faction::FreeState
            } else {
                bleeding_kansas::sim::Faction::ProSlavery
            };
            let mut items = Vec::new();
            for (c, co) in COMPANIES.iter().enumerate() {
                if co.faction != side || !hands::available(world, c) {
                    continue;
                }
                let yours = hands::will_ride(world, c);
                let cash = world.families[0].stores.cash;
                items.push(when(
                    format!(
                        "{} to sit on your place, 3 nights (${})",
                        capitalize(co.name),
                        hands::guard_price(c, 3)
                    ),
                    Cmd::HireGuard(c),
                    yours && cash >= hands::guard_price(c, 3) && hands::guarded(world).is_none(),
                ));
                items.push(when(
                    format!(
                        "{} to ride on someone tonight (${})...",
                        capitalize(co.name),
                        hands::raid_price(c)
                    ),
                    Cmd::Open(Sub::GunsOn(c)),
                    yours && cash >= hands::raid_price(c) && action::night_free(world),
                ));
            }
            if items.is_empty() {
                return MenuSpec::new(
                    "Nobody's for hire",
                    "The companies are gone home, or not come yet.",
                    vec![item("Leave it", Cmd::Close)],
                );
            }
            items.push(item("Leave it", Cmd::Close));
            MenuSpec::new(
                "Men who ride for pay",
                if world.npc(bleeding_kansas::sim::world::PLAYER).faction == side {
                    "They drink on your money and talk on it after. The other side's paper hears everything eventually."
                } else {
                    "They look you over and don't like your politics. Not for any money."
                },
                items,
            )
        }
        Sub::GunsOn(c) => {
            let mut items: Vec<Item> = hands::raid_targets(world, c)
                .into_iter()
                .map(|t| {
                    let fam = world.npc(t).family as usize;
                    item(
                        format!(
                            "The {} place{}",
                            world.families[fam].surname,
                            if world.families[fam].barn_standing {
                                ": burn the barn"
                            } else {
                                ": run off the stock"
                            }
                        ),
                        Cmd::HireRaid(c, t),
                    )
                })
                .collect();
            items.push(item("Think better of it", Cmd::Close));
            MenuSpec::new(
                capitalize(COMPANIES[c].name),
                "They'll go tonight. Nobody on that place will see your face, only theirs. What they say in their cups is their business.",
                items,
            )
        }
        Sub::Justice => justice_menu(world),
        Sub::Dress => dress_menu(world),
        Sub::Clothes => {
            let cash = world.families[0].stores.cash;
            let mut items: Vec<Item> = wardrobe::for_sale(world)
                .into_iter()
                .map(|i| {
                    let it = &ITEMS[i as usize];
                    let p = wardrobe::price(world, i);
                    when(
                        format!("{} (${p}){}", capitalize(it.name), perks(it)),
                        Cmd::BuyItem(i),
                        cash >= p,
                    )
                })
                .collect();
            items.push(item("Back", Cmd::Open(Sub::Store)));
            MenuSpec::new(
                "Dry goods",
                format!("Bolts, hats, boots and notions off the Westport wagon. You have ${cash}."),
                items,
            )
        }
        Sub::Medicines => {
            let cash = world.families[0].stores.cash;
            let chest = world.families[0].stores.medicine;
            let mut items: Vec<Item> = Remedy::ALL
                .into_iter()
                .filter(|r| r.price() > 0)
                .map(|r| {
                    when(
                        format!(
                            "{} (${}, {} in the chest)",
                            capitalize(r.label()),
                            r.price(),
                            chest[r.index()]
                        ),
                        Cmd::BuyMed(r),
                        cash >= r.price(),
                    )
                })
                .collect();
            items.push(item("Back", Cmd::Open(Sub::Store)));
            MenuSpec::new(
                "Physic",
                "Peruvian bark for the ague, calomel for everything else, laudanum for when nothing works.",
                items,
            )
        }
        Sub::Sick => sick_menu(world),
        Sub::Tend(id) => {
            let chest = world.families[0].stores.medicine;
            let mut items: Vec<Item> = Remedy::ALL
                .into_iter()
                .filter(|r| chest[r.index()] > 0)
                .map(|r| {
                    item(
                        format!("Give {} ({} left)", r.label(), chest[r.index()]),
                        Cmd::Dose(id, r),
                    )
                })
                .collect();
            items.push(when(
                "Sit up with them all day",
                Cmd::Nurse(id),
                day_free(world),
            ));
            items.push(item("Back", Cmd::Open(Sub::Sick)));
            let what = world
                .sickness
                .sick(id)
                .map(|c| c.disease.label())
                .unwrap_or("nothing now");
            MenuSpec::new(
                world.name(id).to_string(),
                format!(
                    "Down with {what}. Boiled water and broth do more for the bowel fevers than anything in a bottle."
                ),
                items,
            )
        }
        Sub::Papers => {
            let mut items: Vec<Item> = Vec::new();
            for i in warrant::open_papers(world) {
                let w = &world.warrants.list[i];
                let name = world.name(w.accused).to_string();
                if w.hunter == Some(bleeding_kansas::sim::world::PLAYER) {
                    items.push(when(
                        format!("Ride after {name} (${})", w.bounty),
                        Cmd::RideAfter(i),
                        free && world.action.standoff.is_none() && !warrant::held(world, w.accused),
                    ));
                } else {
                    items.push(when(
                        format!("Take the paper on {name} (${})", w.bounty),
                        Cmd::TakePaper(i),
                        true,
                    ));
                    items.push(when(
                        format!("Ride with the sheriff's men after {name}"),
                        Cmd::RideWith(i),
                        free,
                    ));
                }
            }
            if items.is_empty() {
                return MenuSpec::new(
                    "No papers",
                    "The justice has nobody wanted this week that you could fetch.",
                    vec![item("Back", Cmd::Open(Sub::Justice))],
                );
            }
            items.push(item("Back", Cmd::Open(Sub::Justice)));
            MenuSpec::new(
                "Wanted",
                "Paid by the Territory on delivery; half for a body. The justice doesn't ask which side you're on. He knows.",
                items,
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

/// What a piece does, for a shop line: warm, and whatever it helps.
fn perks(it: &wardrobe::Item) -> String {
    let mut p: Vec<String> = Vec::new();
    if it.warmth >= 0.2 {
        p.push("warm".into());
    }
    for &(s, _) in it.bonus.skills {
        p.push(s.label().into());
    }
    if it.bonus.stealth > 0.0 {
        p.push("quiet".into());
    }
    if it.bonus.stealth < 0.0 {
        p.push("seen a mile off".into());
    }
    if it.bonus.nerve > 0.0 {
        p.push("nerve".into());
    }
    if it.bonus.draw > 0.0 {
        p.push("quick draw".into());
    }
    if it.bonus.sway > 0.0 {
        p.push("steady aim".into());
    }
    if it.bonus.charm > 0.0 {
        p.push("charm".into());
    }
    if let Some(side) = it.mark {
        p.push(format!("reads {}", side.label()));
    }
    if p.is_empty() {
        String::new()
    } else {
        format!(": {}", p.join(", "))
    }
}

fn dress_menu(world: &World) -> MenuSpec {
    let me = bleeding_kansas::sim::world::PLAYER;
    let o = &world.npc(me).outfit;
    let mut items: Vec<Item> = Vec::new();
    for (s, w) in o.on.iter().enumerate() {
        if let Some(w) = w {
            let it = &ITEMS[w.item as usize];
            let taken = if w.from.is_some() { " (taken)" } else { "" };
            items.push(item(
                format!("Take off the {}{taken}", it.name),
                Cmd::TakeOff(s),
            ));
        }
    }
    for (i, w) in world.wardrobe.closet.iter().enumerate() {
        let it = &ITEMS[w.item as usize];
        let taken = match w.from {
            Some((whose, _)) => format!(" ({}'s)", world.name(whose)),
            None => String::new(),
        };
        items.push(item(
            format!("Put on the {}{taken}{}", it.name, perks(it)),
            Cmd::Wear(i),
        ));
    }
    items.push(item("Done", Cmd::Close));
    let looks: Vec<&str> = o.looks().iter().map(|l| l.name).collect();
    MenuSpec::new(
        "Dress",
        format!(
            "{}.{}{}",
            capitalize(&wardrobe::describe(world, me)),
            if looks.is_empty() {
                String::new()
            } else {
                format!(" The county reads you as {}.", looks.join(" and "))
            },
            match o.colors() {
                Some(side) => format!(" From a distance: {}.", side.label()),
                None => String::new(),
            }
        ),
        items,
    )
}

fn sick_menu(world: &World) -> MenuSpec {
    let mut items: Vec<Item> = world
        .living()
        .filter(|n| n.family == 0)
        .filter_map(|n| {
            world.sickness.sick(n.id).map(|c| {
                item(
                    format!("{}: {}", n.name, c.disease.label()),
                    Cmd::Open(Sub::Tend(n.id)),
                )
            })
        })
        .collect();
    let cash = world.families[0].stores.cash;
    items.push(when(
        format!("Send for {} (${})", sickness::DOCTOR, sickness::DOCTOR_FEE),
        Cmd::Doctor,
        cash >= sickness::DOCTOR_FEE,
    ));
    let kept = world.sickness.quarantine.contains(&0);
    items.push(item(
        if kept {
            "Let the children go to school and meeting again"
        } else {
            "Keep the children home"
        },
        Cmd::Quarantine(!kept),
    ));
    if world.sickness.lousy.contains(&0) {
        items.push(when("Boil the bedding", Cmd::Delouse, day_free(world)));
    }
    items.push(item("Leave it", Cmd::Close));
    let chest = world.families[0].stores.medicine;
    let stock: Vec<String> = Remedy::ALL
        .into_iter()
        .filter(|r| chest[r.index()] > 0)
        .map(|r| format!("{} {}", chest[r.index()], r.label()))
        .collect();
    MenuSpec::new(
        "The sick",
        format!(
            "In the chest: {}.{}",
            if stock.is_empty() {
                "nothing".to_string()
            } else {
                stock.join(", ")
            },
            if world.sickness.lousy.contains(&0) {
                " There are lice in the bedding."
            } else {
                ""
            }
        ),
        items,
    )
}

/// Who works the place, and at what.
fn hands_menu(world: &World) -> MenuSpec {
    let mut items: Vec<Item> = hands::workers(world)
        .into_iter()
        .map(|id| {
            let hired = if world.hands.is_hired(id) {
                ", hired"
            } else {
                ""
            };
            item(
                format!(
                    "{}{hired}: {}",
                    world.name(id),
                    hands::task(world, id).label()
                ),
                Cmd::Open(Sub::Assign(id)),
            )
        })
        .collect();
    items.push(when(
        "Hire a hand...",
        Cmd::Open(Sub::Hire),
        world.hands.hired.len() < hands::MAX_HANDS,
    ));
    items.push(item("Leave them to it", Cmd::Close));
    let guard = hands::guarded(world)
        .map(|c| format!(" {} are in the barn.", capitalize(c.name)))
        .unwrap_or_default();
    MenuSpec::new(
        "Hands",
        format!(
            "Grown kin and hired hands take one job each. The rest of the house does what it can.{guard}"
        ),
        items,
    )
}

/// The justice of the peace: suits, papers, and a cell if you want one.
fn justice_menu(world: &World) -> MenuSpec {
    let free = day_free(world);
    let me = bleeding_kansas::sim::world::PLAYER;
    let mut items = vec![
        item("Sue a neighbor...", Cmd::Open(Sub::Sue)),
        item("Who's wanted...", Cmd::Open(Sub::Papers)),
    ];
    let price = warrant::price_on(world, me);
    if price > 0 {
        items.push(when("Give yourself up", Cmd::Surrender, free));
    }
    items.push(item("Leave", Cmd::Close));
    MenuSpec::new(
        "The justice of the peace",
        if price > 0 {
            format!(
                "There's a paper on you in the drawer: ${price} on delivery. The justice looks at you like a man counting money."
            )
        } else {
            "Appointed by the bogus legislature. He writes papers on what people swear to, and people swear to what they believe.".to_string()
        },
        items,
    )
}

/// What a wanted man can do at home.
pub fn wanted_choices(world: &World) -> Vec<Item> {
    let me = bleeding_kansas::sim::world::PLAYER;
    if warrant::price_on(world, me) == 0 {
        return Vec::new();
    }
    vec![
        when(
            "Lie low in the timber a week",
            Cmd::LieLow,
            day_free(world) && world.warrants.lying_low.is_none(),
        ),
        when(
            "Light out for the States (six weeks)",
            Cmd::Run,
            day_free(world),
        ),
    ]
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

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}
