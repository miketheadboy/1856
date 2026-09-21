# 1856
Bleeding Kansas: The Game

## Bevy prototype

This repository contains the first playable slice from the master plan's
Bevy bake-off:

- ten named neighbors wander a small map;
- click a neighbor to inspect their mood and opinion of you;
- select **KILL** to drive an event cascade;
- the cascade log shows death, grief, opinion loss, and gossip.

Install Rust, then run:

```text
cargo run
```

The simulation logic is kept in systems and messages so the visual layer can
later be replaced without changing the cascade behavior.
