# 1856
Bleeding Kansas: The Game

A systems-driven survival narrative prototype set in Kansas Territory, exploring attribution, memory, faction pressure, and emergent historical consequence.

## Project status

This repository currently contains:

- a playable Bevy prototype of the core simulation loop
- a Part I design extension pack focused on temporal drift, narrative scarcity, and identity fragmentation
- a design-first framework for the broader Bleeding Kansas master plan

## Included docs

- [PART I — The Substrate (Extensions)](PART_I_EXTENSIONS.md)

## Bevy prototype

This is the first playable slice from the master plan's Bevy bake-off:

- ten named neighbors wander a small map;
- click a neighbor to inspect their mood and opinion of you;
- select **KILL** to drive an event cascade;
- the cascade log shows death, grief, opinion loss, and gossip.

Install Rust, then from this folder run:

```bash
cargo run
```

The simulation logic is kept in systems and messages so the visual layer can
later be replaced without changing the cascade behavior.

## Design direction

The core design intent is not just a political sim. The game is built around the idea that:

- events are misinterpreted before they are understood
- narratives compete for attention
- institutions harden one version of reality
- legal truth, social truth, and faction truth do not match

The goal is a world where the player can be correct, still be blamed, and still lose because the simulation has already decided what happened.
