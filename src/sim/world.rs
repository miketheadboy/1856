//! World state, tick loop, and the player's verbs.

use std::collections::{HashMap, HashSet, VecDeque};

use super::calendar::{Day, Season};
use super::character::{self, Hidden};
use super::economy::{self, Choice, Household};
use super::events::{
    EVENTS_PER_TICK, EventId, EventKind, FireCause, MAX_CASCADE_DEPTH, Source, Suspect, WorldEvent,
};
use super::geography::{self, Map};
use super::history;
use super::market::{self, Good, Market};
use super::nations::{self, Nation, NationId};
use super::psyche::{self, Body, Emotions, Ideology, Temperament};
use super::rng::SimRng;
use super::systems;

pub type NpcId = u32;
pub type FamilyId = u32;

/// The player is NPC 0, a member of family 0.
pub const PLAYER: NpcId = 0;

/// Days of work to rebuild a burned barn, once there's timber (4 loads).
/// Until then there is nothing left to burn.
pub const REBUILD_DAYS: u32 = 30;

/// Memories kept before low-salience ones are evicted (§14.4), by recall.
pub const MEMORY_CAPACITY: usize = 48;
pub const MEMORY_FLOOR: usize = 16;
/// Memories at or above this weight are identity-forming and never evicted.
pub const STICKY_WEIGHT: u8 = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Faction {
    ProSlavery,
    FreeState,
}

impl Faction {
    pub fn index(self) -> usize {
        match self {
            Faction::ProSlavery => 0,
            Faction::FreeState => 1,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Faction::ProSlavery => "Pro-Slavery",
            Faction::FreeState => "Free-State",
        }
    }
}

/// A reference to a canonical event plus this NPC's private reading of it (§14.3).
#[derive(Clone, Debug)]
pub struct MemoryRef {
    pub event: EventId,
    pub believed: Suspect,
    pub confidence: u8,
    pub source: Source,
    pub day: Day,
    pub weight: u8,
}

#[derive(Clone, Debug)]
pub struct Npc {
    pub id: NpcId,
    pub name: String,
    pub family: FamilyId,
    pub faction: Faction,
    pub alive: bool,
    pub memories: Vec<MemoryRef>,
    /// Day this person was seen in public (tavern, church) — an alibi (§11.4).
    pub alibi: Option<Day>,
    /// Who this person has sworn to hurt, if anyone.
    pub plotting: Option<NpcId>,
    /// Moral momentum: each act of violence makes the next one easier (ext. §17).
    pub violence: u8,
    /// Last day this person acted on a grudge. Revenge takes time to rebuild.
    pub last_revenge: Option<Day>,
    pub age: u8,
    /// 0 dead .. 100 well. Hunger, cold and fever take it; food brings it back.
    pub health: i32,
    pub emotions: Emotions,
    pub temperament: Temperament,
    pub body: Body,
    pub ideology: Ideology,
    /// Shot and survived. Can't work or ride until healed.
    pub wounded: bool,
    /// Luck and malice. The player never sees these.
    pub hidden: Hidden,
    /// Taken into a nation's kin network; gone from settler society.
    pub adopted_by: Option<NationId>,
}

impl Npc {
    /// A sharp memory holds more before the small things fall away.
    pub fn memory_capacity(&self) -> usize {
        MEMORY_FLOOR + ((MEMORY_CAPACITY - MEMORY_FLOOR) as f32 * self.body.recall) as usize
    }

    /// Store a belief. A belief about something already remembered replaces
    /// the old one only if it's held more strongly (a rumor that won).
    pub fn remember(&mut self, memory: MemoryRef) {
        if let Some(old) = self.memories.iter_mut().find(|m| m.event == memory.event) {
            if memory.confidence > old.confidence && memory.believed != old.believed {
                old.believed = memory.believed;
                old.confidence = memory.confidence;
                old.source = memory.source;
                old.day = memory.day;
            }
            return;
        }
        if self.memories.len() >= self.memory_capacity() {
            let victim = self
                .memories
                .iter()
                .enumerate()
                .filter(|(_, m)| m.weight < STICKY_WEIGHT)
                .min_by_key(|(_, m)| (m.weight, m.day))
                .map(|(i, _)| i);
            match victim {
                Some(i) => {
                    self.memories.remove(i);
                }
                None => return,
            }
        }
        self.memories.push(memory);
    }

    pub fn memory_of(&self, event: EventId) -> Option<&MemoryRef> {
        self.memories.iter().find(|m| m.event == event)
    }
}

#[derive(Clone, Debug)]
pub struct Family {
    pub id: FamilyId,
    pub surname: &'static str,
    pub faction: Faction,
    pub farm: (i32, i32),
    pub barn_standing: bool,
    pub barn_burned_on: Option<Day>,
    /// Food, seed, stock and debt (§10).
    pub stores: Household,
    /// The general store: sells food, extends credit, keeps a ledger.
    pub store: bool,
    /// How far to the nearest timber you can cut without trespassing.
    pub timber_miles: f32,
}

impl Family {
    /// Works land and eats from its own stores.
    pub fn farms(&self) -> bool {
        !self.store
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Weather {
    pub storm: bool,
    pub rain: bool,
    /// 0 calm .. 1 gale
    pub wind: f32,
    pub blizzard: bool,
}

#[derive(Clone, Debug)]
struct Scheduled {
    day: Day,
    kind: EventKind,
    caused_by: EventId,
}

pub struct World {
    pub seed: u64,
    pub day: Day,
    pub rng: SimRng,
    pub npcs: Vec<Npc>,
    pub families: Vec<Family>,
    /// Sparse: only relationships that deviate from the default (§14.2).
    relationships: HashMap<(NpcId, NpcId), i16>,
    /// Canonical event log. `EventId` indexes into it.
    pub events: Vec<WorldEvent>,
    pending: VecDeque<EventId>,
    scheduled: Vec<Scheduled>,
    pub truncated_cascades: u32,
    /// Accumulated grievance per faction (indexed by `Faction::index`).
    pub grievance: [i32; 2],
    pub feuds: HashSet<(FamilyId, FamilyId)>,
    /// Weather per day, indexed by `Day.0`.
    pub weather: Vec<Weather>,
    /// 0 wet .. 1 tinder-dry.
    pub dryness: f32,
    /// How hard this winter bites: 0.5 mild .. 1.6 brutal.
    pub winter_severity: f32,
    /// Let the sim make the player's household decisions (headless runs).
    pub autopilot_player: bool,
    /// The storekeeper is waiting on your answer (§10.2).
    pub pending_favor: Option<NpcId>,
    pub market: Market,
    /// The papers, and which history the county has heard.
    pub press: history::Press,
    /// Federal troops keep the peace until this day (negative feedback).
    pub pacified_until: Option<Day>,
    /// After the Panic of 1857, credit is tight.
    pub credit_crunch: bool,
    pub nations: Vec<Nation>,
    pub map: Map,
}

const FAMILIES: [(&str, Faction); 8] = [
    ("Webb", Faction::FreeState),
    ("Mallory", Faction::ProSlavery),
    ("Bell", Faction::FreeState),
    ("Pike", Faction::ProSlavery),
    ("Carter", Faction::FreeState),
    ("Holt", Faction::ProSlavery),
    ("Finch", Faction::FreeState),
    ("Reed", Faction::ProSlavery),
];

/// The general store, pro-slavery, holding everyone's debt.
const STORE: (&str, Faction) = ("Dunmore", Faction::ProSlavery);
const PLAYER_SURNAME: &str = "Ashby";

const GIVEN_NAMES: [&str; 32] = [
    "Jonas",
    "Martha",
    "Cyrus",
    "Elias",
    "Ruth",
    "Samuel",
    "Clara",
    "Thomas",
    "Ada",
    "William",
    "Sarah",
    "Amos",
    "Lydia",
    "Josiah",
    "Hannah",
    "Ezra",
    "Abigail",
    "Silas",
    "Mercy",
    "Levi",
    "Temperance",
    "Caleb",
    "Prudence",
    "Obadiah",
    "Delia",
    "Asa",
    "Hester",
    "Gideon",
    "Nell",
    "Jubal",
    "Cassius",
    "Orpha",
];

pub fn distance(a: (i32, i32), b: (i32, i32)) -> f32 {
    let (dx, dy) = ((a.0 - b.0) as f32, (a.1 - b.1) as f32);
    (dx * dx + dy * dy).sqrt()
}

impl World {
    pub fn new(seed: u64) -> Self {
        Self::with_winter(seed, None)
    }

    /// `winter` forces the winter severity; `None` rolls it from the seed.
    pub fn with_winter(seed: u64, winter: Option<f32>) -> Self {
        let mut rng = SimRng::new(seed);
        let rolled = 0.5 + rng.unit() * 1.1;
        let winter_severity = winter.unwrap_or(rolled);

        let mut families = vec![Family {
            id: 0,
            surname: PLAYER_SURNAME,
            faction: Faction::FreeState,
            farm: geography::to_tile(geography::PLAYER_CLAIM),
            barn_standing: true,
            barn_burned_on: None,
            stores: Household {
                food: 3.0 * 170.0,
                seed: 10,
                acres: 10,
                cattle: 3,
                oxen: 2,
                cash: 15,
                prudence: 0.5,
                ..Default::default()
            },
            store: false,
            timber_miles: 0.0,
        }];
        let mut free_claims = geography::CLAIMS_FREE_STATE.to_vec();
        let mut pro_claims = geography::CLAIMS_PRO_SLAVERY.to_vec();
        for (surname, faction) in FAMILIES {
            let claims = match faction {
                Faction::FreeState => &mut free_claims,
                Faction::ProSlavery => &mut pro_claims,
            };
            let claim = claims.remove(rng.range(0, claims.len() as u32) as usize);
            let farm = geography::to_tile(claim);
            let acres = rng.range(6, 14);
            families.push(Family {
                id: families.len() as FamilyId,
                surname,
                faction,
                farm,
                barn_standing: true,
                barn_burned_on: None,
                stores: Household {
                    seed: acres,
                    acres,
                    cattle: rng.range(1, 7),
                    oxen: rng.range(0, 3),
                    cash: rng.range(0, 25) as i32,
                    prudence: rng.unit(),
                    proud: rng.chance(0.3),
                    impulsive: rng.chance(0.2),
                    ..Default::default()
                },
                store: false,
                timber_miles: 0.0,
            });
        }
        families.push(Family {
            id: families.len() as FamilyId,
            surname: STORE.0,
            faction: STORE.1,
            farm: geography::to_tile(geography::place("Franklin")),
            barn_standing: true,
            barn_burned_on: None,
            stores: Household {
                food: 5000.0,
                cash: 500,
                cattle: 4,
                ..Default::default()
            },
            store: true,
            timber_miles: 0.0,
        });

        let mut npcs = vec![Npc {
            id: PLAYER,
            name: "You".into(),
            family: 0,
            faction: Faction::FreeState,
            alive: true,
            memories: Vec::new(),
            alibi: None,
            plotting: None,
            violence: 0,
            last_revenge: None,
            age: 31,
            health: 100,
            emotions: Emotions::default(),
            temperament: Temperament {
                temper: 0.3,
                courage: 0.5,
                piety: 0.4,
                generosity: 0.5,
                honesty: 0.7,
                sociability: 0.5,
                skepticism: 0.5,
                loyalty: 0.5,
                literacy: 0.9,
            },
            body: Body {
                strength: 0.6,
                hardiness: 0.6,
                marksmanship: 0.5,
                stealth: 0.5,
                alertness: 0.5,
                recall: 1.0,
            },
            ideology: Ideology::for_faction(Faction::FreeState, 0.6),
            wounded: false,
            hidden: Hidden::default(),
            adopted_by: None,
        }];
        let mut given: Vec<&str> = GIVEN_NAMES.to_vec();
        for family in families.iter() {
            // You, a spouse, a child. The child is who hunger takes first.
            let size = match family.id {
                0 => 2,
                _ if family.store => 2,
                _ => rng.range(2, 4),
            };
            for slot in 0..size {
                let first = given.remove(rng.range(0, given.len() as u32) as usize);
                // Two adults (sometimes an elder), then children. Your family: a spouse, a child.
                let adults = if family.id == 0 { 1 } else { 2 };
                let age = if slot < adults {
                    if rng.chance(0.12) {
                        55 + rng.range(0, 15)
                    } else {
                        19 + rng.range(0, 34)
                    }
                } else {
                    3 + rng.range(0, 12)
                } as u8;
                let conviction = 0.2 + rng.unit() * 0.8;
                let mut ideology = Ideology::for_faction(family.faction, conviction);
                // A few people privately don't believe what their family says (ext. §16).
                if rng.chance(0.1) {
                    ideology.private = -ideology.private * 0.5;
                }
                let temperament = Temperament::roll(&mut rng);
                let body = Body::roll(&mut rng, age);
                let hidden = Hidden::roll(&mut rng);
                npcs.push(Npc {
                    id: npcs.len() as NpcId,
                    name: format!("{} {}", first, family.surname),
                    family: family.id,
                    faction: family.faction,
                    alive: true,
                    memories: Vec::new(),
                    alibi: None,
                    plotting: None,
                    violence: 0,
                    last_revenge: None,
                    age,
                    health: 100,
                    emotions: Emotions::default(),
                    temperament,
                    body,
                    ideology,
                    wounded: false,
                    hidden,
                    adopted_by: None,
                });
            }
        }

        let mut world = Self {
            seed,
            day: Day(0),
            rng,
            npcs,
            families,
            relationships: HashMap::new(),
            events: Vec::new(),
            pending: VecDeque::new(),
            scheduled: Vec::new(),
            truncated_cascades: 0,
            grievance: [0; 2],
            feuds: HashSet::new(),
            weather: Vec::new(),
            dryness: 0.3,
            winter_severity,
            autopilot_player: true,
            pending_favor: None,
            market: Market::new(),
            press: history::Press::default(),
            pacified_until: None,
            credit_crunch: false,
            nations: nations::founding(),
            map: Map::county(),
        };
        for f in 0..world.families.len() {
            let farm = world.families[f].farm;
            world.families[f].timber_miles = world.map.miles_to_timber(farm);
        }

        // Winter stores: most families went into 1855 short.
        for f in 1..world.families.len() {
            if world.families[f].store {
                continue;
            }
            let m = world
                .npcs
                .iter()
                .filter(|n| n.family == f as FamilyId)
                .count() as f32;
            world.families[f].stores.food = m * (120.0 + world.rng.range(0, 110) as f32);
        }

        // Old grudges from before the game starts: people mostly blame
        // whoever they already hated (§11.2), so the world needs some hate.
        for _ in 0..14 {
            let a = world.rng.range(1, world.npcs.len() as u32);
            let b = world.rng.range(1, world.npcs.len() as u32);
            if world.npcs[a as usize].faction != world.npcs[b as usize].faction {
                let grudge = -(35 + world.rng.range(0, 30) as i16);
                world.set_opinion(a, b, grudge);
            }
        }

        world.roll_weather();
        world
    }

    // ---- lookups -------------------------------------------------------

    pub fn npc(&self, id: NpcId) -> &Npc {
        &self.npcs[id as usize]
    }

    pub fn npc_mut(&mut self, id: NpcId) -> &mut Npc {
        &mut self.npcs[id as usize]
    }

    pub fn name(&self, id: NpcId) -> &str {
        &self.npcs[id as usize].name
    }

    pub fn family_of(&self, id: NpcId) -> &Family {
        &self.families[self.npc(id).family as usize]
    }

    pub fn farm_of(&self, id: NpcId) -> (i32, i32) {
        self.family_of(id).farm
    }

    pub fn weather_on(&self, day: Day) -> Weather {
        self.weather
            .get(day.0 as usize)
            .copied()
            .unwrap_or_default()
    }

    /// First living member of a family: who a fire is "against".
    pub fn head_of(&self, family: FamilyId) -> Option<NpcId> {
        self.npcs
            .iter()
            .find(|n| n.family == family && n.alive)
            .map(|n| n.id)
    }

    /// Everyone alive and still part of settler society.
    pub fn living(&self) -> impl Iterator<Item = &Npc> {
        self.npcs
            .iter()
            .filter(|n| n.alive && n.adopted_by.is_none())
    }

    pub fn player_alive(&self) -> bool {
        self.npc(PLAYER).alive
    }

    pub fn feud_between(&self, a: FamilyId, b: FamilyId) -> bool {
        self.feuds.contains(&(a.min(b), a.max(b)))
    }

    // ---- relationships (§14.2: default on miss) ------------------------

    pub fn default_opinion(&self, a: NpcId, b: NpcId) -> i16 {
        let (na, nb) = (self.npc(a), self.npc(b));
        if na.family == nb.family {
            50
        } else if na.faction == nb.faction {
            10
        } else {
            -10
        }
    }

    pub fn opinion(&self, a: NpcId, b: NpcId) -> i16 {
        self.relationships
            .get(&(a, b))
            .copied()
            .unwrap_or_else(|| self.default_opinion(a, b))
    }

    pub fn set_opinion(&mut self, a: NpcId, b: NpcId, value: i16) {
        self.relationships.insert((a, b), value.clamp(-100, 100));
    }

    pub fn adjust_opinion(&mut self, a: NpcId, b: NpcId, delta: i16) -> i16 {
        let value = (self.opinion(a, b) + delta).clamp(-100, 100);
        self.relationships.insert((a, b), value);
        value
    }

    pub fn relationship_count(&self) -> usize {
        self.relationships.len()
    }

    // ---- event bus (§15) -----------------------------------------------

    fn push_event(
        &mut self,
        kind: EventKind,
        parent: Option<EventId>,
        caused_by: Option<EventId>,
        cascade_depth: u8,
    ) -> EventId {
        let id = self.events.len() as EventId;
        self.events.push(WorldEvent {
            id,
            day: self.day,
            parent,
            caused_by,
            cascade_depth,
            kind,
        });
        self.pending.push_back(id);
        id
    }

    /// Start a new chain. `caused_by` links it to whatever set it in motion
    /// on an earlier day.
    pub fn emit_root(&mut self, kind: EventKind, caused_by: Option<EventId>) -> EventId {
        self.push_event(kind, None, caused_by, 0)
    }

    /// Emit a same-tick consequence, subject to the cascade depth limit.
    pub fn emit_child(&mut self, parent: &WorldEvent, kind: EventKind) -> Option<EventId> {
        if parent.cascade_depth >= MAX_CASCADE_DEPTH {
            self.truncated_cascades += 1;
            return None;
        }
        Some(self.push_event(kind, Some(parent.id), None, parent.cascade_depth + 1))
    }

    /// Act on `kind` on a later day, as a new root linked back to `caused_by`.
    pub fn schedule(&mut self, delay_days: u32, kind: EventKind, caused_by: EventId) {
        self.scheduled.push(Scheduled {
            day: Day(self.day.0 + delay_days.max(1)),
            kind,
            caused_by,
        });
    }

    /// Process queued events, up to the per-tick budget. Leftovers wait.
    pub fn run_cascades(&mut self) -> usize {
        let mut processed = 0;
        while processed < EVENTS_PER_TICK {
            let Some(id) = self.pending.pop_front() else {
                break;
            };
            let event = self.events[id as usize].clone();
            systems::dispatch(self, &event);
            processed += 1;
        }
        processed
    }

    pub fn pending_events(&self) -> usize {
        self.pending.len()
    }

    // ---- the daily tick (§16) ------------------------------------------

    pub fn advance_day(&mut self) {
        self.day = Day(self.day.0 + 1);
        self.roll_weather();
        self.roll_alibis();
        self.rebuild_barns();
        self.release_scheduled();
        self.natural_fires();
        history::daily(self);
        market::daily(self);
        economy::daily(self);
        psyche::daily(self);
        character::daily_evil(self);
        systems::spread_gossip(self);
        if self.day.is_first_of_month() {
            self.monthly();
        }
        self.run_cascades();
    }

    pub fn run_days(&mut self, days: u32) {
        for _ in 0..days {
            self.advance_day();
        }
    }

    fn roll_weather(&mut self) {
        let (storm_p, rain_p) = match self.day.season() {
            Season::Winter => (0.01, 0.12),
            Season::Spring => (0.12, 0.30),
            Season::Summer => (0.10, 0.18),
            Season::Autumn => (0.04, 0.14),
        };
        let storm = self.rng.chance(storm_p);
        // A storm is not always a soaking one; dry lightning is the dangerous kind.
        let rain = self.rng.chance(rain_p) || (storm && self.rng.chance(0.5));
        let wind = self.rng.unit();
        let blizzard =
            self.day.season() == Season::Winter && self.rng.chance(0.05 * self.winter_severity);
        self.dryness = if rain {
            self.dryness * 0.35
        } else {
            (self.dryness + 0.04).min(1.0)
        };
        self.weather.push(Weather {
            storm,
            rain,
            wind,
            blizzard,
        });
    }

    /// A barn goes back up once there's timber for it and a few weeks' work.
    /// Families who can't buy timber cut their own along the creek: slow, and
    /// the good stands are on someone else's land.
    fn rebuild_barns(&mut self) {
        let today = self.day.0;
        // No cash for timber after a month: the less scrupulous cut it on
        // treaty land (§7.3's ring of scarcity, and someone else's trees).
        let short: Vec<FamilyId> = self
            .families
            .iter()
            .filter(|f| {
                f.farms()
                    && f.barn_burned_on
                        .is_some_and(|d| today == d.0 + REBUILD_DAYS)
                    && f.stores.goods[Good::Timber.index()] < 4.0
            })
            .map(|f| f.id)
            .collect();
        for fid in short {
            let Some(head) = self.head_of(fid) else {
                continue;
            };
            if self
                .rng
                .chance(1.0 - self.npc(head).temperament.honesty * 0.7)
            {
                nations::cut_reserve_timber(self, fid);
            }
        }
        for f in &mut self.families {
            let timber = &mut f.stores.goods[Good::Timber.index()];
            if f.barn_burned_on
                .is_some_and(|d| today == d.0 + REBUILD_DAYS * 2)
                && *timber < 4.0
            {
                *timber = 4.0;
            }
            if f.barn_burned_on
                .is_some_and(|d| today >= d.0 + REBUILD_DAYS)
                && *timber >= 4.0
            {
                *timber -= 4.0;
                f.barn_standing = true;
                f.barn_burned_on = None;
            }
        }
    }

    /// Some people are seen in town each day; the pious are seen at church on
    /// Sundays. Being seen is an alibi.
    fn roll_alibis(&mut self) {
        let day = self.day;
        let sunday = day.0 % 7 == 3;
        for i in 1..self.npcs.len() {
            let n = &self.npcs[i];
            if !n.alive || n.wounded {
                continue;
            }
            let mut p = 0.04 + 0.16 * n.temperament.sociability;
            if sunday {
                p += 0.6 * n.temperament.piety;
            }
            if self.rng.chance(p) {
                self.npcs[i].alibi = Some(day);
            }
        }
    }

    fn release_scheduled(&mut self) {
        let today = self.day;
        let (due, later): (Vec<_>, Vec<_>) = self.scheduled.drain(..).partition(|s| s.day <= today);
        self.scheduled = later;
        for s in due {
            let kind = match s.kind {
                EventKind::Retaliation {
                    actor,
                    target,
                    method,
                } => match systems::resolve_plot(self, actor, target, method) {
                    Some(method) => EventKind::Retaliation {
                        actor,
                        target,
                        method,
                    },
                    None => continue,
                },
                other => other,
            };
            self.emit_root(kind, Some(s.caused_by));
        }
    }

    /// Fires nobody set (§12): lightning, hearths, and spring burns that got away.
    fn natural_fires(&mut self) {
        let weather = self.weather_on(self.day);
        let season = self.day.season();
        let month = self.day.month();
        for f in 1..self.families.len() {
            if !self.families[f].barn_standing {
                continue;
            }
            let Some(head) = self.head_of(f as FamilyId) else {
                continue;
            };
            // Luck is hidden, but lightning knows.
            let luck = self.npc(head).hidden.luck;
            let cause = if weather.storm && self.rng.chance(0.015 / luck) {
                Some(FireCause::Lightning)
            } else if season == Season::Winter && self.rng.chance(0.0015) {
                Some(FireCause::Hearth)
            } else if (month == 3 || month == 4)
                && !weather.rain
                && self.rng.chance(0.004 * (0.5 + self.dryness))
            {
                Some(FireCause::EscapedBurn)
            } else {
                None
            };
            if let Some(cause) = cause {
                self.emit_root(
                    EventKind::Fire {
                        owner: head,
                        cause,
                        spread_from: None,
                    },
                    None,
                );
            }
        }
    }

    fn monthly(&mut self) {
        // Trust is slow to rebuild (§13): grudges soften a little each month,
        // faster for the pious.
        let keys: Vec<_> = self.relationships.keys().copied().collect();
        for (a, b) in keys {
            let default = self.default_opinion(a, b);
            let value = self.relationships[&(a, b)];
            if value < default {
                let n = self.npc(a);
                let deacon = if character::is(n, character::Archetype::Deacon) {
                    2
                } else {
                    1
                };
                let thaw = (2 + (4.0 * n.temperament.piety) as i16) * deacon;
                self.relationships
                    .insert((a, b), (value + thaw).min(default));
            }
        }
        for g in &mut self.grievance {
            *g = (*g - 12).max(0);
        }
        nations::monthly(self);
    }

    // ---- player verbs --------------------------------------------------

    /// Kill someone. The cascade runs immediately.
    pub fn player_kill(&mut self, victim: NpcId) -> Option<EventId> {
        if !self.player_alive() || victim == PLAYER || !self.npc(victim).alive {
            return None;
        }
        let id = self.emit_root(
            EventKind::Death {
                victim,
                killer: Some(PLAYER),
            },
            None,
        );
        self.run_cascades();
        Some(id)
    }

    /// Set fire to someone's barn. Check the wind first (§12).
    pub fn player_burn(&mut self, owner: NpcId) -> Option<EventId> {
        if !self.player_alive() || owner == PLAYER || !self.npc(owner).alive {
            return None;
        }
        self.npc_mut(PLAYER).alibi = None;
        let id = self.emit_root(
            EventKind::Fire {
                owner,
                cause: FireCause::Arson(PLAYER),
                spread_from: None,
            },
            None,
        );
        self.run_cascades();
        Some(id)
    }

    /// A hard choice for your own household (§10). Returns whether it fed anyone.
    pub fn player_choose(&mut self, choice: Choice) -> bool {
        if !self.player_alive() {
            return false;
        }
        let done = economy::act(self, 0, choice);
        self.run_cascades();
        done
    }

    /// Buy at Dunmore's. Buy enough and you move the price (§ market).
    pub fn player_buy(&mut self, good: Good, qty: f32) -> f32 {
        if !self.player_alive() {
            return 0.0;
        }
        let got = market::buy(self, 0, good, qty);
        self.run_cascades();
        got
    }

    /// Sell to Dunmore's at the store's bid.
    pub fn player_sell(&mut self, good: Good, qty: f32) -> i32 {
        if !self.player_alive() {
            return 0;
        }
        let dollars = market::sell(self, 0, good, qty);
        self.run_cascades();
        dollars
    }

    /// Steal from a particular neighbor.
    pub fn player_steal(&mut self, from: NpcId) -> bool {
        if !self.player_alive() || from == PLAYER || !self.npc(from).alive {
            return false;
        }
        let family = self.npc(from).family;
        economy::steal_from(self, PLAYER, family, from);
        self.run_cascades();
        true
    }

    /// Answer the storekeeper. Sign his petition, or lose what he can carry off.
    pub fn player_answer_favor(&mut self, sign: bool) {
        let Some(creditor) = self.pending_favor.take() else {
            return;
        };
        self.emit_root(
            EventKind::Favor {
                creditor,
                debtor: PLAYER,
                complied: sign,
            },
            None,
        );
        self.run_cascades();
    }

    /// Be seen in town today. Visibility is an alibi (§11.4).
    pub fn player_go_to_tavern(&mut self) {
        let day = self.day;
        self.npc_mut(PLAYER).alibi = Some(day);
    }
}
