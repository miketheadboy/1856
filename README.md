# 1856
Bleeding Kansas: The Game

A systems-driven survival narrative prototype set in Kansas Territory, exploring attribution, memory, faction pressure, and emergent historical consequence.

## Project status

This repository currently contains:

- a headless simulation core (`src/sim/`) with no engine dependency (master plan §21)
- a Bevy view that draws the sim and forwards the player's verbs
- a Part I design extension pack focused on temporal drift, narrative scarcity, and identity fragmentation

## Included docs

- [PART I — The Substrate (Extensions)](PART_I_EXTENSIONS.md)

## The simulation

One county, winter 1855 onward (§25). Eight families, about twenty neighbors,
two factions, your farm in the middle. What's built:

- **Event bus with cascade control** (§15): systems emit events and never call
  each other. Same-day chains stop at depth 6; consequences on later days start
  a new chain linked back by `caused_by`, so a feud can run for months.
- **Attribution engine** (§11): nobody sees causes. Victims score every
  suspect on old hatred, proximity, motive, pattern and alibi, plus rumor,
  then act on whatever they conclude with full force.
- **Fire** (§12): lightning, hearths, spring burns that got away, and arson all
  look identical afterward. Fire spreads with wind and dryness.
- **Gossip that mutates**: each listener reruns attribution, nudged by what they
  heard, and may land on someone else entirely.
- **Sparse relationships** (§14.2), shared canonical events with per-NPC
  beliefs (§14.3), and capped memory with sticky trauma (§14.4).
- **Revenge, feuds, and faction grievance**, with moral momentum (ext. §17):
  each act of violence makes the next one easier.
- **Seeded determinism** (§18.1): same seed, same history.

### Headless runner

No graphics needed. This is where you tune the systems:

```bash
cargo run --bin headless --no-default-features                   # one year, seed 1856
cargo run --bin headless --no-default-features -- --seed 15 --truth
cargo run --bin headless --no-default-features -- --trees 5       # cascade debug trees
cargo run --bin headless --no-default-features -- --survey 60     # how often do wars start from nothing?
```

`--truth` shows the real cause of each fire. Without it you get what the county
believes. Each run ends with a **WARS NOBODY STARTED** section: feuds whose
whole lineage traces back to a fire no human set. That's the Phase 1 gate (§24),
and a test (`some_seed_produces_a_war_nobody_started`) keeps it passing.

### Bevy view

```bash
cargo run
```

Days pass on their own. Click a neighbor to see their opinion of you and who
they blame for the last thing that happened to them. **KILL**, **BURN BARN**,
**BE SEEN** (go to the tavern for an alibi), **PAUSE**. Sprites shade from warm
to cold blue as a neighbor's opinion of you drops. Black barns are burned.
The log only shows what the county is saying, not what happened.

### Tests

```bash
cargo test --no-default-features
```

Covers determinism, cascade depth, grief, attribution bias (hatred, alibis,
rumor), memory eviction, and the Phase 1 gate.

## Design direction

The core design intent is not just a political sim. The game is built around the idea that:

- events are misinterpreted before they are understood
- narratives compete for attention
- institutions harden one version of reality
- legal truth, social truth, and faction truth do not match

The goal is a world where the player can be correct, still be blamed, and still lose because the simulation has already decided what happened.
