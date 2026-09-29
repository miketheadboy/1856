//! What people wear, and what it does (the outfit pass). Clothes in 1856
//! Kansas were warmth, trade, and a flag: a red flannel shirt and a Bowie
//! knife said Missouri from a hundred yards; a black frock said the pulpit;
//! a boiled shirt said you meant to court someone, or sell them something.
//!
//! - **Pieces.** Hats, coats, shirts, footwear and kit (knives, belts,
//!   spectacles, a Bible, a flask, a watch). Each is warm or not, helps a
//!   skill or doesn't, shows a side or doesn't.
//! - **Looks.** Every path has its look. Wear the whole of it and the
//!   county reads you that way: a set bonus on top of the pieces.
//! - **Evidence.** Clothes remember where they came from. Take a dead man's
//!   hat and wear it to church, and his widow knows that hat.
//! - **Colors.** A marked piece is what a witness sees when they can't see a
//!   face. Wear the other side's colors on a raid and they blame the other
//!   side.

use super::calendar::{Day, Season};
use super::events::{EventId, EventKind, Source, Suspect, WorldEvent};
use super::life::Skill;
use super::psyche::LifeStage;
use super::world::{Faction, NpcId, PLAYER, World, is_woman};

pub type ItemId = u8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Hat,
    Coat,
    Shirt,
    Feet,
    /// Two kit slots: a knife and a Bible, a flask and a watch.
    Kit,
}

/// What a piece does for whoever wears it. Every field is read somewhere:
/// `skills` by `life::Life::skill` (the player) and the NPC checks that use
/// the same skill; `nerve` by `action::nerve`; `draw` by `action::their_draw`
/// and the player's draw; `sway` by `action::sway`; `stealth` by the raid
/// plan and perception; `charm` by courting; `warmth` by exposure and
/// sickness.
#[derive(Clone, Copy, Debug, Default)]
pub struct Bonus {
    pub skills: &'static [(Skill, f32)],
    pub nerve: f32,
    /// Seconds off the draw.
    pub draw: f32,
    /// Off the wander of your sights.
    pub sway: f32,
    pub stealth: f32,
    pub charm: f32,
}

const NONE: Bonus = Bonus {
    skills: &[],
    nerve: 0.0,
    draw: 0.0,
    sway: 0.0,
    stealth: 0.0,
    charm: 0.0,
};

pub struct Item {
    pub name: &'static str,
    pub slot: Slot,
    /// Dollars at Dunmore's. Zero: not sold, only made or taken.
    pub price: i32,
    /// 0..0.5 each; a whole outfit near 1 keeps a body warm in January.
    pub warmth: f32,
    pub bonus: Bonus,
    /// Which side it shows, to anyone who can't see your face.
    pub mark: Option<Faction>,
    /// Some(true): a woman's piece; Some(false): a man's. None: anyone's.
    pub women: Option<bool>,
    /// Worth taking off a body or out of a house.
    pub dear: bool,
}

macro_rules! item {
    ($name:expr, $slot:ident, $price:expr, $warmth:expr, $bonus:expr) => {
        Item {
            name: $name,
            slot: Slot::$slot,
            price: $price,
            warmth: $warmth,
            bonus: $bonus,
            mark: None,
            women: None,
            dear: false,
        }
    };
}

const fn b(skills: &'static [(Skill, f32)]) -> Bonus {
    Bonus { skills, ..NONE }
}

/// The catalogue. Index is the `ItemId`; order is fixed (saves and tests
/// refer to it).
pub const ITEMS: &[Item] = &[
    // Hats.
    item!("slouch hat", Hat, 1, 0.05, NONE),
    item!("straw hat", Hat, 1, 0.0, b(&[(Skill::Farming, 0.04)])),
    Item {
        dear: true,
        ..item!(
            "stovepipe hat",
            Hat,
            4,
            0.05,
            Bonus {
                skills: &[(Skill::Oratory, 0.05), (Skill::Trade, 0.04)],
                stealth: -0.1,
                ..NONE
            }
        )
    },
    item!(
        "preacher's hat",
        Hat,
        2,
        0.05,
        b(&[(Skill::Scripture, 0.06)])
    ),
    Item {
        women: Some(true),
        ..item!("sunbonnet", Hat, 1, 0.02, b(&[(Skill::Farming, 0.03)]))
    },
    item!(
        "coonskin cap",
        Hat,
        1,
        0.12,
        Bonus {
            skills: &[(Skill::Trapping, 0.06)],
            stealth: 0.03,
            ..NONE
        }
    ),
    Item {
        dear: true,
        ..item!(
            "beaver hat",
            Hat,
            5,
            0.08,
            Bonus {
                skills: &[(Skill::Trade, 0.05)],
                charm: 0.06,
                ..NONE
            }
        )
    },
    item!(
        "forage cap",
        Hat,
        1,
        0.03,
        Bonus {
            nerve: 0.02,
            ..NONE
        }
    ),
    Item {
        mark: Some(Faction::ProSlavery),
        ..item!(
            "slouch hat and feather",
            Hat,
            1,
            0.05,
            Bonus {
                nerve: 0.02,
                ..NONE
            }
        )
    },
    Item {
        women: Some(true),
        dear: true,
        ..item!(
            "bonnet with ribbons",
            Hat,
            3,
            0.03,
            Bonus {
                charm: 0.08,
                ..NONE
            }
        )
    },
    // Coats.
    item!("sack coat", Coat, 4, 0.2, NONE),
    Item {
        dear: true,
        ..item!(
            "frock coat",
            Coat,
            8,
            0.2,
            Bonus {
                skills: &[
                    (Skill::Oratory, 0.06),
                    (Skill::Trade, 0.04),
                    (Skill::Letters, 0.03)
                ],
                charm: 0.04,
                ..NONE
            }
        )
    },
    item!(
        "black frock",
        Coat,
        7,
        0.2,
        b(&[(Skill::Scripture, 0.08), (Skill::Oratory, 0.03)])
    ),
    item!(
        "buffalo coat",
        Coat,
        6,
        0.45,
        Bonus {
            stealth: -0.08,
            ..NONE
        }
    ),
    item!(
        "blanket capote",
        Coat,
        3,
        0.35,
        Bonus {
            stealth: 0.04,
            ..NONE
        }
    ),
    item!(
        "fringed hunting shirt",
        Coat,
        3,
        0.2,
        Bonus {
            skills: &[(Skill::Trapping, 0.06)],
            stealth: 0.06,
            ..NONE
        }
    ),
    item!("linen duster", Coat, 3, 0.08, Bonus { draw: 0.03, ..NONE }),
    item!(
        "army greatcoat",
        Coat,
        5,
        0.35,
        Bonus {
            nerve: 0.03,
            ..NONE
        }
    ),
    Item {
        women: Some(true),
        ..item!("wool shawl", Coat, 2, 0.25, NONE)
    },
    item!("patched coat", Coat, 0, 0.12, NONE),
    // Shirts.
    item!(
        "hickory shirt",
        Shirt,
        1,
        0.05,
        b(&[(Skill::Farming, 0.04), (Skill::Carpentry, 0.03)])
    ),
    Item {
        mark: Some(Faction::ProSlavery),
        ..item!(
            "red flannel shirt",
            Shirt,
            1,
            0.1,
            Bonus {
                nerve: 0.02,
                ..NONE
            }
        )
    },
    item!(
        "boiled white shirt",
        Shirt,
        2,
        0.03,
        Bonus {
            skills: &[(Skill::Oratory, 0.03)],
            charm: 0.08,
            ..NONE
        }
    ),
    item!("linsey shirt", Shirt, 1, 0.08, NONE),
    Item {
        women: Some(true),
        ..item!(
            "calico dress",
            Shirt,
            2,
            0.08,
            Bonus {
                charm: 0.04,
                ..NONE
            }
        )
    },
    Item {
        women: Some(true),
        dear: true,
        ..item!(
            "silk dress",
            Shirt,
            9,
            0.05,
            Bonus {
                charm: 0.12,
                skills: &[(Skill::Trade, 0.03)],
                ..NONE
            }
        )
    },
    item!("rags", Shirt, 0, 0.02, NONE),
    // Feet.
    item!("brogans", Feet, 2, 0.08, b(&[(Skill::Farming, 0.02)])),
    Item {
        dear: true,
        ..item!(
            "riding boots",
            Feet,
            4,
            0.1,
            Bonus {
                nerve: 0.02,
                draw: 0.02,
                ..NONE
            }
        )
    },
    item!(
        "moccasins",
        Feet,
        1,
        0.06,
        Bonus {
            stealth: 0.1,
            ..NONE
        }
    ),
    item!("bare feet", Feet, 0, 0.0, NONE),
    // Kit.
    Item {
        mark: Some(Faction::ProSlavery),
        dear: true,
        ..item!(
            "Bowie knife",
            Kit,
            3,
            0.0,
            Bonus {
                nerve: 0.04,
                ..NONE
            }
        )
    },
    Item {
        dear: true,
        ..item!(
            "revolver belt",
            Kit,
            12,
            0.0,
            Bonus {
                draw: 0.08,
                nerve: 0.02,
                ..NONE
            }
        )
    },
    item!("spectacles", Kit, 2, 0.0, b(&[(Skill::Letters, 0.08)])),
    item!("pocket Bible", Kit, 1, 0.0, b(&[(Skill::Scripture, 0.06)])),
    item!(
        "whiskey flask",
        Kit,
        1,
        0.02,
        Bonus {
            skills: &[(Skill::Drink, 0.08)],
            nerve: 0.02,
            sway: -0.05,
            ..NONE
        }
    ),
    Item {
        dear: true,
        ..item!(
            "watch and chain",
            Kit,
            10,
            0.0,
            Bonus {
                skills: &[(Skill::Trade, 0.06)],
                charm: 0.03,
                ..NONE
            }
        )
    },
    item!(
        "neckerchief",
        Kit,
        1,
        0.03,
        Bonus {
            stealth: 0.03,
            ..NONE
        }
    ),
    Item {
        mark: Some(Faction::FreeState),
        ..item!("Sharps sling", Kit, 2, 0.0, Bonus { sway: 0.08, ..NONE })
    },
    item!(
        "tin star",
        Kit,
        0,
        0.0,
        Bonus {
            nerve: 0.05,
            ..NONE
        }
    ),
    item!(
        "powder horn",
        Kit,
        1,
        0.0,
        Bonus {
            sway: 0.03,
            skills: &[(Skill::Trapping, 0.02)],
            ..NONE
        }
    ),
];

pub fn id(name: &str) -> ItemId {
    ITEMS
        .iter()
        .position(|i| i.name == name)
        .unwrap_or_else(|| panic!("no such piece: {name}")) as ItemId
}

pub fn item(i: ItemId) -> &'static Item {
    &ITEMS[i as usize]
}

/// A path's look: wear all of it and the county reads you that way.
pub struct Look {
    pub name: &'static str,
    /// The life path it goes with (`life::paths`), if any.
    pub path: Option<&'static str>,
    pub pieces: &'static [&'static str],
    pub bonus: Bonus,
}

pub const LOOKS: &[Look] = &[
    Look {
        name: "the preacher",
        path: Some("lay preacher"),
        pieces: &["preacher's hat", "black frock", "pocket Bible"],
        bonus: Bonus {
            skills: &[(Skill::Scripture, 0.1)],
            nerve: 0.02,
            ..NONE
        },
    },
    Look {
        name: "the silver tongue",
        path: Some("silver tongue"),
        pieces: &["stovepipe hat", "frock coat", "boiled white shirt"],
        bonus: Bonus {
            skills: &[(Skill::Oratory, 0.1)],
            charm: 0.05,
            ..NONE
        },
    },
    Look {
        name: "the rake",
        path: Some("rake"),
        pieces: &["beaver hat", "boiled white shirt", "riding boots"],
        bonus: Bonus {
            charm: 0.15,
            ..NONE
        },
    },
    Look {
        name: "the souse",
        path: Some("town souse"),
        pieces: &["patched coat", "whiskey flask"],
        bonus: Bonus {
            skills: &[(Skill::Drink, 0.12)],
            ..NONE
        },
    },
    Look {
        name: "the ruffian",
        path: Some("ruffian"),
        pieces: &["slouch hat and feather", "red flannel shirt", "Bowie knife"],
        bonus: Bonus {
            nerve: 0.06,
            draw: 0.03,
            ..NONE
        },
    },
    Look {
        name: "the Free-State rifleman",
        path: None,
        pieces: &["slouch hat", "hickory shirt", "Sharps sling"],
        bonus: Bonus {
            sway: 0.06,
            nerve: 0.03,
            ..NONE
        },
    },
    Look {
        name: "the vagabond",
        path: Some("vagabond"),
        pieces: &["blanket capote", "moccasins"],
        bonus: Bonus {
            stealth: 0.08,
            skills: &[(Skill::Trapping, 0.05)],
            ..NONE
        },
    },
    Look {
        name: "the trapper",
        path: None,
        pieces: &["coonskin cap", "fringed hunting shirt", "powder horn"],
        bonus: Bonus {
            skills: &[(Skill::Trapping, 0.1)],
            stealth: 0.04,
            ..NONE
        },
    },
    Look {
        name: "the man of business",
        path: Some("master of industry"),
        pieces: &["beaver hat", "frock coat", "watch and chain"],
        bonus: Bonus {
            skills: &[(Skill::Trade, 0.1), (Skill::Letters, 0.04)],
            ..NONE
        },
    },
    Look {
        name: "the bounty man",
        path: None,
        pieces: &["linen duster", "revolver belt", "riding boots"],
        bonus: Bonus {
            draw: 0.05,
            nerve: 0.05,
            ..NONE
        },
    },
    Look {
        name: "the homesteader",
        path: Some("pillar of the community"),
        pieces: &["straw hat", "hickory shirt", "brogans"],
        bonus: Bonus {
            skills: &[(Skill::Farming, 0.08), (Skill::Carpentry, 0.05)],
            ..NONE
        },
    },
];

/// One worn piece and, if it was taken, the act it was taken in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Worn {
    pub item: ItemId,
    /// (whose it was, the killing or theft it came out of).
    pub from: Option<(NpcId, EventId)>,
}

impl Worn {
    pub fn new(item: ItemId) -> Self {
        Worn { item, from: None }
    }
}

pub const SLOTS: usize = 6;

/// Hat, coat, shirt, feet, kit, kit.
#[derive(Clone, Debug, Default)]
pub struct Outfit {
    pub on: [Option<Worn>; SLOTS],
    /// 0 new .. 1 rags. Worn cloth is colder and keeps lice.
    pub wear: f32,
}

fn slot_index(s: Slot) -> &'static [usize] {
    match s {
        Slot::Hat => &[0],
        Slot::Coat => &[1],
        Slot::Shirt => &[2],
        Slot::Feet => &[3],
        Slot::Kit => &[4, 5],
    }
}

impl Outfit {
    pub fn pieces(&self) -> impl Iterator<Item = &'static Item> + '_ {
        self.on.iter().flatten().map(|w| item(w.item))
    }

    pub fn has(&self, name: &str) -> bool {
        self.pieces().any(|i| i.name == name)
    }

    /// Put a piece on; returns what came off.
    pub fn put_on(&mut self, w: Worn) -> Option<Worn> {
        let slots = slot_index(item(w.item).slot);
        let at = slots
            .iter()
            .copied()
            .find(|&s| self.on[s].is_none())
            .unwrap_or(slots[0]);
        self.on[at].replace(w)
    }

    pub fn warmth(&self) -> f32 {
        let w: f32 = self.pieces().map(|i| i.warmth).sum();
        w * (1.0 - 0.5 * self.wear)
    }

    /// The side a stranger reads off you, if any: the strongest marks win.
    pub fn colors(&self) -> Option<Faction> {
        let (mut ps, mut fs) = (0, 0);
        for i in self.pieces() {
            match i.mark {
                Some(Faction::ProSlavery) => ps += 1,
                Some(Faction::FreeState) => fs += 1,
                None => {}
            }
        }
        match ps.cmp(&fs) {
            std::cmp::Ordering::Greater => Some(Faction::ProSlavery),
            std::cmp::Ordering::Less => Some(Faction::FreeState),
            _ => None,
        }
    }

    /// The looks worn whole.
    pub fn looks(&self) -> Vec<&'static Look> {
        LOOKS
            .iter()
            .filter(|l| l.pieces.iter().all(|p| self.has(p)))
            .collect()
    }

    /// Pieces and looks together.
    pub fn bonus(&self) -> Total {
        let mut t = Total::default();
        let mut add = |b: &Bonus| {
            for &(s, v) in b.skills {
                t.skills[s.index()] += v;
            }
            t.nerve += b.nerve;
            t.draw += b.draw;
            t.sway += b.sway;
            t.stealth += b.stealth;
            t.charm += b.charm;
        };
        for i in self.pieces() {
            add(&i.bonus);
        }
        for l in self.looks() {
            add(&l.bonus);
        }
        t
    }
}

/// An outfit's bonuses summed.
#[derive(Clone, Copy, Debug, Default)]
pub struct Total {
    pub skills: [f32; super::life::SKILLS],
    pub nerve: f32,
    pub draw: f32,
    pub sway: f32,
    pub stealth: f32,
    pub charm: f32,
}

pub fn of(world: &World, id: NpcId) -> Total {
    world.npc(id).outfit.bonus()
}

// ---- the player's closet --------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct Wardrobe {
    /// What's in your trunk and not on your back.
    pub closet: Vec<Worn>,
    /// A man down in the dirt, today: (who, the act, who's watching, day).
    pub lootable: Option<(NpcId, EventId, Vec<NpcId>, Day)>,
    /// Kin who've seen you in a dead man's clothes, per piece (so a widow
    /// knows the hat once, not every Sunday).
    known: Vec<(NpcId, EventId)>,
}

/// Refresh the skill bonus the life page reads.
fn refresh(world: &mut World) {
    world.life.gear = world.npc(PLAYER).outfit.bonus().skills;
}

pub fn wear(world: &mut World, closet_index: usize) -> bool {
    if closet_index >= world.wardrobe.closet.len() {
        return false;
    }
    let w = world.wardrobe.closet.remove(closet_index);
    if let Some(off) = world.npc_mut(PLAYER).outfit.put_on(w) {
        world.wardrobe.closet.push(off);
    }
    refresh(world);
    true
}

pub fn take_off(world: &mut World, slot: usize) -> bool {
    let Some(w) = world
        .npc_mut(PLAYER)
        .outfit
        .on
        .get_mut(slot)
        .and_then(|s| s.take())
    else {
        return false;
    };
    world.wardrobe.closet.push(w);
    refresh(world);
    true
}

/// What Dunmore has on the shelf for you: priced pieces for your sex.
pub fn for_sale(world: &World) -> Vec<ItemId> {
    let woman = is_woman(world.name(PLAYER));
    (0..ITEMS.len() as ItemId)
        .filter(|&i| {
            let it = item(i);
            it.price > 0 && it.women.is_none_or(|w| w == woman)
        })
        .collect()
}

/// Prices go up a third through a blockade, like everything else.
pub fn price(world: &World, i: ItemId) -> i32 {
    let p = item(i).price;
    if world.market.blockade {
        p + (p + 2) / 3
    } else {
        p
    }
}

pub fn buy(world: &mut World, i: ItemId) -> bool {
    let cost = price(world, i);
    if cost <= 0 || world.families[0].stores.cash < cost || !for_sale(world).contains(&i) {
        return false;
    }
    world.families[0].stores.cash -= cost;
    world.wardrobe.closet.push(Worn::new(i));
    true
}

// ---- dressing the county --------------------------------------------------

/// Everyone gets dressed on the first day, by side, purse and temperament.
/// The roll is hashed from the seed and the person, not drawn from the
/// world's rng, so dressing the county doesn't reshuffle its history.
pub fn founding(world: &mut World) {
    for i in 0..world.npcs.len() {
        let o = dress(world, i as NpcId);
        world.npcs[i].outfit = o;
    }
    refresh(world);
}

pub(crate) fn dress(world: &World, who: NpcId) -> Outfit {
    let n = world.npc(who);
    let woman = is_woman(&n.name) && who != PLAYER;
    let child = LifeStage::of(n.age) == LifeStage::Child;
    let side = n.faction;
    let fam = n.family as usize;
    let rich = world.families[fam].stores.cash > 60 || world.families[fam].store;
    let (pious, temper, social) = (
        n.temperament.piety,
        n.temperament.temper,
        n.temperament.sociability,
    );
    let roll = {
        let mut z = world.seed ^ (who as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 11) as f32 / (1u64 << 53) as f32
    };
    let mut o = Outfit {
        wear: 0.2 + 0.4 * roll,
        ..Default::default()
    };
    let mut on = |name: &str| {
        o.put_on(Worn::new(id(name)));
    };
    if child {
        on(if woman { "sunbonnet" } else { "slouch hat" });
        on(if woman {
            "calico dress"
        } else {
            "linsey shirt"
        });
        on(if roll < 0.5 { "bare feet" } else { "brogans" });
        return o;
    }
    if woman {
        on(if rich && roll < 0.4 {
            "bonnet with ribbons"
        } else {
            "sunbonnet"
        });
        on(if rich && roll < 0.2 {
            "silk dress"
        } else {
            "calico dress"
        });
        on("wool shawl");
        on("brogans");
        return o;
    }
    // Men: a Missourian's colors, a deacon's black, a merchant's frock.
    if side == Faction::ProSlavery && temper > 0.5 && roll < 0.7 {
        on("slouch hat and feather");
        on("red flannel shirt");
        on("Bowie knife");
    } else if pious > 0.7 {
        on("preacher's hat");
        on("black frock");
        on("pocket Bible");
        on("hickory shirt");
    } else if rich {
        on(if social > 0.5 {
            "stovepipe hat"
        } else {
            "beaver hat"
        });
        on("frock coat");
        on("boiled white shirt");
        on("watch and chain");
    } else {
        on(if roll < 0.5 {
            "slouch hat"
        } else {
            "straw hat"
        });
        on("hickory shirt");
        on(if roll < 0.3 {
            "blanket capote"
        } else {
            "sack coat"
        });
    }
    if side == Faction::FreeState && roll > 0.6 {
        on("Sharps sling");
    }
    on(if roll < 0.15 { "moccasins" } else { "brogans" });
    o
}

// ---- loot -----------------------------------------------------------------

/// Take the best of what's on a body (or out of a house), and a few
/// dollars. `act`: the killing, wounding or theft it comes out of: the
/// pieces remember it. Returns what you took.
pub fn loot(world: &mut World, victim: NpcId, act: EventId, eyes: &[NpcId]) -> Vec<&'static str> {
    let mut took = Vec::new();
    let mut pieces: Vec<(usize, Worn)> = world
        .npc(victim)
        .outfit
        .on
        .iter()
        .enumerate()
        .filter_map(|(s, w)| w.map(|w| (s, w)))
        .filter(|(_, w)| item(w.item).price > 0 || item(w.item).dear)
        .collect();
    // The dear things first: a watch before a shirt.
    pieces.sort_by_key(|(_, w)| {
        let it = item(w.item);
        (std::cmp::Reverse(it.dear), std::cmp::Reverse(it.price))
    });
    for (s, w) in pieces.into_iter().take(2) {
        world.npcs[victim as usize].outfit.on[s] = None;
        world.wardrobe.closet.push(Worn {
            item: w.item,
            from: Some((victim, act)),
        });
        took.push(item(w.item).name);
    }
    // His gun, if he had one on him.
    let fam = world.npc(victim).family as usize;
    let a = &mut world.families[fam].stores.arms;
    if a.rifles > 0 {
        a.rifles -= 1;
        world.families[0].stores.arms.rifles += 1;
        took.push("his Sharps");
    } else if a.guns > 0 {
        a.guns -= 1;
        world.families[0].stores.arms.guns += 1;
        took.push("his gun");
    }
    let cash = world.families[fam].stores.cash.clamp(0, 5);
    if cash > 0 {
        world.families[fam].stores.cash -= cash;
        world.families[0].stores.cash += cash;
        took.push("what was in his pockets");
    }
    world.emit_root(
        EventKind::Looted {
            looter: PLAYER,
            victim,
            pieces: took.len() as u8,
            seen: !eyes.is_empty(),
        },
        Some(act),
    );
    // Robbing the fallen is its own sin to those who saw it.
    for &e in eyes {
        if e != PLAYER {
            world.adjust_opinion(e, PLAYER, -20);
        }
    }
    world.run_cascades();
    took
}

/// Someone you just shot is lying there. Until tomorrow, you could go
/// through his pockets.
pub(crate) fn lootable(world: &mut World, victim: NpcId, act: EventId, eyes: Vec<NpcId>) {
    world.wardrobe.lootable = Some((victim, act, eyes, world.day));
}

/// Go through the pockets of the one lying there, if anyone is.
pub fn loot_fallen(world: &mut World) -> Vec<&'static str> {
    match world.wardrobe.lootable.take() {
        Some((v, act, eyes, day)) if day == world.day => loot(world, v, act, &eyes),
        _ => Vec::new(),
    }
}

// ---- the tick -------------------------------------------------------------

/// Cloth wears out; rags let the cold in and the lice. And the dead man's
/// hat on your head in public: his people know it.
pub fn daily(world: &mut World) {
    let today = world.day;
    let winter = today.season() == Season::Winter;
    let wear = if winter { 0.0015 } else { 0.001 };
    for n in world.npcs.iter_mut().filter(|n| n.alive) {
        n.outfit.wear = (n.outfit.wear + wear).min(1.0);
    }
    recognized(world, today);
}

fn recognized(world: &mut World, today: Day) {
    // Only in public: in town, at church, at a bee, being seen.
    if world.npc(PLAYER).alibi != Some(today) || !world.player_alive() {
        return;
    }
    let taken: Vec<(NpcId, EventId)> = world
        .npc(PLAYER)
        .outfit
        .on
        .iter()
        .flatten()
        .filter_map(|w| w.from)
        .collect();
    for (whose, act) in taken {
        let fam = world.npc(whose).family;
        let kin: Vec<NpcId> = world
            .living()
            .filter(|n| {
                n.family == fam && n.id != PLAYER && LifeStage::of(n.age) != LifeStage::Child
            })
            .map(|n| n.id)
            .collect();
        for k in kin {
            if world.wardrobe.known.contains(&(k, act)) {
                continue;
            }
            let eye = 0.15 + 0.3 * world.npc(k).body.alertness;
            if !world.rng.chance(eye) {
                continue;
            }
            world.wardrobe.known.push((k, act));
            world.emit_root(
                EventKind::Belief {
                    holder: k,
                    about: act,
                    blamed: Suspect::Person(PLAYER),
                    confidence: 85,
                    source: Source::Victim,
                    reason: "knew the clothes on your back",
                },
                Some(act),
            );
        }
    }
    world.run_cascades();
}

/// A man left alive and robbed remembers it.
pub fn on_event(world: &mut World, ev: &WorldEvent) {
    if let EventKind::Looted { looter, victim, .. } = ev.kind
        && world.npc(victim).alive
    {
        world.adjust_opinion(victim, looter, -30);
    }
}

/// Everyone's outfit, for panels and the lab: who wears what.
pub fn describe(world: &World, id: NpcId) -> String {
    let o = &world.npc(id).outfit;
    let mut s: Vec<&str> = o.pieces().map(|i| i.name).collect();
    if s.is_empty() {
        s.push("next to nothing");
    }
    let rag = if o.wear > 0.8 {
        ", in rags"
    } else if o.wear > 0.5 {
        ", worn thin"
    } else {
        ""
    };
    format!("{}{rag}", s.join(", "))
}

/// The family whose colors a witness would read off the actor.
pub fn colors_of(world: &World, id: NpcId) -> Option<Faction> {
    world.npc(id).outfit.colors()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_look_is_made_of_real_pieces() {
        for l in LOOKS {
            for p in l.pieces {
                let _ = id(p);
            }
        }
    }

    #[test]
    fn a_whole_look_beats_its_pieces() {
        let mut o = Outfit::default();
        for p in ["preacher's hat", "black frock"] {
            o.put_on(Worn::new(id(p)));
        }
        let partial = o.bonus().skills[Skill::Scripture.index()];
        o.put_on(Worn::new(id("pocket Bible")));
        let whole = o.bonus().skills[Skill::Scripture.index()];
        assert!(o.looks().iter().any(|l| l.name == "the preacher"));
        assert!(whole > partial + 0.06 + 0.09, "{partial} -> {whole}");
    }

    #[test]
    fn red_flannel_reads_missouri() {
        let mut o = Outfit::default();
        o.put_on(Worn::new(id("red flannel shirt")));
        o.put_on(Worn::new(id("Bowie knife")));
        assert_eq!(o.colors(), Some(Faction::ProSlavery));
        o.put_on(Worn::new(id("hickory shirt")));
        assert_eq!(
            o.colors(),
            Some(Faction::ProSlavery),
            "the knife still shows"
        );
    }

    #[test]
    fn two_kit_slots() {
        let mut o = Outfit::default();
        assert!(o.put_on(Worn::new(id("pocket Bible"))).is_none());
        assert!(o.put_on(Worn::new(id("whiskey flask"))).is_none());
        let off = o.put_on(Worn::new(id("spectacles")));
        assert_eq!(off.map(|w| item(w.item).name), Some("pocket Bible"));
    }

    #[test]
    fn rags_are_cold() {
        let mut o = Outfit::default();
        o.put_on(Worn::new(id("buffalo coat")));
        let new = o.warmth();
        o.wear = 1.0;
        assert!(o.warmth() < new * 0.6);
    }

    #[test]
    fn everyone_is_dressed() {
        let w = World::new(5);
        for n in w.living() {
            assert!(n.outfit.on.iter().flatten().count() >= 3, "{}", n.name);
        }
    }

    #[test]
    fn the_widow_knows_the_hat() {
        let mut w = World::new(5);
        w.autopilot_player = false;
        let victim = (1..w.npcs.len() as NpcId)
            .find(|&i| {
                let n = w.npc(i);
                n.family != 0
                    && !is_woman(&n.name)
                    && LifeStage::of(n.age) == LifeStage::Adult
                    && w.living()
                        .filter(|m| {
                            m.family == n.family
                                && m.id != i
                                && LifeStage::of(m.age) == LifeStage::Adult
                        })
                        .count()
                        > 0
                    && n.outfit.pieces().any(|p| p.price > 0)
            })
            .unwrap();
        let death = w.emit_root(
            EventKind::Death {
                victim,
                killer: Some(PLAYER),
            },
            None,
        );
        let took = loot(&mut w, victim, death, &[]);
        assert!(!took.is_empty());
        let got = w.wardrobe.closet.len() - 1;
        wear(&mut w, got);
        let mut knew = false;
        for _ in 0..40 {
            let today = w.day;
            w.npc_mut(PLAYER).alibi = Some(today);
            recognized(&mut w, today);
            if w.events.iter().any(|e| matches!(
                e.kind,
                EventKind::Belief { about, blamed: Suspect::Person(PLAYER), source: Source::Victim, .. } if about == death
            )) {
                knew = true;
                break;
            }
        }
        assert!(knew, "somebody knew that coat");
    }
}
