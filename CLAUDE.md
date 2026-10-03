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
src/main.rs      Bevy view: app wiring, screens (County, Claim, Town). No rules here.
  county.rs      the wide map: zoom/pan, click a neighbor, your claim or a town
  claim.rs       your homestead on foot, drawn from sim state
  town.rs        Lawrence, Franklin, Lecompton streets; who's in town today
  walk.rs        walking, spots, and what each spot offers today
  cmds.rs        commands -> sim verbs; submenus (neighbor, after dark, store...)
  scene.rs       letterboxed moments: the knock, the petition, the muster, riders at the gate...
  duel.rs        standoffs by hand (talk down, stare down, quick-draw and aim), the road ambush, the bench
                 (the posse at your gate plays the same standoff)
  rig.rs         cut-out rigs: a skeleton of parts, two-bone reach (rifle arm to its target,
                 the other hand on the barrel, knees over planted feet), knock-back springs,
                 the fall; clothes off the outfit, old wounds in the pose
  raid.rs        a neighbor's place at night: lantern cones, a dog, crawl, hold E to do it
  ui.rs          blue command windows, toast, top bar, status (Tab), paper (N)
  look.rs        the printed-sheet pass over the whole frame (assets/look.txt, hot-reloaded;
                 assets/shaders/look.wgsl); tension and night move it; BK_LOOK=0 turns it off
  audio.rs       the score: four stems (assets/audio/stem_*.ogg) mixed by tension, night and
                 fresh killings; BK_MUTE=1
                 rig parts load from assets/rig/<part>.png, scene plates from
                 assets/plates/<scene>.png (docs/ART.md has the contracts)
  panels.rs      text readouts; scenery.rs map art, seasons, weather, night
src/bin/lab.rs   the lab: run one system at a time and look inside it
src/bin/headless.rs  chronicle runner and seed survey
benches/sim.rs   criterion benches
assets/fonts/    IM Fell English, EB Garamond (OFL licenses alongside)
docs/ART.md      the art, sound and look guide: delivery specs and the creative brief
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
| `market` | Dunmore's store: thirteen goods (corn, seed, cattle, salt, powder, timber, whiskey, hides, coffee, sugar, bacon, cloth, iron), stock-driven prices, freight, blockade |
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
| `action` | Phase B: plots against you park at the gate as standoffs (talk, face, draw, back down) or you confront a neighbor; raid plans and ambush plans; the minigame's witnesses are *staged* for the perception system; men faced down are cowed for a season |
| `arms` | Phase C: each house's armory (Sharps "books" from the East, old guns, lead, balls, cartridges, rails) and where it's hid; the bench (cast, roll, split); Wednesday crates the river towns may seize; the sheriff's posse searching Free-State houses after its musters |
| `hands` | Phase D: kin and hired hands set to the fields, the stock, the timber or the night watch; a watch turns riders back at the fence (neighbors in a feud post one too); hands by the month from hungry households, wages to their people, the unpaid walk and the sour talk (the other side learns where the guns are); partisan companies (Kickapoo Rangers, Buford's, the Stubbs, Lane's) rented to sit on your place or ride on a neighbor's, and the other side's paper printing who paid |
| `warrant` | Phase D: the justice writes papers on what people swear to (eyes, losses, a neighbor's word; never the paper), slow on his own side; posses and bounty hunters; men come in, run, or fight; trials at Lecompton; you can give yourself up, lie low, light out for the States, meet them at the gate, or take a paper yourself (paid on delivery, half for a body). Violence only: no paper on a freedom seeker |
| `larder` | what the food is made of: meat as a share of it (butchering, the buffalo, game, Missouri bacon; no meat, less work), garden stores dug in September and eaten through the cold half-year (a cellar keeps them, sugar puts them up; none by late winter is scurvy), milk from a cow in the grass months (children), coffee a pound a week (a fortnight out and tempers fray, until they parch corn), cloth to mend clothes before they're rags; water from a well, the creek (low and foul in a dry August) or hauled by the barrel (a hand's day) |
| `freight` | the wagon to the river: Westport (three days each way, cheap, and Missourians stop Free-State wagons in a closed year) or the Lane Trail (a month, dearer, safe); a ton to the load by ox team; neighbors of your side send orders; a light-fingered teamster skims them; a man on the road has an alibi and isn't home to be shot; the webs out of the wagon: a free-state house hiding a freedom seeker drives them north up the Lane Trail (and neighbors' guests ride along), the Lane Trail brings the rifle crate the river towns would open, a Westport wagon in a cholera summer brings it home, a stopped teamster swears it was the Missouri neighbor he hates (and the Herald prints it), and a short-weight name sticks a year (`standing`); orders go only with a man whose word is good, at a dollar a hundredweight |
| `standing` | disparity multipliers. Each position a person holds (woman, widow, child, makes a mark, seen begging, in debt, new to the county, jailbird, hired man, the other side all round, man of property) has a **word** factor (whose testimony carries: gossip, oaths, court, credit, claims, nursing) and a **repute** factor (who gets the benefit of the doubt: blame's "no account" term, how few oaths make a paper, conviction). Products, not sums |
| `sickness` | ague in the bottoms, cholera up the river, typhoid from a fouled well, the flux, measles, whooping cough, diphtheria, smallpox, typhus from lice, consumption, scurvy, lung fever, wounds gone bad; spread by house, school and neighbor; quinine works, the doctor's calomel and lancet don't; herbs (physic skill), nursing, quarantine, boiling the bedding; a lousy blanket as a dark verb; the burying: friends who sit up with one dead of a catching fever carry it home (enemies stay away and stay well), and a careful house that keeps away is remembered for it; the first bowel fever in a house sends its head looking for a poisoner through the attribution engine (`bad water` against the neighbor he hates) |
| `wardrobe` | 40 pieces with warmth, skill bonuses, nerve, draw, aim, stealth, charm and side colors; a look per path with a set bonus; colors are what a faceless witness sees; loot from the fallen and from trunks, and taken clothes that their owners' kin recognize |
| `intrigue` | the player's dark verbs: secrets learned at the groggery or by moonlight, blackmail, exposure, sabotage (shoot a cow, foul a well, cut a fence, wet the hay) read through attribution, slander |
| `railroad` | the Underground Railroad (MVP §25): freedom seekers who pick doors by word and travel in the dark of the moon; families answer by private belief (hide, turn away, turn in for the reward); food as evidence; pursuers who linger and ask; captures read by attribution; harboring charged under the 1855 slave code; when a fugitive is taken, whoever came into money that fortnight (a bank note in a letter, or the reward) is suspected of informing; debt or hungry children thin a lukewarm Free-State conscience |
| `legacy` | the papers write you up (readers take the paper's view of you); a scandal shuts the store's book; your children drift toward your path and, near grown, take after you or rebel |
| `homestead` | improvements (well, smokehouse, crib, cellar, rail fence) that take days and timber and each change a mechanic; timber cut off the creek bottoms until the stands are stumps; winter firewood; spring pasture burns that green the grass or get away |
| `mortality` | childhood sickness by age, season, frailty, hunger and exposure |
| `geography` | the county map at half a mile per tile; terrain effects |
| `chronicle` | text for every event; cascade trees; lineage; "wars nobody started" |
| `marks` | what a life leaves on a person, each earned by one event and read by other systems: came through the fever (sits up with the dead unafraid, nurses better), has killed a man (private; weariness spares more, and the anniversary comes back on him, with his victim's spirit), buried a child (stays the hand at a house with children; grieves again a year on), raised an enemy's barn, turned riders back at the fence, kept a station (private; the next knock is easier), took the fifty dollars (private; the second time is easier, and a lukewarm conscience comes back in a month), knows the river road (stopped half as often), married across the line ("their side" weighs half), the starving time (the house lays by more), reads the county's letters. Public marks move `standing`, so blame, oaths, credit and gossip feel them |
| `audit` | the books against each other: every system's state checked against the others' (dead men sick, hired or jailed; papers nobody swore to; coats in hat slots; deaths with no event). `lab audit` and `the_books_agree_every_day` |
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
cargo run --bin lab --no-default-features -- standoff --seeds 40
cargo run --bin lab --no-default-features -- arms --days 330
cargo run --bin lab --no-default-features -- law --days 730
cargo run --bin lab --no-default-features -- hands --seeds 10 --days 540
cargo run --bin lab --no-default-features -- sick --seeds 20 --days 730
cargo run --bin lab --no-default-features -- dress
cargo run --release --bin lab --no-default-features -- audit --seeds 30 --days 730
cargo run --release --bin lab --no-default-features -- webs --seeds 30 --days 730
cargo run --release --bin lab --no-default-features -- matrix --seeds 30 --days 730
cargo run --release --bin lab --no-default-features -- marks --seeds 30 --days 730
cargo run --release --bin lab --no-default-features -- standing --seeds 30 --days 730
cargo run --release --bin lab --no-default-features -- larder --days 400 --seeds 30
cargo run --bin headless --no-default-features -- --seed 15 --truth
cargo run --bin headless --no-default-features -- --survey 30 --days 730

cargo run                                         # the game (needs a display)
BK_SEED=15 BK_START_DAYS=200 BK_DAY_SECONDS=2 cargo run
BK_SCREEN=lawrence cargo run                      # start in county|claim|lawrence|franklin|lecompton
BK_PLAY=gate cargo run                            # straight into gate|door|raid|ambush|bench|posse|paper
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
7. **The books agree, and lie only on purpose.** A record the county keeps
   may be false (a padded muster roll, a bigamous marriage, a bedside death
   written up as murder, or as fever), but every such lie is an event
   (`RollPadded`, `Bigamy`, a `Death` caused by its `Wounded`) and
   `audit::lies` checks each against the truth. A lie with no event behind
   it is a bug. Anything that holds a person (a list of the sick,
   the hired, the jailed, the wanted) handles `Death`/`Perished` the day
   it happens. New cross-system facts get a rule in `audit::check`; run
   `lab audit` after any change that touches two systems. A new web (one
   system's event moving another's books) goes in `lab webs`'s watch list,
   so a cut wire shows as "never fired". Every event kind belongs to a
   system in `debug::OWNERS`; `lab matrix` draws the systems as a graph
   (same-day and delayed links, three-system chains) and names the islands.
   When an effect has a cause on an earlier day, pass it as `caused_by`: a
   link the log doesn't record is a link the matrix can't see.
8. **Tests live with their module** (`#[cfg(test)] mod tests` at the bottom).
   Cross-system and gate tests live in `src/sim/tests.rs`.

## Balancing

Tune with the lab, not by feel: `lab gate`, `lab evil`, `lab winter`,
`headless --survey`. Current targets (two years, 30 seeds): a war nobody
started in about half of seeds, ~3–4 violent deaths per seed, harsh winters
measurably worse than mild ones by August 1856. Known open issues: faction
grievance still peaks above 100 in winter 1855–56 (saturates the UI bars and
trips the Westport blockade early); fires run ~30 per two years (Phase D's
watches brought them down from ~34); the justice's papers land on the wrong
man ~90% of the time, because the county's blame does (`headless --survey`
prints it). There are ~5.6 papers a seed: a woman's or an unlettered man's
oath weighs less, but a death from an old wound is now a murder.

Realism anchors (tests hold them; the lab measures them):

- **Eyewitnesses** (Wells & Loftus): right ~60% of the time, from light,
  distance, fear, a weapon and familiarity. When they're wrong they are just
  as sure, and the man they name is of the same side
  (`witnesses_are_sure_and_often_wrong`).
- **Rumor** (Allport & Postman, Rosnow, Bartlett): talk goes as importance ×
  ambiguity × county fear (`systems::talkability`, `county_fear`). About 40%
  of tellings carry the teller's blame. Stories sharpen toward the teller's
  grudges, and the teller's own memory moves first.
- **Standing** (Crenshaw, Fricker): `lab standing --seeds 30 --days 730`
  prints the disparity table, per 100 person-years against grown people
  with none of the positions. Last run:
  - women are wrongly blamed ×0.3 but their claims are jumped ×1.9;
  - widows' claims are jumped ×3.6;
  - the poor are wrongly blamed ×1.5–1.7 and papered ×3.6–30;
  - jailbirds are wrongly blamed ×16.

## Style

- Comments explain why, in the design's voice, with § references. No
  narration of what the code obviously does.
- Chronicle text is period-plain and short. The default Bevy font lacks some
  glyphs; the view runs text through `plain()`.
- No button walls. Verbs live on things: walk up to the barn, the creek, a
  door in town, a neighbor, and press E. Command windows grey out what
  can't be done today. Moments that won't wait are scenes (`scene.rs`).
- Visual style: survey plat on charcoal, bone type, live greens and river
  blue, oxblood and brass accents. Woodcut sprites (`tools/woodcut.py`
  writes `assets/sprites/`); seasons on the grass, weather and smoke over the
  map, and night by the moon (`src/scenery.rs`). The log is a broadsheet
  column under whichever paper spoke last. A few quiet Dylan nods (names,
  "a hard rain", the death line, "tangled up", "shelter from the storm",
  "it's all over now"). Titles and allusions only, never lyrics.

## Git

Work on the designated `claude/...` branch; commit with clear messages; the
open PR is miketheadboy/1856#2.
