# CLAUDE.md

Guidance for working in this repo. Read this first.

## What this is

**Bleeding Kansas (1856)**: a systems-driven survival narrative set in Douglas
County, Kansas Territory, from November 1855. The design lives in the master
plan (Google Drive: `BLEEDING_KANSAS_master_plan.md`) and
`PART_I_EXTENSIONS.md`. Section numbers in code comments (`§11`, `ext. §16`)
refer to those documents.

The thesis: nobody sees causes, only effects. People blame whoever they
already hated, act on it with full force, and history is what survived long
enough to be believed. The player can be right, still be blamed, and still lose.

## Layout

```
src/sim/         the simulation. No engine dependency, ever (§21).
src/main.rs      Bevy view: draws the sim, forwards player verbs. No rules here.
src/bin/lab.rs   the lab: run one system at a time and look inside it
src/bin/headless.rs  chronicle runner and seed survey
benches/sim.rs   criterion benches
assets/fonts/    IM Fell English, EB Garamond (OFL licenses alongside)
docs/screenshots/
```

Sim modules (`src/sim/`):

| module | owns |
| --- | --- |
| `world` | `World` state, the daily tick, player verbs, relationships (sparse, default on miss) |
| `events` | `EventKind`, `WorldEvent`, cascade depth limit |
| `systems` | event handlers: fire, death, perception, gossip, belief, opinion, revenge, faction, favors, cruelty, wounds, press |
| `attribution` | the §11 engine: candidates, per-term score `parts`, softmax, sampled verdict |
| `economy` | households (§10): food, seed, stock, oxen, debt; hunger choices by temperament |
| `market` | Dunmore's store: eight goods, stock-driven prices, freight, blockade |
| `psyche` | body, temperament, emotions (+contagion), ideology, life stages, derived drives |
| `character` | archetypes from stat combos, hidden luck/malice, daily evil |
| `history` | 1855–57 timeline with mechanical effects; the three partisan papers |
| `nations` | Delaware, Shawnee, Kaw as encroached-upon sovereign actors |
| `bison` | the herd west of the county, robe-price feedback loop |
| `institutions` | Jack of Hearts, Free State Hotel, church, barbershop, river-town secrets |
| `ghosts` | oaths sworn over the dead, inherited oaths, children who come of age, shame when kin think the dead deserved it, restless spirits seen by moonlight |
| `reconcile` | the better way: mercy at the moment of the act, barn raisings (a rival among the hands), apologies, condolence at a child's grave, enemy-of-my-enemy, feuds that starve, truces, the player's broker verb |
| `family` | kin and absence: errands (buffalo, fishing, courting, town, drinking) take the player away; harm at home while away costs kin's regard |
| `farmwork` | the farm year: plow, plant (almanac folk wait for the moon and plant late), hay, picking corn before December, butchering, fences; strays feed attribution; short hay kills stock |
| `life` | the player's days: one activity a day (chores, fish, roam, trap, camp, visit, court, drink, preach, baptize, speech, build, file a claim, study, write home), eight skills that grow by doing, spirits, Jones-style goals, emergent paths (preacher, souse, rake, silver tongue, vagabond, pillar...) |
| `romance` | couples, courtship, seduction, affairs, scandal, marriages that make in-laws (and end feuds), births (and deaths in childbed) |
| `civic` | county projects (schoolhouse, bridge, lyceum), the Lecompton land office and claim jumpers, festivals |
| `mail` | the Lawrence post office on Wednesdays (not through a blockade): money, deaths back home, kin coming out (new household members); the unlettered need a neighbor to read, who then knows their business |
| `law` | the pro-slavery justice of the peace and lawsuits over jumped claims; the 1855–57 elections (Topeka votes, Missourians at territorial polls, votes sold at the store, Walker's rejected returns); militia musters (Wakarusa, Jones's posse, Franklin, Hickory Point) with shirkers noticed and men shot |
| `bees` | husking bees (shuck the host's corn, the red ear), quilting bees (quilts cut winter need), spelling bees, singing school (courting); the union meeting that mixes both sides until tension splits it North and South |
| `land` | Lawrence town lots through the 1856 emigration, the Sack, the 1857 boom and the Panic; buying a broken family's relinquishment (they depart for the States) |
| `intrigue` | the player's dark verbs: secrets learned at the groggery or by moonlight, blackmail, exposure, sabotage (shoot a cow, foul a well, cut a fence, wet the hay) read through attribution, slander |
| `railroad` | the Underground Railroad (MVP §25): freedom seekers who pick doors by word and travel in the dark of the moon; families answer by private belief (hide, turn away, turn in for the reward); food as evidence; pursuers who linger and ask; captures read by attribution; harboring charged under the 1855 slave code |
| `legacy` | the papers write you up (readers take the paper's view of you); a scandal shuts the store's book; your children drift toward your path and, near grown, take after you or rebel |
| `mortality` | childhood sickness by age, season, frailty, hunger and exposure |
| `geography` | the county map at half a mile per tile; terrain effects |
| `chronicle` | text for every event; cascade trees; lineage; "wars nobody started" |
| `debug` | daily metrics, event trace, `explain_npc` / `explain_blame` / `explain_price` |
| `calendar`, `rng` | `Day` (day 0 = 1 Nov 1855), seeded SplitMix64 |

## Commands

The sim, tests and tools build without Bevy; always pass `--no-default-features`
unless you need the window.

```bash
cargo test --no-default-features                 # unit tests + Phase 1/2 gates
cargo clippy --all-targets --no-default-features # must be clean
cargo fmt --check                                # must be clean
cargo bench --no-default-features                # tick/attribution/market costs

cargo run --bin lab --no-default-features -- help
cargo run --bin lab --no-default-features -- npc Pike --days 200
cargo run --bin lab --no-default-features -- blame <event-id>
cargo run --bin lab --no-default-features -- gate --seeds 30
cargo run --bin lab --no-default-features -- life preacher --days 540
cargo run --bin lab --no-default-features -- county --days 730
cargo run --bin headless --no-default-features -- --seed 15 --truth
cargo run --bin headless --no-default-features -- --survey 30 --days 730

cargo run                                         # the game (needs a display)
BK_SEED=15 BK_START_DAYS=200 BK_DAY_SECONDS=2 cargo run
```

Linux containers need `libwayland-dev libxkbcommon-dev libasound2-dev
libudev-dev pkg-config` to build Bevy, plus `xvfb mesa-vulkan-drivers
libxkbcommon-x11-0` to run it headless for screenshots. Set
`BEVY_ASSET_ROOT` to the repo root when running the binary directly.

## Rules of the codebase

1. **Emit, don't call (§15).** Systems react to events and emit new ones. A
   new consequence is a new handler in `systems::dispatch`, never a call from
   one system into another. Same-tick chains use `emit_child` (depth-limited);
   consequences on later days use `schedule` / `emit_root(.., caused_by)`.
2. **Every stat drives a mechanic.** If a number doesn't change an outcome
   somewhere, it doesn't belong. When adding a stat, wire it in the same change
   and say where in its doc comment.
3. **The player never sees truth.** Chronicle lines for secret acts
   (revenge plots, slander, paid blackmail) are gated on `omniscient`. Private
   ideology, malice and luck never appear in the view.
4. **Deterministic.** All randomness goes through `world.rng`. Same seed, same
   history (there's a test). No `HashMap` iteration order in anything that
   touches the RNG or event order.
5. **History serves the systems.** Real events, places and people are used
   where sources agree; mark approximations (`approximate: true` on places).
   Native nations follow Miner & Unrau (*The End of Indian Kansas*): encroached
   upon, legalistic, neutral in the slavery fight, not raiders.
6. **Observable.** Anything new should show up in `debug::explain_*`, the
   chronicle, or `lab`. If emergence can't be told from a bug, add a lens.
7. **Tests live with their module** (`#[cfg(test)] mod tests` at the bottom).
   Cross-system and gate tests live in `src/sim/tests.rs`.

## Balancing

Tune with the lab, not by feel: `lab gate`, `lab evil`, `lab winter`,
`headless --survey`. Current targets (two years, 30 seeds): a war nobody
started in about half of seeds, ~3–4 violent deaths per seed, harsh winters
measurably worse than mild ones by August 1856. Known open issues: faction
grievance still peaks above 100 in winter 1855–56 (saturates the UI bars and
trips the Westport blockade early); fires run ~25 per two years.

## Style

- Comments explain why, in the design's voice, with § references. No
  narration of what the code obviously does.
- Chronicle text is period-plain and short. The default Bevy font lacks some
  glyphs; the view runs text through `plain()`.
- The view shows only the verbs that apply today (`available` in main.rs):
  the ballot on election day, the door when someone's knocking.
- Visual style: survey plat on charcoal, bone type, live greens and river
  blue, oxblood and brass accents. A few quiet Dylan nods (names, "a hard
  rain", the death line). Titles and allusions only, never lyrics.

## Git

Work on the designated `claude/...` branch; commit with clear messages; the
open PR is miketheadboy/1856#1.
