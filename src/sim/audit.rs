//! The auditor: every system keeps its own books, and the books have to
//! agree. A dead man on the payroll, a jailed man on a posse, a coat worn in
//! a slot it doesn't fit, a death with no event behind it: each system alone
//! looks fine, and only laid side by side do they show the bug (rule 6: if
//! emergence can't be told from a bug, add a lens).
//!
//! `check` is read-only and cheap enough to run every day of a test; the
//! cross-system tests in `tests.rs` and `lab audit` run it across seeds.

use super::events::{EventKind, MAX_CASCADE_DEPTH, Suspect};
use super::wardrobe::{self, ITEMS};
use super::warrant::{Held, State};
use super::world::{NpcId, World};

/// One place where two systems disagree.
#[derive(Clone, Debug, PartialEq)]
pub struct Breach {
    /// The rule that was broken, short and stable (for grouping in the lab).
    pub rule: &'static str,
    pub detail: String,
}

impl std::fmt::Display for Breach {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.rule, self.detail)
    }
}

struct Books<'a> {
    world: &'a World,
    out: Vec<Breach>,
}

impl Books<'_> {
    fn breach(&mut self, rule: &'static str, detail: String) {
        self.out.push(Breach { rule, detail });
    }

    fn who(&self, id: NpcId) -> String {
        match self.world.npcs.get(id as usize) {
            Some(n) => format!("{} (#{id})", n.name),
            None => format!("#{id}"),
        }
    }

    fn exists(&mut self, rule: &'static str, id: NpcId) -> bool {
        if (id as usize) < self.world.npcs.len() {
            true
        } else {
            self.breach(rule, format!("no such person #{id}"));
            false
        }
    }

    /// In the county and able to act: alive, not gone to the States, not
    /// taken in by a nation.
    fn about(&self, id: NpcId) -> bool {
        let n = self.world.npc(id);
        n.alive && !n.departed && n.adopted_by.is_none()
    }

    fn held(&self, id: NpcId) -> bool {
        self.world.warrants.held.iter().any(|h| h.0 == id)
    }
}

/// Everything the systems' books disagree on today. Empty is healthy.
pub fn check(world: &World) -> Vec<Breach> {
    let mut b = Books {
        world,
        out: Vec::new(),
    };
    events(&mut b);
    deaths(&mut b);
    people(&mut b);
    households(&mut b);
    hands(&mut b);
    warrants(&mut b);
    sickness(&mut b);
    wardrobe_(&mut b);
    hearts(&mut b);
    feuds(&mut b);
    b.out
}

/// The event log is the county's memory; everything else points into it.
fn events(b: &mut Books) {
    let w = b.world;
    for (i, e) in w.events.iter().enumerate() {
        if e.id as usize != i {
            b.breach("event-id", format!("event at {i} has id {}", e.id));
        }
        if e.day > w.day {
            b.breach("event-future", format!("#{} is dated after today", e.id));
        }
        if e.cascade_depth > MAX_CASCADE_DEPTH {
            b.breach(
                "cascade-depth",
                format!("#{} at depth {}", e.id, e.cascade_depth),
            );
        }
        for link in [e.parent, e.caused_by].into_iter().flatten() {
            if link >= e.id {
                b.breach(
                    "event-cause",
                    format!("#{} is caused by #{link}, which is not earlier", e.id),
                );
            }
        }
        if let Some(p) = e.parent
            && let Some(parent) = w.events.get(p as usize)
            && parent.day != e.day
        {
            // Same-tick chains only; later days go through `schedule`.
            b.breach(
                "child-same-day",
                format!(
                    "#{} (day {}) has parent #{p} from day {}",
                    e.id, e.day.0, parent.day.0
                ),
            );
        }
    }
}

/// The death system and every other system that kills must agree: a man is
/// dead exactly when one death stands in the log for him.
fn deaths(b: &mut Books) {
    let w = b.world;
    let mut deaths = vec![0u32; w.npcs.len()];
    for e in &w.events {
        if let EventKind::Death { victim, .. } | EventKind::Perished { victim, .. } = e.kind
            && b.exists("death-victim", victim)
        {
            deaths[victim as usize] += 1;
        }
    }
    for n in &w.npcs {
        let d = deaths[n.id as usize];
        if d > 1 {
            b.breach("died-twice", format!("{} has {d} deaths", b.who(n.id)));
        }
        if n.alive && d > 0 {
            b.breach("dead-walking", format!("{} died but is alive", b.who(n.id)));
        }
        if !n.alive && d == 0 {
            b.breach(
                "dead-unrecorded",
                format!("{} is dead with no death", b.who(n.id)),
            );
        }
    }
}

fn people(b: &mut Books) {
    let w = b.world;
    for n in &w.npcs {
        if n.id as usize >= w.npcs.len() || w.npcs[n.id as usize].id != n.id {
            b.breach("npc-id", format!("{} is out of place", n.name));
        }
        if n.family as usize >= w.families.len() {
            b.breach(
                "npc-family",
                format!("{} in family {}", b.who(n.id), n.family),
            );
        }
        let e = &n.emotions;
        for (name, v) in [
            ("fear", e.fear),
            ("anger", e.anger),
            ("grief", e.grief),
            ("zeal", e.zeal),
        ] {
            if !(0.0..=100.0).contains(&v) {
                b.breach("emotion-range", format!("{} {name} {v}", b.who(n.id)));
            }
        }
        if !n.alive && n.plotting.is_some() {
            b.breach(
                "dead-plotting",
                format!("{} plots from the grave", b.who(n.id)),
            );
        }
    }
}

fn households(b: &mut Books) {
    let w = b.world;
    for f in &w.families {
        let s = &f.stores;
        let figures = [("food", s.food), ("timber", f.timber_miles)];
        for (name, v) in figures {
            if !v.is_finite() || v < 0.0 {
                b.breach(
                    "household-range",
                    format!("the {}s' {name} is {v}", f.surname),
                );
            }
        }
        if s.debt < 0 {
            b.breach(
                "household-range",
                format!("the {}s owe {}", f.surname, s.debt),
            );
        }
        if let Some(c) = s.creditor {
            b.exists("creditor", c);
        }
        for (g, v) in s.goods.iter().enumerate() {
            if !v.is_finite() || *v < -0.001 {
                b.breach(
                    "household-goods",
                    format!("the {}s hold {v} of good {g}", f.surname),
                );
            }
        }
    }
    for (i, g) in w.grievance.iter().enumerate() {
        if *g < 0 {
            b.breach("grievance-range", format!("faction {i} grievance {g}"));
        }
    }
}

/// Hands (Phase D) against death, jail, family and the payroll.
fn hands(b: &mut Books) {
    let w = b.world;
    if w.hands.hired.len() > super::hands::MAX_HANDS {
        b.breach("hands-cap", format!("{} hired", w.hands.hired.len()));
    }
    let mut seen: Vec<NpcId> = Vec::new();
    for h in &w.hands.hired {
        if !b.exists("hand", h.npc) {
            continue;
        }
        if seen.contains(&h.npc) {
            b.breach("hand-twice", format!("{} hired twice", b.who(h.npc)));
        }
        seen.push(h.npc);
        if !b.about(h.npc) {
            b.breach(
                "hand-absent",
                format!("{} is hired but not here", b.who(h.npc)),
            );
        }
        if b.held(h.npc) {
            b.breach(
                "hand-held",
                format!("{} is hired and in jail", b.who(h.npc)),
            );
        }
        if w.npc(h.npc).family == 0 {
            b.breach("hand-kin", format!("{} is kin, not a hand", b.who(h.npc)));
        }
    }
    let mut tasked: Vec<NpcId> = Vec::new();
    for &(id, _) in &w.hands.tasks {
        if !b.exists("task", id) {
            continue;
        }
        if tasked.contains(&id) {
            b.breach("task-twice", format!("{} has two jobs", b.who(id)));
        }
        tasked.push(id);
        let hired = w.hands.hired.iter().any(|h| h.npc == id);
        if w.npc(id).family != 0 && !hired {
            b.breach(
                "task-stranger",
                format!("{} works your place unhired", b.who(id)),
            );
        }
    }
}

/// The justice's papers against the jail, the graveyard and the posse.
fn warrants(b: &mut Books) {
    let w = b.world;
    for (i, p) in w.warrants.list.iter().enumerate() {
        if !b.exists("warrant", p.accused) {
            continue;
        }
        if w.events.get(p.about as usize).is_none() {
            b.breach(
                "warrant-about",
                format!("paper {i} on missing event #{}", p.about),
            );
        }
        if p.issued > w.day {
            b.breach("warrant-future", format!("paper {i} issued after today"));
        }
        if p.state == State::Open && !w.npc(p.accused).alive {
            b.breach(
                "warrant-dead",
                format!("paper {i} still open on the dead {}", b.who(p.accused)),
            );
        }
        // The law against attribution: the justice writes only what someone
        // swore to believe. A paper nobody believed is the law inventing.
        let sworn = w.events.iter().any(|e| {
            e.day <= p.issued
                && matches!(e.kind, EventKind::Belief { about, blamed: Suspect::Person(x), .. }
                    if about == p.about && x == p.accused)
        });
        if !sworn {
            b.breach(
                "paper-unsworn",
                format!("paper {i} on {} that nobody believed", b.who(p.accused)),
            );
        }
        if let Some(h) = p.hunter
            && b.exists("hunter", h)
            && h == p.accused
        {
            b.breach("warrant-self", format!("{} hunts himself", b.who(h)));
        }
    }
    let mut held: Vec<NpcId> = Vec::new();
    for &(id, _, how) in &w.warrants.held {
        if !b.exists("held", id) {
            continue;
        }
        if held.contains(&id) {
            b.breach("held-twice", format!("{} held twice", b.who(id)));
        }
        held.push(id);
        if how == Held::Jail && !w.npc(id).alive {
            b.breach(
                "held-dead",
                format!("{} is dead in the jail books", b.who(id)),
            );
        }
    }
    for &(i, rider) in &w.warrants.posse_riders {
        if i >= w.warrants.list.len() {
            b.breach("posse-paper", format!("rider on missing paper {i}"));
        } else if b.exists("posse", rider) && b.held(rider) {
            b.breach(
                "posse-held",
                format!("{} rides a posse from jail", b.who(rider)),
            );
        }
    }
}

/// Sickness against death and immunity.
fn sickness(b: &mut Books) {
    let w = b.world;
    let mut seen: Vec<(NpcId, u8)> = Vec::new();
    for c in &w.sickness.cases {
        if !b.exists("case", c.who) {
            continue;
        }
        let key = (c.who, c.disease as u8);
        if seen.contains(&key) {
            b.breach(
                "case-twice",
                format!("{} has {:?} twice", b.who(c.who), c.disease),
            );
        }
        seen.push(key);
        if !w.npc(c.who).alive {
            b.breach(
                "case-dead",
                format!("{} is dead and still sick", b.who(c.who)),
            );
        }
        if c.since > w.day {
            b.breach(
                "case-future",
                format!("{} took sick after today", b.who(c.who)),
            );
        }
        // Care multiplies the risk: nursing brings it down, the lancet up.
        if !(0.05..=2.0).contains(&c.care) {
            b.breach("case-care", format!("{} care {}", b.who(c.who), c.care));
        }
    }
    for list in [&w.sickness.lousy, &w.sickness.quarantine] {
        for &f in list {
            if f as usize >= w.families.len() {
                b.breach("sick-family", format!("no family {f}"));
            }
        }
    }
}

/// Clothes against their slots, their catalogue and where they came from.
fn wardrobe_(b: &mut Books) {
    let w = b.world;
    let check = |b: &mut Books, owner: String, worn: &wardrobe::Worn| {
        let Some(item) = ITEMS.get(worn.item as usize) else {
            b.breach("item-id", format!("{owner} wears item {}", worn.item));
            return;
        };
        if let Some((from, ev)) = worn.from {
            b.exists("item-from", from);
            if w.events.get(ev as usize).is_none() {
                b.breach(
                    "item-from",
                    format!("{owner}'s {} from missing #{ev}", item.name),
                );
            }
        }
    };
    for n in &w.npcs {
        let owner = b.who(n.id);
        for (slot, worn) in n.outfit.on.iter().enumerate() {
            let Some(worn) = worn else { continue };
            check(b, owner.clone(), worn);
            if let Some(item) = ITEMS.get(worn.item as usize)
                && !wardrobe::slot_index(item.slot).contains(&slot)
            {
                b.breach(
                    "item-slot",
                    format!("{owner} wears {} in slot {slot}", item.name),
                );
            }
        }
        if !(0.0..=1.0).contains(&n.outfit.wear) {
            b.breach("outfit-wear", format!("{owner} wear {}", n.outfit.wear));
        }
    }
    for worn in &w.wardrobe.closet {
        check(b, "the closet".into(), worn);
    }
    if let Some((fallen, _, _, day)) = &w.wardrobe.lootable
        && b.exists("lootable", *fallen)
        && *day > w.day
    {
        b.breach("lootable", "a body to loot from tomorrow".into());
    }
}

/// Marriages against death and each other: no one is married twice over.
fn hearts(b: &mut Books) {
    let w = b.world;
    let mut wed: Vec<NpcId> = Vec::new();
    for &(x, y) in &w.hearts.couples {
        if !b.exists("couple", x) || !b.exists("couple", y) {
            continue;
        }
        if x == y {
            b.breach("couple-self", format!("{} wed to himself", b.who(x)));
        }
        if !(w.npc(x).alive && w.npc(y).alive) {
            continue;
        }
        for id in [x, y] {
            if wed.contains(&id) {
                b.breach("wed-twice", format!("{} has two living spouses", b.who(id)));
            }
            wed.push(id);
        }
    }
}

fn feuds(b: &mut Books) {
    let w = b.world;
    let mut list: Vec<_> = w.feuds.iter().copied().collect();
    list.sort_unstable();
    for (a, c) in list {
        if a == c {
            b.breach("feud-self", format!("family {a} feuds with itself"));
        }
        if a as usize >= w.families.len() || c as usize >= w.families.len() {
            b.breach("feud-family", format!("feud {a}-{c} names no family"));
        }
    }
}

/// The breaches grouped by rule with a count and the first example, for
/// the lab.
pub fn summary(breaches: &[Breach]) -> Vec<(&'static str, usize, String)> {
    let mut out: Vec<(&'static str, usize, String)> = Vec::new();
    for br in breaches {
        match out.iter_mut().find(|r| r.0 == br.rule) {
            Some(r) => r.1 += 1,
            None => out.push((br.rule, 1, br.detail.clone())),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::world::PLAYER;

    #[test]
    fn a_fresh_county_is_in_order() {
        for seed in 0..5 {
            let w = World::new(seed);
            let br = check(&w);
            assert!(br.is_empty(), "seed {seed}: {:?}", br);
        }
    }

    #[test]
    fn the_auditor_catches_a_dead_man_on_the_payroll() {
        let mut w = World::new(3);
        w.run_days(5);
        let victim = w.living().find(|n| n.family != 0).unwrap().id;
        w.npc_mut(victim).alive = false;
        let rules: Vec<_> = check(&w).into_iter().map(|b| b.rule).collect();
        assert!(rules.contains(&"dead-unrecorded"), "{rules:?}");
    }

    #[test]
    fn the_auditor_catches_a_coat_on_a_head() {
        let mut w = World::new(3);
        let coat = ITEMS
            .iter()
            .position(|i| i.slot == wardrobe::Slot::Coat)
            .unwrap() as u8;
        w.npc_mut(PLAYER).outfit.on[0] = Some(wardrobe::Worn {
            item: coat,
            from: None,
        });
        let rules: Vec<_> = check(&w).into_iter().map(|b| b.rule).collect();
        assert!(rules.contains(&"item-slot"), "{rules:?}");
    }
}
