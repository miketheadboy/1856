use super::attribution::{self, Rumor};
use super::chronicle;
use super::events::{EventKind, FireCause, MAX_CASCADE_DEPTH, Suspect};
use super::world::{NpcId, PLAYER, World};

fn first_of_family(w: &World, family: u32) -> NpcId {
    w.head_of(family).unwrap()
}

#[test]
fn same_seed_same_history() {
    let mut a = World::new(42);
    let mut b = World::new(42);
    a.run_days(200);
    b.run_days(200);
    assert_eq!(a.events.len(), b.events.len());
    assert_eq!(
        chronicle::chronicle(&a, true),
        chronicle::chronicle(&b, true)
    );
}

#[test]
fn different_seeds_diverge() {
    let mut a = World::new(1);
    let mut b = World::new(2);
    a.run_days(200);
    b.run_days(200);
    assert_ne!(
        chronicle::chronicle(&a, true),
        chronicle::chronicle(&b, true)
    );
}

#[test]
fn relationships_are_sparse_and_default_on_miss() {
    let w = World::new(7);
    let (a, b) = (first_of_family(&w, 1), first_of_family(&w, 2));
    assert!(
        w.relationship_count() <= 14,
        "only seeded grudges are stored"
    );
    let expected = if w.npc(a).faction == w.npc(b).faction {
        10
    } else {
        -10
    };
    // Either a seeded grudge or the faction default.
    assert!(w.opinion(a, b) == expected || w.opinion(a, b) <= -35);
}

#[test]
fn cascade_depth_is_capped() {
    let mut w = World::new(3);
    let victim = first_of_family(&w, 1);
    w.player_kill(victim);
    assert!(
        w.events
            .iter()
            .all(|e| e.cascade_depth <= MAX_CASCADE_DEPTH)
    );
}

#[test]
fn killing_someone_makes_their_family_grieve() {
    let mut w = World::new(5);
    let family = w.npc(first_of_family(&w, 3)).family;
    let victim = first_of_family(&w, 3);
    w.player_kill(victim);
    assert!(!w.npc(victim).alive);
    let mourners = w
        .events
        .iter()
        .filter(|e| matches!(e.kind, EventKind::Grief { deceased, .. } if deceased == victim))
        .count();
    let kin = w.living().filter(|n| n.family == family).count();
    assert_eq!(mourners, kin);
}

#[test]
fn prior_hostility_dominates_attribution() {
    let mut w = World::new(11);
    let owner = first_of_family(&w, 1);
    // Find two outsiders; make the owner hate one of them.
    let outsiders: Vec<NpcId> = w
        .living()
        .filter(|n| n.id != PLAYER && n.family != w.npc(owner).family && n.age >= 16)
        .map(|n| n.id)
        .take(2)
        .collect();
    let (hated, neutral) = (outsiders[0], outsiders[1]);
    w.set_opinion(owner, hated, -90);
    w.set_opinion(owner, neutral, 0);
    let fire = w.emit_root(
        EventKind::Fire {
            owner,
            cause: FireCause::Hearth,
            spread_from: None,
        },
        None,
    );
    let cands = attribution::candidates(&w, owner, fire, None);
    let score = |s: Suspect| cands.iter().find(|c| c.suspect == s).unwrap().score;
    assert!(score(Suspect::Person(hated)) > score(Suspect::Person(neutral)) + 40.0);
}

#[test]
fn an_alibi_protects_you() {
    let mut w = World::new(13);
    let owner = first_of_family(&w, 2);
    w.set_opinion(owner, PLAYER, -80);
    let fire = w.emit_root(
        EventKind::Fire {
            owner,
            cause: FireCause::Hearth,
            spread_from: None,
        },
        None,
    );
    let without = attribution::candidates(&w, owner, fire, None);
    w.player_go_to_tavern();
    let with = attribution::candidates(&w, owner, fire, None);
    let player = |c: &Vec<attribution::Candidate>| {
        c.iter()
            .find(|c| c.suspect == Suspect::Person(PLAYER))
            .unwrap()
            .score
    };
    assert!(player(&with) < player(&without) - 40.0);
}

#[test]
fn rumors_push_belief() {
    let mut w = World::new(17);
    let owner = first_of_family(&w, 4);
    let fire = w.emit_root(
        EventKind::Fire {
            owner,
            cause: FireCause::Hearth,
            spread_from: None,
        },
        None,
    );
    let rumor = Rumor {
        suspect: Suspect::Person(PLAYER),
        strength: 40.0,
    };
    let base = attribution::candidates(&w, owner, fire, None);
    let pushed = attribution::candidates(&w, owner, fire, Some(rumor));
    let p = |c: &[attribution::Candidate]| {
        let probs = attribution::beliefs(c);
        c.iter()
            .zip(probs)
            .find(|(c, _)| c.suspect == Suspect::Person(PLAYER))
            .unwrap()
            .1
    };
    assert!(p(&pushed) > p(&base));
}

#[test]
fn memory_is_capped_but_sticky_survives() {
    use super::events::Source;
    use super::world::{MEMORY_CAPACITY, MemoryRef};
    let mut w = World::new(19);
    let npc = w.npc_mut(1);
    npc.remember(MemoryRef {
        event: 9999,
        believed: Suspect::Person(PLAYER),
        confidence: 90,
        source: Source::Victim,
        day: super::Day(0),
        weight: 255,
    });
    for i in 0..100 {
        npc.remember(MemoryRef {
            event: i,
            believed: Suspect::Accident,
            confidence: 50,
            source: Source::Bystander,
            day: super::Day(i),
            weight: 10,
        });
    }
    assert_eq!(npc.memories.len(), npc.memory_capacity());
    assert!(npc.memory_capacity() <= MEMORY_CAPACITY);
    assert!(npc.memory_of(9999).is_some());
}

/// Phase 1 gate (§24): can the sim generate a war that nobody started?
#[test]
fn some_seed_produces_a_war_nobody_started() {
    let found = (1..=40).any(|seed| {
        let mut w = World::new(seed);
        w.run_days(365);
        !chronicle::wars_nobody_started(&w).is_empty()
    });
    assert!(
        found,
        "no seed in 1..=40 produced a feud rooted in a natural fire"
    );
}

/// The player can corner corn. The county notices.
#[test]
fn cornering_corn_spikes_the_price_and_breeds_resentment() {
    use super::market::Good;
    let mut w = World::new(21);
    w.winter_severity = 1.6;
    w.autopilot_player = false;
    w.run_days(60); // into the cold
    let before = w.market.price(Good::Corn);
    w.families[0].stores.cash = 5000;
    w.player_buy(Good::Corn, 10_000.0);
    assert!(w.market.stock(Good::Corn) < 1.0, "bought the store out");
    w.run_days(1);
    assert!(
        w.market.price(Good::Corn) > before * 2.0,
        "scarcity moves the price"
    );
    let rep_before: f32 = super::psyche::reputation(&w, PLAYER);
    w.run_days(90);
    assert!(
        super::psyche::reputation(&w, PLAYER) < rep_before,
        "the hoarder's name goes bad in a hungry winter"
    );
}

/// Phase 2 gate (§24): does a bad winter change the political map six months later?
/// Politics here: thefts, storekeeper favors, and accusations over thefts by
/// August 1856. Accusations over fires are left out: they are dominated by
/// whichever feud cascades a seed happens to roll, not by the winter.
#[test]
fn a_harsh_winter_sours_the_summer() {
    use super::events::Suspect;
    let politics = |severity: f32| -> usize {
        (1..=24)
            .map(|seed| {
                let mut w = World::with_winter(seed, Some(severity));
                w.run_days(290);
                w.events
                    .iter()
                    .map(|e| match e.kind {
                        EventKind::Theft { .. } | EventKind::Favor { .. } => 20,
                        EventKind::Belief {
                            blamed: Suspect::Person(_),
                            confidence,
                            about,
                            ..
                        } if confidence >= 20
                            && matches!(w.events[about as usize].kind, EventKind::Theft { .. }) =>
                        {
                            1
                        }
                        _ => 0,
                    })
                    .sum::<usize>()
            })
            .sum()
    };
    let (mild, harsh) = (politics(0.5), politics(1.6));
    assert!(
        harsh as f32 > mild as f32 * 1.1,
        "mild {mild}, harsh {harsh}"
    );
}
