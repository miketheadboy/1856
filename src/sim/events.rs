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
    /// Pull the rails at night; the stock wanders.
    CutFence,
    /// Wet the haystack: it rots before February.
    SpoilHay,
    /// A blanket out of a lousy house, given as a kindness.
    FouledBlanket,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hardship {
    Hunger,
    Cold,
    Fever,
    /// A named sickness (`sickness`).
    Sickness(super::sickness::Disease),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loot {
    Cow,
    Grain,
    /// Out of the trunks: Sunday clothes, a watch, what was in the jar.
    Goods,
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
        /// Men who rode.
        joined: u8,
        dodged: u8,
        wounded: u8,
        /// The player rode.
        you: bool,
        /// Dead men's names on the roll: the roll the county sees is
        /// `joined + padded`.
        padded: u8,
    },
    /// Hitched up for the river: Westport or the Lane Trail, with this many
    /// neighbors' orders aboard (`freight`).
    WagonOut {
        teamster: NpcId,
        route_west: bool,
        orders: u8,
    },
    /// Home from the river with the load.
    WagonBack {
        teamster: NpcId,
        /// From Westport (and the river's cholera), or down the Lane Trail
        /// (and maybe a crate of rifles).
        route_west: bool,
        tenths_of_a_ton: u8,
    },
    /// Missourians on the Westport road took a Free-State wagon's load.
    WagonStopped {
        teamster: NpcId,
        seized: u16,
    },
    /// The burying of one dead of a catching sickness: friends sat up with
    /// the body and some carried it home (`sickness`). Friends who kept away
    /// for fear of it are remembered for keeping away.
    Wake {
        dead: NpcId,
        came: u8,
        stayed_away: u8,
    },
    /// A neighbor's order came back light: the teamster kept some.
    ShortWeight {
        teamster: NpcId,
        noticed_by: NpcId,
    },
    /// A letter from back home: the bigamist's first spouse is alive. The
    /// marriage here is undone.
    Bigamy {
        bigamist: NpcId,
        spouse: NpcId,
    },
    /// A captain kept a dead man's name on the muster roll: his pay and
    /// rations, and a stronger company on paper (the rolls of 1855–56 were
    /// padded; the 1857 Oxford returns were copied out of a Cincinnati
    /// directory). Known only to the captain until someone reads the roll.
    RollPadded {
        name: NpcId,
        index: u8,
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
    Born {
        child: NpcId,
        mother: NpcId,
    },
    /// A broken family sells its claim and goes back to the States.
    ClaimBought {
        family: FamilyId,
        seller: NpcId,
        price: u16,
    },
    /// Someone escaping slavery knocks, on a dark night.
    SeekerAtDoor {
        seeker: u16,
        family: FamilyId,
    },
    Sheltered {
        seeker: u16,
        family: FamilyId,
    },
    TurnedAway {
        seeker: u16,
        family: FamilyId,
    },
    /// Slave catchers ride through asking questions.
    Pursuers {
        seeker: u16,
    },
    /// Taken back. `at` is the house they were found in; `informer` who talked.
    Captured {
        seeker: u16,
        at: Option<FamilyId>,
        informer: Option<NpcId>,
    },
    /// Reached free soil to the north.
    Freedom {
        seeker: u16,
    },
    /// Harboring a fugitive under the 1855 slave code.
    Charged {
        accused: NpcId,
        convicted: bool,
        fine: u16,
    },
    /// Somebody bought the store out. Everyone hears who.
    Cornered {
        buyer: NpcId,
        good: super::market::Good,
        units: u16,
    },
    /// A household finished building something.
    Improved {
        family: FamilyId,
        what: super::homestead::Improvement,
    },
    /// A timber stand cut to the last load.
    StandCut {
        family: FamilyId,
        tile: (i32, i32),
    },
    /// A big move in a price at the store.
    PriceMove {
        good: super::market::Good,
        cents: u32,
        rising: bool,
    },
    /// Riders at your gate after dark. What happens next is yours to answer.
    RidersAtGate {
        actor: NpcId,
        target: NpcId,
        method: Retaliation,
    },
    /// How a standoff with `other` ended. `yours`: you rode to them.
    /// `law`: somebody came, or went, with a warrant.
    Standoff {
        other: NpcId,
        end: super::action::End,
        yours: bool,
        law: bool,
    },
    /// Someone on a place in the dark. `seen`: how many saw a face.
    Prowler {
        prowler: NpcId,
        victim: NpcId,
        seen: u8,
    },
    /// A shot from the dark that missed. Still an attempt on a man's life.
    ShotAt {
        shooter: NpcId,
        target: NpcId,
    },
    /// A crate of "books" came in on the Wednesday hack.
    ArmsArrived {
        family: FamilyId,
        rifles: u8,
    },
    /// The river towns opened the crate first.
    Intercepted {
        family: FamilyId,
        rifles: u8,
    },
    /// The sheriff's posse turned the house over. `seized`: rifles taken.
    Searched {
        family: FamilyId,
        seized: u8,
    },
    /// Someone on watch saw riders coming and called out; they turned back.
    /// No `watchman`: hired guns were sitting on the place.
    TurnedBack {
        rider: NpcId,
        target: NpcId,
        watchman: Option<NpcId>,
    },
    /// Taken on by the month.
    HandHired {
        hand: NpcId,
        family: FamilyId,
    },
    /// Walked off, or let go. `unpaid`: there was no money on the first.
    HandQuit {
        hand: NpcId,
        family: FamilyId,
        unpaid: bool,
    },
    /// A hand talked in town about what he's seen in the house.
    HandTalked {
        hand: NpcId,
        family: FamilyId,
    },
    /// A company paid to sit on a place, or to ride on one. `printed`: the
    /// other side's paper found out who paid.
    Hirelings {
        company: u8,
        payer: NpcId,
        target: Option<NpcId>,
        printed: bool,
    },
    /// The justice wrote a paper on a man for a violent act people swear to.
    Warrant {
        accused: NpcId,
        about: EventId,
        bounty: u16,
    },
    /// Riders out to serve a warrant. `found`: the man was there to be found.
    PosseOut {
        accused: NpcId,
        leader: NpcId,
        men: u8,
        found: bool,
    },
    /// Taken in on a warrant.
    Arrested {
        accused: NpcId,
        by: NpcId,
    },
    /// Before the justice. `days` in the Lecompton jail if convicted.
    Tried {
        accused: NpcId,
        judge: NpcId,
        convicted: bool,
        days: u16,
        fine: u16,
    },
    /// Lit out ahead of the law.
    Fled {
        accused: NpcId,
        days: u16,
    },
    /// The price on a man's head, paid out.
    BountyPaid {
        hunter: NpcId,
        accused: NpcId,
        dollars: u16,
        dead: bool,
    },
    /// Came down sick.
    FellSick {
        who: NpcId,
        disease: super::sickness::Disease,
    },
    /// Got up again.
    Recovered {
        who: NpcId,
        disease: super::sickness::Disease,
    },
    /// Word of an epidemic up the river.
    Epidemic {
        disease: super::sickness::Disease,
    },
    /// The doctor rode out.
    DoctorCalled {
        family: FamilyId,
    },
    /// Something given, kindly or not.
    Gift {
        from: NpcId,
        to: NpcId,
    },
    /// Went through a fallen man's pockets, or a house's trunks. `seen`:
    /// somebody watched.
    Looted {
        looter: NpcId,
        victim: NpcId,
        pieces: u8,
        seen: bool,
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
