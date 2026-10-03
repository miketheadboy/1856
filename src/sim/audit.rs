//! The auditor: every system keeps its own books, and the books have to
//! agree. A dead man on the payroll, a jailed man on a posse, a coat worn in
//! a slot it doesn't fit, a death with no event behind it: each system alone
//! looks fine, and only laid side by side do they show the bug (rule 6: if
//! emergence can't be told from a bug, add a lens).
//!
//! `check` is read-only and cheap enough to run every day of a test; the
//! cross-system tests in `tests.rs` and `lab audit` run it across seeds.

use super::events::{Cruelty, EventKind, MAX_CASCADE_DEPTH, Suspect};
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
    market(&mut b);
    ghosts(&mut b);
    standoff(&mut b);
    law(&mut b);
    railroad(&mut b);
    gossip(&mut b);
    lies(&mut b);
    wagons(&mut b);
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

/// Dunmore's shelves: prices and stock stay real numbers.
fn market(b: &mut Books) {
    let m = &b.world.market;
    for (g, st) in m.goods.iter().enumerate() {
        if !st.stock.is_finite() || st.stock < -0.001 {
            b.breach("market-stock", format!("good {g} stock {}", st.stock));
        }
        if !st.price.is_finite() || st.price <= 0.0 {
            b.breach("market-price", format!("good {g} price {}", st.price));
        }
    }
    for (name, v) in [("freight", m.freight_factor), ("trail", m.trail_safety)] {
        if !v.is_finite() || v < 0.0 {
            b.breach("market-range", format!("{name} {v}"));
        }
    }
    for f in &b.world.families {
        let a = &f.stores.arms;
        if !a.lead.is_finite() || a.lead < -0.001 {
            b.breach(
                "arms-lead",
                format!("the {}s hold {} lead", f.surname, a.lead),
            );
        }
    }
}

/// Oaths against the graveyard: an open oath needs a living man to keep it
/// and a living man to keep it on; a ghost needs to be dead.
fn ghosts(b: &mut Books) {
    let w = b.world;
    for o in &w.ghosts.oaths {
        if o.done || !b.exists("oath", o.holder) || !b.exists("oath", o.target) {
            continue;
        }
        if o.holder == o.target {
            b.breach("oath-self", format!("{} swore on himself", b.who(o.holder)));
        }
        if o.woke && !(w.npc(o.holder).alive && w.npc(o.target).alive) {
            b.breach(
                "oath-dead",
                format!(
                    "{}'s oath on {} is open with a dead man in it",
                    b.who(o.holder),
                    b.who(o.target)
                ),
            );
        }
    }
    for h in &w.ghosts.haunts {
        if b.exists("haunt", h.spirit) && w.npc(h.spirit).alive {
            b.breach(
                "haunt-living",
                format!("{} haunts while alive", b.who(h.spirit)),
            );
        }
    }
}

/// The standoff at the gate against the jail and the graveyard.
fn standoff(b: &mut Books) {
    let w = b.world;
    let Some(s) = &w.action.standoff else { return };
    if b.exists("standoff", s.actor) && !w.npc(s.actor).alive {
        b.breach(
            "standoff-dead",
            format!("{} at the gate, dead", b.who(s.actor)),
        );
    }
    for &r in &s.riders {
        if b.exists("standoff", r) && (!w.npc(r).alive || b.held(r)) {
            b.breach(
                "standoff-rider",
                format!("{} rides up from jail or grave", b.who(r)),
            );
        }
    }
    if let Some(i) = s.serving
        && i >= w.warrants.list.len()
    {
        b.breach("standoff-paper", format!("serving missing paper {i}"));
    }
}

/// The muster roll against the jail and the grave.
fn law(b: &mut Books) {
    let w = b.world;
    if let Some((_, _, men)) = &w.law.muster {
        for &m in men {
            if b.exists("muster", m) && !w.npc(m).alive {
                b.breach("muster-dead", format!("{} mustered dead", b.who(m)));
            }
        }
    }
    for d in &w.law.disputes {
        if d.plaintiff as usize >= w.families.len() {
            b.breach("dispute-family", format!("no family {}", d.plaintiff));
        }
        b.exists("dispute", d.defendant);
    }
}

/// Freedom seekers: whose door, who's after them.
fn railroad(b: &mut Books) {
    let w = b.world;
    let r = &w.railroad;
    let n = r.seekers.len();
    if let Some(i) = r.at_door
        && i as usize >= n
    {
        b.breach("seeker-door", format!("seeker {i} at the door of {n}"));
    }
    for &(i, _, _) in &r.pursuit {
        if i as usize >= n {
            b.breach("seeker-pursuit", format!("pursuit of missing seeker {i}"));
        }
    }
    for list in [&r.safe, &r.hostile] {
        for &f in list {
            if f as usize >= w.families.len() {
                b.breach("seeker-family", format!("no family {f}"));
            }
        }
    }
    for s in &r.seekers {
        for &id in &s.noticed_by {
            b.exists("seeker-noticed", id);
        }
    }
}

/// Gossip against belief: people pass on only what they themselves believe,
/// the dead don't talk (except as ghosts, by moonlight), nobody tells
/// himself, and every telling lands on an event that already happened.
/// Only today's tellings are read, so a daily audit stays linear.
fn gossip(b: &mut Books) {
    let w = b.world;
    let today: Vec<_> = w
        .events
        .iter()
        .rev()
        .take_while(|e| e.day == w.day)
        .filter(|e| matches!(e.kind, EventKind::Gossip { .. }))
        .collect();
    if today.is_empty() {
        return;
    }
    let mut died = vec![u32::MAX; w.npcs.len()];
    for e in &w.events {
        if let EventKind::Death { victim, .. } | EventKind::Perished { victim, .. } = e.kind
            && let Some(d) = died.get_mut(victim as usize)
        {
            *d = (*d).min(e.id);
        }
    }
    for e in today {
        let EventKind::Gossip {
            teller,
            listener,
            about,
            blamed,
        } = e.kind
        else {
            continue;
        };
        if !b.exists("gossip", teller) || !b.exists("gossip", listener) {
            continue;
        }
        if teller == listener {
            b.breach(
                "gossip-self",
                format!("#{}: {} told himself", e.id, b.who(teller)),
            );
        }
        if about >= e.id {
            b.breach("gossip-about", format!("#{} tells of later #{about}", e.id));
            continue;
        }
        let ghost = w
            .ghosts
            .haunts
            .iter()
            .any(|h| h.spirit == teller && h.death == about);
        if died[teller as usize] < e.id && !ghost {
            b.breach(
                "gossip-dead",
                format!("#{}: the dead {} talked", e.id, b.who(teller)),
            );
        }
        if died[listener as usize] < e.id {
            b.breach(
                "gossip-deaf",
                format!("#{}: told the dead {}", e.id, b.who(listener)),
            );
        }
        // What the teller says must be what the teller believes (or did,
        // before changing his mind): a Belief of his own, earlier in the log.
        // Ghosts speak the truth of their own deaths.
        // Slander is a lie on purpose, told through the same mouths.
        let slander = e.parent.is_some_and(|p| {
            matches!(
                w.events[p as usize].kind,
                EventKind::Cruelty {
                    act: Cruelty::Slander { .. },
                    ..
                }
            )
        });
        let believed = ghost
            || slander
            || w.events[..e.id as usize].iter().any(|x| {
                matches!(x.kind, EventKind::Belief { holder, about: a, blamed: bl, .. }
                    if holder == teller && a == about && bl == blamed)
            });
        if !believed {
            b.breach(
                "gossip-unbelieved",
                format!(
                    "#{}: {} told {} a thing he never believed about #{about}",
                    e.id,
                    b.who(teller),
                    b.who(listener)
                ),
            );
        }
    }
}

/// The county's records may lie, but only on purpose, and every lie leaves
/// an event behind it. A padded roll names only the dead, each by a
/// `RollPadded`; a bigamous marriage is a real marriage until the letter
/// comes; a death from an old wound names the man who gave the wound, and
/// nobody saw it happen.
fn lies(b: &mut Books) {
    let w = b.world;
    for &name in &w.law.padded {
        if !b.exists("padded", name) {
            continue;
        }
        if w.npc(name).alive {
            b.breach(
                "padded-living",
                format!("{} padded onto the roll alive", b.who(name)),
            );
        }
        let evented = w
            .events
            .iter()
            .rev()
            .any(|e| matches!(e.kind, EventKind::RollPadded { name: n, .. } if n == name));
        if !evented {
            b.breach(
                "padded-silent",
                format!("{} on the roll with no RollPadded", b.who(name)),
            );
        }
    }
    for &(liar, here, _, _) in &w.hearts.bigamous {
        if !b.exists("bigamy", liar) || !b.exists("bigamy", here) {
            continue;
        }
        let wed = w
            .hearts
            .couples
            .iter()
            .any(|&(x, y)| (x, y) == (liar.min(here), liar.max(here)));
        if !wed {
            b.breach(
                "bigamy-unwed",
                format!("{} a bigamist with no wedding", b.who(liar)),
            );
        }
    }
    // Nobody watches a man die of a fever in his bed.
    for e in w.events.iter().rev().take_while(|e| e.day == w.day) {
        // (Those who saw the shooting carry their sight over; nobody sees
        // the death itself.)
        if let EventKind::Belief {
            about,
            reason: "saw it with their own eyes",
            ..
        } = e.kind
            && super::systems::fatal_wound(w, &w.events[about as usize]).is_some()
        {
            b.breach(
                "wound-witnessed",
                format!("#{} saw a bedside death happen", e.id),
            );
        }
    }
}

/// Wagons on the road against the jail, the barn and the calendar; the
/// larder against its own sums.
fn wagons(b: &mut Books) {
    let w = b.world;
    let mut seen: Vec<NpcId> = Vec::new();
    for t in &w.freight.trips {
        if !b.exists("wagon", t.teamster) {
            continue;
        }
        if seen.contains(&t.teamster) {
            b.breach(
                "wagon-twice",
                format!("{} drives two wagons", b.who(t.teamster)),
            );
        }
        seen.push(t.teamster);
        if t.back < w.day {
            b.breach(
                "wagon-late",
                format!("{} never came home", b.who(t.teamster)),
            );
        }
        if w.warrants
            .held
            .iter()
            .any(|h| h.0 == t.teamster && h.2 == Held::Jail)
        {
            b.breach(
                "wagon-jailed",
                format!("{} drives from jail", b.who(t.teamster)),
            );
        }
    }
    for &(t, _) in &w.freight.shorted {
        if b.exists("short-weight", t)
            && !w
                .events
                .iter()
                .any(|e| matches!(e.kind, EventKind::ShortWeight { teamster, .. } if teamster == t))
        {
            b.breach(
                "short-weight-silent",
                format!("{} called a short-weight man with no ShortWeight", b.who(t)),
            );
        }
    }
    // The webs out of the wagon: each consequence is today's, and it holds
    // to its route.
    for e in w.events.iter().rev().take_while(|e| e.day == w.day) {
        let Some(cause) = e.parent.or(e.caused_by) else {
            continue;
        };
        let from = &w.events[cause as usize].kind;
        match (&e.kind, from) {
            // Nobody carries a fugitive south into Missouri.
            (
                EventKind::Freedom { .. },
                EventKind::WagonOut {
                    route_west: true, ..
                },
            ) => b.breach(
                "seeker-south",
                format!("#{} freed by a Westport wagon", e.id),
            ),
            (
                EventKind::FellSick { who, disease },
                EventKind::WagonBack {
                    teamster,
                    route_west,
                    ..
                },
            ) if who != teamster
                || *disease != super::sickness::Disease::Cholera
                || !route_west =>
            {
                b.breach(
                    "wagon-sick",
                    format!("#{} caught off a wagon it shouldn't have", e.id),
                )
            }
            (
                EventKind::ArmsArrived { family, .. },
                EventKind::WagonBack {
                    teamster,
                    route_west,
                    ..
                },
            ) if *route_west || w.npc(*teamster).family != *family => b.breach(
                "wagon-rifles",
                format!("#{} rifles off the wrong wagon", e.id),
            ),
            (
                EventKind::Belief {
                    holder,
                    blamed: Suspect::Person(x),
                    ..
                },
                EventKind::WagonStopped { teamster, .. },
            ) if holder == teamster && w.npc(*x).faction == w.npc(*holder).faction => b.breach(
                "wagon-blame",
                format!("#{} a teamster blames his own side for the road", e.id),
            ),
            _ => {}
        }
    }
    for f in &w.families {
        let l = &f.stores.larder;
        if !(0.0..=1.0).contains(&l.meat_share) {
            b.breach(
                "larder-meat",
                format!("the {}s' meat share {}", f.surname, l.meat_share),
            );
        }
        if !l.garden.is_finite() || l.garden < -0.01 {
            b.breach(
                "larder-garden",
                format!("the {}s' garden {}", f.surname, l.garden),
            );
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
