//! World state, tick loop, and the player's verbs.

use std::collections::{HashMap, HashSet, VecDeque};

use super::calendar::{Day, Season};
use super::events::{
    EVENTS_PER_TICK, EventId, EventKind, FireCause, MAX_CASCADE_DEPTH, Source, Suspect, WorldEvent,
};
use super::rng::SimRng;
use super::systems;

pub type NpcId = u32;
pub type FamilyId = u32;

/// The player is NPC 0, a member of family 0.
pub const PLAYER: NpcId = 0;

/// Days to rebuild a burned barn. Until then there is nothing left to burn.
pub const REBUILD_DAYS: u32 = 90;

/// Memories kept per NPC before low-salience ones are evicted (§14.4).
pub const MEMORY_CAPACITY: usize = 32;
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
    pub mood: i32,
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
}

impl Npc {
    pub fn remember(&mut self, memory: MemoryRef) {
        if self.memories.iter().any(|m| m.event == memory.event) {
            return;
        }
        if self.memories.len() >= MEMORY_CAPACITY {
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
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Weather {
    pub storm: bool,
    pub rain: bool,
    /// 0 calm .. 1 gale
    pub wind: f32,
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

const FARM_SLOTS: [(i32, i32); 8] = [
    (5, 5),
    (15, 4),
    (25, 5),
    (4, 15),
    (26, 15),
    (5, 25),
    (15, 26),
    (25, 25),
];

pub fn distance(a: (i32, i32), b: (i32, i32)) -> f32 {
    let (dx, dy) = ((a.0 - b.0) as f32, (a.1 - b.1) as f32);
    (dx * dx + dy * dy).sqrt()
}

impl World {
    pub fn new(seed: u64) -> Self {
        let mut rng = SimRng::new(seed);

        let mut families = vec![Family {
            id: 0,
            surname: "",
            faction: Faction::FreeState,
            farm: (15, 15),
            barn_standing: true,
            barn_burned_on: None,
        }];
        let mut slots = FARM_SLOTS.to_vec();
        for (surname, faction) in FAMILIES {
            let slot = slots.remove(rng.range(0, slots.len() as u32) as usize);
            let jitter = |rng: &mut SimRng| rng.range(0, 5) as i32 - 2;
            let farm = (slot.0 + jitter(&mut rng), slot.1 + jitter(&mut rng));
            families.push(Family {
                id: families.len() as FamilyId,
                surname,
                faction,
                farm,
                barn_standing: true,
                barn_burned_on: None,
            });
        }

        let mut npcs = vec![Npc {
            id: PLAYER,
            name: "You".into(),
            family: 0,
            faction: Faction::FreeState,
            mood: 60,
            alive: true,
            memories: Vec::new(),
            alibi: None,
            plotting: None,
            violence: 0,
            last_revenge: None,
        }];
        let mut given: Vec<&str> = GIVEN_NAMES.to_vec();
        for family in families.iter().skip(1) {
            let size = rng.range(2, 4);
            for _ in 0..size {
                let first = given.remove(rng.range(0, given.len() as u32) as usize);
                npcs.push(Npc {
                    id: npcs.len() as NpcId,
                    name: format!("{} {}", first, family.surname),
                    family: family.id,
                    faction: family.faction,
                    mood: 45 + rng.range(0, 25) as i32,
                    alive: true,
                    memories: Vec::new(),
                    alibi: None,
                    plotting: None,
                    violence: 0,
                    last_revenge: None,
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
        };

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

    pub fn living(&self) -> impl Iterator<Item = &Npc> {
        self.npcs.iter().filter(|n| n.alive)
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
        self.dryness = if rain {
            self.dryness * 0.35
        } else {
            (self.dryness + 0.04).min(1.0)
        };
        self.weather.push(Weather { storm, rain, wind });
    }

    fn rebuild_barns(&mut self) {
        let today = self.day.0;
        for f in &mut self.families {
            if f.barn_burned_on
                .is_some_and(|d| today >= d.0 + REBUILD_DAYS)
            {
                f.barn_standing = true;
                f.barn_burned_on = None;
            }
        }
    }

    /// Some people are seen in town each day. Being seen is an alibi.
    fn roll_alibis(&mut self) {
        let day = self.day;
        for i in 1..self.npcs.len() {
            if self.npcs[i].alive && self.rng.chance(0.12) {
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
            let cause = if weather.storm && self.rng.chance(0.015) {
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
        // Trust is slow to rebuild (§13): grudges soften by 2 a month.
        let keys: Vec<_> = self.relationships.keys().copied().collect();
        for (a, b) in keys {
            let default = self.default_opinion(a, b);
            let value = self.relationships[&(a, b)];
            if value < default {
                self.relationships.insert((a, b), (value + 2).min(default));
            }
        }
        for g in &mut self.grievance {
            *g = (*g - 12).max(0);
        }
        for n in &mut self.npcs {
            if n.alive && n.mood < 60 {
                n.mood += 5;
            }
        }
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

    /// Be seen in town today. Visibility is an alibi (§11.4).
    pub fn player_go_to_tavern(&mut self) {
        let day = self.day;
        self.npc_mut(PLAYER).alibi = Some(day);
    }
}
