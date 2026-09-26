//! The Territory and the States beyond the county line: a dated timeline of
//! real events, and the partisan papers that told the county what they meant.
//!
//! History here is an input, not a cutscene. Each event moves grievance,
//! emotions, freight or credit. Each paper prints its side's version of the
//! week, and readers who believe it hold on hard (ext. §14).

use super::calendar::Day;
use super::events::{EventId, EventKind, Source, Suspect};
use super::psyche;
use super::world::{Faction, NpcId, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Paper {
    /// Lawrence, G. W. Brown. Free-State. Press thrown in the river May 1856.
    HeraldOfFreedom,
    /// Lawrence. Free-State. Destroyed alongside the Herald.
    KansasFreeState,
    /// Atchison, Stringfellow & Kelley. Pro-Slavery. Never missed an issue.
    SquatterSovereign,
}

impl Paper {
    pub const ALL: [Paper; 3] = [
        Paper::HeraldOfFreedom,
        Paper::KansasFreeState,
        Paper::SquatterSovereign,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Paper::HeraldOfFreedom => "Herald of Freedom",
            Paper::KansasFreeState => "Kansas Free State",
            Paper::SquatterSovereign => "Squatter Sovereign",
        }
    }

    pub fn faction(self) -> Faction {
        match self {
            Paper::HeraldOfFreedom | Paper::KansasFreeState => Faction::FreeState,
            Paper::SquatterSovereign => Faction::ProSlavery,
        }
    }

    /// Day of the week it goes to press (day % 7).
    fn press_day(self) -> u32 {
        match self {
            Paper::HeraldOfFreedom => 5,
            Paper::KansasFreeState => 1,
            Paper::SquatterSovereign => 2,
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// What a history event does to the county, in numbers.
#[derive(Clone, Copy, Debug, Default)]
pub struct Effect {
    pub grievance_free: i32,
    pub grievance_pro: i32,
    pub zeal_free: f32,
    pub zeal_pro: f32,
    pub fear_all: f32,
    /// Multiplies Westport freight until changed again.
    pub freight: Option<f32>,
    /// Multiplies trail safety for hides and freighters.
    pub trail: Option<f32>,
    /// Days of federal peace: revenge is damped (negative feedback).
    pub pacify_days: u32,
    /// Credit tightens: the store halves what it will lend.
    pub credit_crunch: bool,
    /// Destroys these presses.
    pub silence: &'static [Paper],
    /// Brings a press back.
    pub revive: &'static [Paper],
    /// Extra corn on the eastern market: prices ease.
    pub grain_glut: f32,
}

pub struct Moment {
    pub date: (i32, u32, u32),
    pub title: &'static str,
    /// How the Free-State papers told it.
    pub free_state: &'static str,
    /// How the Pro-Slavery papers told it.
    pub pro_slavery: &'static str,
    pub effect: Effect,
}

const NONE: &[Paper] = &[];

/// The timeline. Dates are as reported at the time; the county hears of
/// distant events about a week late (`heard_after`).
pub const TIMELINE: &[Moment] = &[
    Moment {
        date: (1855, 11, 21),
        title: "Charles Dow shot dead at Hickory Point",
        free_state: "A Free-State man murdered in a claim dispute by the ruffian Coleman.",
        pro_slavery: "A quarrel over a claim; Coleman defended himself.",
        effect: Effect {
            grievance_free: 15,
            zeal_free: 8.0,
            fear_all: 5.0,
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1855, 11, 27),
        title: "Missourians camp on the Wakarusa. Lawrence besieged",
        free_state: "Fifteen hundred border ruffians encamp to wipe out Lawrence.",
        pro_slavery: "The sheriff's posse gathers to enforce the laws of the Territory.",
        effect: Effect {
            grievance_free: 15,
            grievance_pro: 10,
            fear_all: 20.0,
            freight: Some(0.6),
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1855, 12, 6),
        title: "Thomas Barber killed on the road home",
        free_state: "Barber, unarmed, shot by the posse on his way home.",
        pro_slavery: "An abolitionist falls in an exchange on the road.",
        effect: Effect {
            grievance_free: 20,
            zeal_free: 12.0,
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1855, 12, 8),
        title: "Governor Shannon's treaty ends the Wakarusa War",
        free_state: "Shannon backs down. Lawrence stands.",
        pro_slavery: "The Governor's weakness lets the abolitionists escape justice.",
        effect: Effect {
            fear_all: -10.0,
            freight: Some(1.0),
            pacify_days: 30,
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1856, 1, 18),
        title: "Reese Brown hacked to death near Easton",
        free_state: "Captain Brown butchered with a hatchet by drunken Kickapoo Rangers.",
        pro_slavery: "A notorious agitator killed after an affray at Easton.",
        effect: Effect {
            grievance_free: 15,
            zeal_free: 10.0,
            fear_all: 8.0,
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1856, 2, 11),
        title: "President Pierce declares the Topeka movement insurrection",
        free_state: "Pierce sides with the bogus legislature against the people.",
        pro_slavery: "The President stands by the lawful government of Kansas.",
        effect: Effect {
            zeal_pro: 10.0,
            grievance_free: 10,
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1856, 4, 5),
        title: "Treaty of Paris ends the Crimean War",
        free_state: "Peace in Europe; eastern grain prices ease.",
        pro_slavery: "Peace in Europe; grain is cheaper at Westport.",
        effect: Effect {
            grain_glut: 120.0,
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1856, 4, 23),
        title: "Sheriff Jones shot in Lawrence",
        free_state: "Jones wounded by an unknown hand; Lawrence regrets the act.",
        pro_slavery: "Sheriff Jones shot down in the abolition town.",
        effect: Effect {
            grievance_pro: 25,
            zeal_pro: 12.0,
            fear_all: 8.0,
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1856, 5, 21),
        title: "Lawrence sacked. Presses destroyed, Free State Hotel burned",
        free_state: "The ruffians gut Lawrence. Our type is in the river.",
        pro_slavery: "The nuisances at Lawrence abated by order of the grand jury.",
        effect: Effect {
            grievance_free: 40,
            zeal_free: 15.0,
            zeal_pro: 5.0,
            fear_all: 10.0,
            silence: &[Paper::HeraldOfFreedom, Paper::KansasFreeState],
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1856, 5, 22),
        title: "Senator Sumner caned at his desk by Preston Brooks",
        free_state: "The South answers argument with a cane.",
        pro_slavery: "Brooks chastises the slanderer of South Carolina.",
        effect: Effect {
            zeal_free: 10.0,
            zeal_pro: 6.0,
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1856, 5, 25),
        title: "Five men dragged out and killed on Pottawatomie Creek",
        free_state: "Dark news from Pottawatomie; the deed is not ours.",
        pro_slavery: "Abolition assassins murder five law-abiding men in the night.",
        effect: Effect {
            grievance_pro: 40,
            zeal_pro: 15.0,
            fear_all: 18.0,
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1856, 6, 2),
        title: "Fight at Black Jack. Pate's company surrenders to John Brown",
        free_state: "Captain Pate and his marauders taken at Black Jack.",
        pro_slavery: "Pate taken under a flag of truce by Brown's banditti.",
        effect: Effect {
            grievance_pro: 15,
            grievance_free: 5,
            fear_all: 10.0,
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1856, 6, 15),
        title: "The Missouri River closed to Free-State emigrants",
        free_state: "Boats searched at Lexington; our people turned back, their rifles taken.",
        pro_slavery: "Missouri guards her river from the armed hordes of the Emigrant Aid Company.",
        effect: Effect {
            freight: Some(0.4),
            grievance_free: 10,
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1856, 7, 4),
        title: "Colonel Sumner's dragoons disperse the Topeka legislature",
        free_state: "Federal bayonets scatter the people's legislature.",
        pro_slavery: "The unlawful body at Topeka dispersed at last.",
        effect: Effect {
            grievance_free: 10,
            zeal_free: 6.0,
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1856, 8, 12),
        title: "Free-State men take Franklin, then Fort Saunders and Fort Titus",
        free_state: "The proslavery blockhouses fall; Titus a prisoner.",
        pro_slavery: "Lane's army storms Franklin and burns Titus out.",
        effect: Effect {
            grievance_pro: 25,
            zeal_free: 8.0,
            fear_all: 12.0,
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1856, 8, 25),
        title: "Cheyenne and emigrants clash on the Platte road",
        free_state: "Trouble with the Cheyenne on the Platte; the emigrant road is unsafe.",
        pro_slavery: "Trouble with the Cheyenne on the Platte; freighters wait at Fort Kearny.",
        effect: Effect {
            trail: Some(0.6),
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1856, 8, 30),
        title: "Osawatomie burned. Frederick Brown killed",
        free_state: "Reid's army burns Osawatomie; Brown's son shot down.",
        pro_slavery: "Brown's nest at Osawatomie cleaned out.",
        effect: Effect {
            grievance_free: 25,
            zeal_free: 10.0,
            fear_all: 10.0,
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1856, 9, 9),
        title: "Governor Geary arrives with the dragoons",
        free_state: "A new governor. The times, some say, are changing.",
        pro_slavery: "Another governor from Washington, another meddler.",
        effect: Effect {
            pacify_days: 90,
            freight: Some(0.9),
            fear_all: -8.0,
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1856, 9, 15),
        title: "Twenty-seven hundred Missourians turned back from Lawrence",
        free_state: "Geary meets the ruffian army at Franklin and sends it home.",
        pro_slavery: "The Governor disarms the Territorial militia at the very gates of Lawrence.",
        effect: Effect {
            fear_all: -5.0,
            grievance_pro: 10,
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1856, 11, 4),
        title: "Buchanan elected President",
        free_state: "Buchanan wins. Frémont carries the North.",
        pro_slavery: "Buchanan! The Union and the Constitution are safe.",
        effect: Effect {
            zeal_pro: 10.0,
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1857, 3, 6),
        title: "The Supreme Court decides Dred Scott",
        free_state: "Taney: the Negro has no rights the white man is bound to respect.",
        pro_slavery: "The Court settles it: Congress cannot bar slavery from the Territories.",
        effect: Effect {
            zeal_free: 12.0,
            zeal_pro: 12.0,
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1857, 7, 29),
        title: "Colonel Sumner's cavalry charges the Cheyenne on the Solomon",
        free_state: "Sabres against the Cheyenne on Solomon's Fork.",
        pro_slavery: "Sumner punishes the Cheyenne on the Solomon.",
        effect: Effect {
            trail: Some(0.8),
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1857, 8, 24),
        title: "Ohio Life Insurance fails. Panic in New York",
        free_state: "Banks failing in the East. Money is scarce.",
        pro_slavery: "Northern banks collapse; hard money only at Westport.",
        effect: Effect {
            credit_crunch: true,
            freight: Some(0.7),
            ..Effect::ZERO
        },
    },
    Moment {
        date: (1857, 11, 1),
        title: "The Herald of Freedom prints again",
        free_state: "The Herald returns.",
        pro_slavery: "Brown's lying sheet is back.",
        effect: Effect {
            revive: &[Paper::HeraldOfFreedom],
            ..Effect::ZERO
        },
    },
];

impl Effect {
    pub const ZERO: Effect = Effect {
        grievance_free: 0,
        grievance_pro: 0,
        zeal_free: 0.0,
        zeal_pro: 0.0,
        fear_all: 0.0,
        freight: None,
        trail: None,
        pacify_days: 0,
        credit_crunch: false,
        silence: NONE,
        revive: NONE,
        grain_glut: 0.0,
    };
}

/// News from outside the county arrives about a week late.
pub const HEARD_AFTER: u32 = 7;

/// State the news system keeps.
#[derive(Clone, Debug, Default)]
pub struct Press {
    /// Which presses are running (indexed by `Paper`).
    pub silenced: [bool; 3],
    /// Timeline entries already heard.
    pub heard: usize,
    /// Local events each paper has already printed.
    pub printed: Vec<EventId>,
}

fn day_of(date: (i32, u32, u32)) -> u32 {
    let mut d = Day(0);
    while d.date() < date {
        d = Day(d.0 + 1);
    }
    d.0
}

/// Daily: news arrives, and papers go to press.
pub fn daily(world: &mut World) {
    while let Some(m) = TIMELINE.get(world.press.heard) {
        if day_of(m.date) + HEARD_AFTER > world.day.0 {
            break;
        }
        let index = world.press.heard;
        world.press.heard += 1;
        // Events before the game starts are already old news.
        if day_of(m.date) + HEARD_AFTER + 30 < world.day.0 {
            continue;
        }
        let id = world.emit_root(EventKind::History { index }, None);
        apply(world, &m.effect);
        // The Herald and the Sovereign cover the Territory; the Kansas Free
        // State sticks to local news.
        for paper in [Paper::HeraldOfFreedom, Paper::SquatterSovereign] {
            if !world.press.silenced[paper.index()] {
                world.emit_root(
                    EventKind::Headline {
                        paper,
                        about: None,
                        blamed: None,
                        history: Some(index),
                    },
                    Some(id),
                );
            }
        }
    }

    for paper in Paper::ALL {
        if world.day.0 % 7 == paper.press_day() && !world.press.silenced[paper.index()] {
            print_local(world, paper);
        }
    }
}

pub fn apply(world: &mut World, e: &Effect) {
    // Distant outrages count, but less than a neighbor's barn.
    world.grievance[Faction::FreeState.index()] += e.grievance_free / 2;
    world.grievance[Faction::ProSlavery.index()] += e.grievance_pro / 2;
    let people: Vec<(NpcId, Faction)> = world.living().map(|n| (n.id, n.faction)).collect();
    for (id, faction) in people {
        let zeal = match faction {
            Faction::FreeState => e.zeal_free,
            Faction::ProSlavery => e.zeal_pro,
        };
        psyche::feel(world, id, |em| {
            em.zeal += zeal;
            em.fear += e.fear_all;
        });
    }
    if let Some(f) = e.freight {
        world.market.freight_factor = f;
    }
    if let Some(t) = e.trail {
        world.market.trail_safety = t;
    }
    if e.pacify_days > 0 {
        world.pacified_until = Some(Day(world.day.0 + e.pacify_days));
    }
    if e.credit_crunch {
        world.credit_crunch = true;
    }
    if e.grain_glut > 0.0 {
        world.market.goods[super::market::Good::Corn.index()].stock += e.grain_glut;
    }
    for p in e.silence {
        world.press.silenced[p.index()] = true;
    }
    for p in e.revive {
        world.press.silenced[p.index()] = false;
    }
}

/// The week's worst local harm, told the paper's way: someone on the other
/// side did it, whatever the truth.
fn print_local(world: &mut World, paper: Paper) {
    let side = paper.faction();
    let week_ago = world.day.0.saturating_sub(7);
    let story = world
        .events
        .iter()
        .rev()
        .take_while(|e| e.day.0 >= week_ago)
        .filter(|e| !world.press.printed.contains(&e.id))
        .filter_map(|e| {
            let weight = match e.kind {
                EventKind::Death { .. } => 5,
                EventKind::Wounded { .. } => 4,
                EventKind::Fire {
                    spread_from: None, ..
                } => 3,
                EventKind::Cruelty { .. } => 2,
                EventKind::Theft { .. } => 1,
                _ => return None,
            };
            let victim = super::attribution::victim_of(world, e.id)?;
            // Papers print their own side's suffering.
            (world.npc(victim).faction == side).then_some((weight, e.id))
        })
        .max_by_key(|(w, _)| *w)
        .map(|(_, id)| id);
    let Some(about) = story else {
        return;
    };
    // The villain: whoever the paper's own readers blame most, on the other side.
    let blamed = villain(world, about, side);
    world.press.printed.push(about);
    world.emit_root(
        EventKind::Headline {
            paper,
            about: Some(about),
            blamed,
            history: None,
        },
        Some(about),
    );
}

fn villain(world: &World, about: EventId, side: Faction) -> Option<Suspect> {
    let mut tally: Vec<(NpcId, u32)> = Vec::new();
    for n in world.living().filter(|n| n.faction == side) {
        if let Some(m) = n.memory_of(about)
            && let Suspect::Person(p) = m.believed
            && world.npc(p).faction != side
        {
            match tally.iter_mut().find(|(q, _)| *q == p) {
                Some(t) => t.1 += m.confidence as u32,
                None => tally.push((p, m.confidence as u32)),
            }
        }
    }
    tally
        .into_iter()
        .max_by_key(|t| t.1)
        .map(|(p, _)| Suspect::Person(p))
}

/// Readers of a paper take its version as their own. Printed beliefs are
/// confident and heavy: gossip will struggle to shake them.
pub fn read(world: &mut World, ev_id: EventId, paper: Paper, about: EventId, blamed: Suspect) {
    let readers: Vec<NpcId> = world
        .living()
        .filter(|n| n.faction == paper.faction() && n.temperament.literacy > 0.4)
        .map(|n| n.id)
        .collect();
    let parent = world.events[ev_id as usize].clone();
    for r in readers {
        let skeptic = world.npc(r).temperament.skepticism;
        let confidence = (85.0 - 40.0 * skeptic) as u8;
        world.emit_child(
            &parent,
            EventKind::Belief {
                holder: r,
                about,
                blamed,
                confidence,
                source: Source::Newspaper(paper),
                reason: "it was in the paper",
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_is_in_order() {
        for pair in TIMELINE.windows(2) {
            assert!(
                pair[0].date <= pair[1].date,
                "{} out of order",
                pair[1].title
            );
        }
    }

    #[test]
    fn day_of_matches_calendar() {
        assert_eq!(day_of((1855, 11, 1)), 0);
        assert_eq!(Day(day_of((1856, 5, 21))).date(), (1856, 5, 21));
    }

    #[test]
    fn sack_of_lawrence_silences_the_free_state_press() {
        let mut w = World::new(1);
        w.run_days(day_of((1856, 5, 21)) + HEARD_AFTER + 1);
        assert!(w.press.silenced[Paper::HeraldOfFreedom.index()]);
        assert!(w.press.silenced[Paper::KansasFreeState.index()]);
        assert!(!w.press.silenced[Paper::SquatterSovereign.index()]);
    }

    #[test]
    fn history_raises_grievance() {
        let mut w = World::new(1);
        w.run_days(day_of((1855, 12, 20)));
        let heard = w
            .events
            .iter()
            .filter(|e| matches!(e.kind, EventKind::History { .. }))
            .count();
        assert!(
            heard >= 3,
            "Hickory Point and the Wakarusa War reach the county"
        );
    }
}
