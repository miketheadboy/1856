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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Retaliation {
    Arson,
    Ambush,
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
