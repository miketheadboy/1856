//! The event bus (§15). Systems never call each other; they emit events.
//!
//! Every event lives once in the canonical log (§14.3). NPCs hold references
//! to it plus their own belief about its cause, which may be wrong (§11).

use super::calendar::Day;
use super::world::{Faction, FamilyId, NpcId};

pub type EventId = u32;

/// Same-tick chains stop here (§15.3). Consequences that play out on later
/// days start a new root linked by `caused_by`, so a feud can run for months
/// without tripping the limit.
pub const MAX_CASCADE_DEPTH: u8 = 6;

/// Events processed per tick before the rest roll over to the next one.
pub const EVENTS_PER_TICK: usize = 600;

/// What someone thinks caused a harm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Suspect {
    Nature,
    Accident,
    Person(NpcId),
    /// Settler prejudice's favorite answer for lost stock (see `nations`).
    Nation(super::nations::NationId),
}

/// What actually started a fire. All four look identical afterward (§12).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireCause {
    Lightning,
    Hearth,
    EscapedBurn,
    Arson(NpcId),
}

impl FireCause {
    pub fn truth(self) -> Suspect {
        match self {
            FireCause::Lightning => Suspect::Nature,
            FireCause::Hearth | FireCause::EscapedBurn => Suspect::Accident,
            FireCause::Arson(p) => Suspect::Person(p),
        }
    }
}

/// How a belief was acquired.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Saw the culprit with their own eyes.
    Witnessed,
    /// Owns the loss, and worked out a culprit.
    Victim,
    /// Was nearby and drew a conclusion.
    Bystander,
    /// Heard it from someone.
    Told(NpcId),
    /// Read it in the paper. Printed words stick (ext. §14: newspapers freeze a version).
    Newspaper(super::history::Paper),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Retaliation {
    Arson,
    Ambush,
}

/// What a household gave up, or who it turned to, when the food ran out (§10).
/// Every option spends the future to buy the present.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Desperate {
    /// Ate the corn they were holding back to plant.
    EatSeed { bushels: u32 },
    /// Butchered breeding stock. The herd takes years to rebuild.
    Slaughter,
    /// Butchered a plow ox. Without oxen, no new sod gets broken.
    SlaughterOx,
    /// Bought food on credit at the store.
    Borrow { lender: NpcId, dollars: i32 },
    /// Asked a neighbor for food.
    Beg { neighbor: NpcId, granted: bool },
}

/// Harm done for its own sake (see `character`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cruelty {
    /// Tell `listener` that the victim caused `about`. A lie.
    Slander { listener: NpcId, about: EventId },
    /// Shoot a neighbor's cow and leave it.
    KillStock,
    /// Throw a carcass in the well. Everyone who drinks sickens.
    FoulWell,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hardship {
    Hunger,
    Cold,
    Fever,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loot {
    Cow,
    Grain,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EventKind {
    Fire {
        owner: NpcId,
        cause: FireCause,
        spread_from: Option<NpcId>,
    },
    Death {
        victim: NpcId,
        killer: Option<NpcId>,
    },
    Grief {
        mourner: NpcId,
        deceased: NpcId,
    },
    /// Someone formed a belief about who caused an event (§11).
    Belief {
        holder: NpcId,
        about: EventId,
        blamed: Suspect,
        confidence: u8,
        source: Source,
        reason: &'static str,
    },
    OpinionChange {
        holder: NpcId,
        target: NpcId,
        delta: i16,
        /// Filled in when the change is applied.
        after: i16,
    },
    Gossip {
        teller: NpcId,
        listener: NpcId,
        about: EventId,
        blamed: Suspect,
    },
    FactionGrievance {
        faction: Faction,
        level: i32,
        authorized: bool,
    },
    FeudDeclared {
        a: FamilyId,
        b: FamilyId,
    },
    /// Died of the substrate, not of a person: hunger, cold, or fever.
    Perished {
        victim: NpcId,
        cause: Hardship,
    },
    Desperation {
        family: FamilyId,
        act: Desperate,
    },
    /// Hunger's crime. Victims attribute it like any other harm (§11).
    Theft {
        thief: NpcId,
        victim: NpcId,
        loot: Loot,
    },
    Cruelty {
        actor: NpcId,
        victim: NpcId,
        act: Cruelty,
    },
    /// Shot and survived.
    Wounded {
        victim: NpcId,
        attacker: NpcId,
    },
    Harvest {
        family: FamilyId,
        planted: u32,
        food: u32,
    },
    /// The storekeeper calls in a debt as a political favor (§10.2).
    Favor {
        creditor: NpcId,
        debtor: NpcId,
        complied: bool,
    },
    /// Something happened out in the Territory or back in the States.
    History {
        index: usize,
    },
    /// A paper went to press with its version of the week.
    Headline {
        paper: super::history::Paper,
        about: Option<EventId>,
        blamed: Option<Suspect>,
        history: Option<usize>,
    },
    /// Timber cut on treaty land.
    Trespass {
        family: FamilyId,
        nation: super::nations::NationId,
    },
    Annuity {
        nation: super::nations::NationId,
        paid: u32,
        docked: u32,
    },
    /// A nation protests to its agent. Washington rarely acts.
    Complaint {
        nation: super::nations::NationId,
        acted: bool,
    },
    /// Riders from one side or the other took what they wanted on reserve land.
    Plundered {
        nation: super::nations::NationId,
    },
    /// A hungry family at a settler's door.
    Visit {
        nation: super::nations::NationId,
        host: NpcId,
        fed: bool,
    },
    /// Taken in by council.
    Adopted {
        person: NpcId,
        nation: super::nations::NationId,
    },
    /// The Kaw come back from the fall buffalo hunt.
    KawHunt {
        good: bool,
    },
    /// A settler back from weeks on the range west.
    BuffaloHunt {
        hunter: NpcId,
        animals: u32,
    },
    /// A fight at a gathering place.
    Brawl {
        venue: super::institutions::Venue,
        a: NpcId,
        b: NpcId,
    },
    /// Someone sold silence, or couldn't buy it.
    Blackmail {
        victim: NpcId,
        extorter: NpcId,
        paid: bool,
    },
    /// Kin of the dead swears on the grave.
    OathSworn {
        holder: NpcId,
        target: NpcId,
        over: NpcId,
    },
    /// The oath passes to the next of kin.
    OathInherited {
        heir: NpcId,
        target: NpcId,
    },
    /// A child who swore comes of age.
    OathWakes {
        holder: NpcId,
        target: NpcId,
    },
    /// Kin who believe the dead had it coming: no oath, only shame.
    ShameCarried {
        holder: NpcId,
        over: NpcId,
    },
    /// A child who carried shame comes of age, and it turns their life.
    ShameWakes {
        holder: NpcId,
        over: NpcId,
        turn: super::ghosts::ShameTurn,
    },
    /// Someone saw the dead, out by the place it happened.
    SpiritSeen {
        witness: NpcId,
        spirit: NpcId,
    },
    /// The truth is known, or the killer is dead. The haunting stops.
    SpiritRests {
        spirit: NpcId,
    },
    /// A plotter had the target in reach and let it go (§13).
    Spared {
        actor: NpcId,
        target: NpcId,
        why: super::reconcile::Mercy,
    },
    /// Neighbors raise a burned barn in a day. A rival among them counts most.
    BarnRaising {
        owner: NpcId,
        hands: u8,
        rival: Option<NpcId>,
    },
    /// The humble own what they did, to the family they did it to.
    Apology {
        actor: NpcId,
        to: NpcId,
    },
    /// An enemy came to a child's burying.
    Condolence {
        visitor: NpcId,
        mourner: NpcId,
    },
    /// A feud is over, for now.
    FeudEnded {
        a: FamilyId,
        b: FamilyId,
        how: super::reconcile::Peace,
    },
    /// Something happened at home while the player was off somewhere.
    WhereWereYou {
        kin: NpcId,
        missed: EventId,
        errand: super::family::Errand,
    },
    /// The player's day: chores, the creek, the woods, a porch, a glass.
    Pastime {
        what: super::life::Pastime,
        amount: u16,
    },
    Sermon {
        preacher: NpcId,
        flock: u8,
    },
    Baptism {
        preacher: NpcId,
        convert: NpcId,
        cold: bool,
    },
    Speech {
        speaker: NpcId,
        calm: bool,
        heard: u8,
    },
    /// A secret. Only the omniscient chronicle sees it.
    Affair {
        a: NpcId,
        b: NpcId,
    },
    /// Somebody saw.
    Scandal {
        a: NpcId,
        b: NpcId,
        wronged: NpcId,
    },
    Marriage {
        a: NpcId,
        b: NpcId,
    },
    Built {
        project: super::civic::Project,
    },
    ClaimFiled {
        family: FamilyId,
        /// The register "lost" the papers.
        delayed: bool,
    },
    ClaimJumped {
        family: FamilyId,
        lost: u8,
        /// A neighbor moved the stakes, or a stranger if None.
        neighbor: Option<NpcId>,
    },
    Festival {
        holiday: super::civic::Holiday,
        crowd: u8,
    },
    /// A cow got through bad fence and never came home. The owner decides why.
    Strayed {
        owner: NpcId,
    },
    /// Not enough hay put up: stock died on the winter prairie.
    StockStarved {
        family: FamilyId,
        head: u8,
    },
    /// The mail came up from Westport.
    Letter {
        family: FamilyId,
        letter: super::mail::Letter,
        /// A neighbor who read it to them, if nobody at home could.
        reader: Option<NpcId>,
    },
    /// Kin from back east arrive.
    KinArrived {
        family: FamilyId,
        newcomer: Option<NpcId>,
    },
    /// A case before the justice of the peace.
    Lawsuit {
        plaintiff: NpcId,
        defendant: NpcId,
        judge: NpcId,
        acres: u8,
        won: bool,
    },
    /// The count at this precinct.
    Election {
        index: u8,
        free_state: u16,
        pro_slavery: u16,
        missourians: u16,
    },
    /// A vote bought at the store, found out.
    VoteSold {
        seller: NpcId,
    },
    /// A muster closes: who went, who stayed home, who came back hurt.
    Muster {
        index: u8,
        joined: u8,
        dodged: u8,
        wounded: u8,
        /// The player rode.
        you: bool,
    },
    /// A paper writes you up.
    Notice {
        paper: super::history::Paper,
        about: EventId,
        praise: bool,
    },
    /// A child of yours turns fifteen, and turns out like you or against you.
    Legacy {
        child: NpcId,
        path: &'static str,
        rebelled: bool,
    },
    BeeHeld {
        bee: super::bees::Bee,
        host: FamilyId,
        crowd: u8,
    },
    /// The union meeting divides, North and South.
    ChurchSplit,
    /// A big move in a price at the store.
    PriceMove {
        good: super::market::Good,
        cents: u32,
        rising: bool,
    },
    /// Someone decided to act on a belief. Resolved by the retaliation system.
    Retaliation {
        actor: NpcId,
        target: NpcId,
        method: Retaliation,
    },
}

#[derive(Clone, Debug)]
pub struct WorldEvent {
    pub id: EventId,
    pub day: Day,
    /// Same-tick parent: this event was emitted while processing `parent`.
    pub parent: Option<EventId>,
    /// Earlier event that set this one in motion on a later day.
    pub caused_by: Option<EventId>,
    pub cascade_depth: u8,
    pub kind: EventKind,
}

impl WorldEvent {
    /// Parent for display and ancestry purposes, whichever link exists.
    pub fn origin(&self) -> Option<EventId> {
        self.parent.or(self.caused_by)
    }
}
