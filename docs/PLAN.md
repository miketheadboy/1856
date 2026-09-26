# Bleeding Kansas — build log and plan

A running record of everything asked for, what exists, and what comes next.
Section refs (§) point at the master plan and `PART_I_EXTENSIONS.md`.

## Where it stands

- 30+ sim modules, 123 tests, clippy/fmt clean, two simulated years in ~25 ms.
- Balance (50 seeds, two years): a war nobody started in ~18/50, ~1 violent
  death per seed-year, harsh winters measurably worse (Phase 2 gate).
- The Bevy view: three scales (county map, your claim on foot, town
  streets), command windows on things, letterboxed scenes, a status screen
  and the paper. Next: Phase B, action.

## Everything asked for, and where it went

| ask | status | where |
| --- | --- | --- |
| depth, many stats, all tied to mechanics; stat combos, hidden multipliers, chance of evil | done | `psyche`, `character` |
| a living, cornerable economy | done | `market`, `economy` |
| contemporary history and news, partisan papers | done | `history`, `legacy` |
| Native nations, historically accurate (eastern nations legalistic and neutral; plains nations off-map; adoption where proven) | done | `nations` |
| scale-accurate map, institutions, buffalo | done | `geography`, `institutions`, `bison` |
| modular, unit-tested, observable; lab tool and benches | done | `debug`, `bin/lab.rs`, `benches/` |
| gritty/elegant/timeless art, Dylan nods, greens and river blue | done | `main.rs`, `scenery.rs` |
| moon, ghosts, spooky lore where historical | moon, ghosts done; folklore pending | `calendar`, `ghosts`; see **Folklore** |
| Underground Railroad | done | `railroad` |
| player paths (romance, family, tycoon, preacher, souse, silver tongue, real estate, baptisms, ruffian, vagabond...) | done | `life`, `land`, `romance` |
| Sims × Stardew × Jones × RDR2-camp loops; careers | done (sim) | `life`, `farmwork`, `civic`, `bees` |
| vengeance only if justified; shame | done | `ghosts` |
| reconciliation: mercy, altruism, coincidence, enemy of my enemy, social engineering | done | `reconcile` |
| seduction, infidelity, blackmail, sabotage, generosity, humility, childhood mortality | done | `romance`, `intrigue`, `psyche`, `mortality` |
| absence: out fishing when your cousin dies | done | `family` |
| town life, bureaucracy, community building | done | `civic`, `law`, `mail`, `bees` |
| farm calendar, trapping, night camp, post office, court, voting, muster, bees, church split, kids inherit paths, press about you, scandal cuts credit, real estate + Panic, births | done | see module table in CLAUDE.md |
| seasons, weather, smoke, night, woodcut sprites, broadsheet, more Dylan | done | `scenery.rs`, `tools/woodcut.py` |
| constructive/destructive environment | first pass done | `homestead`, `geography` (stumps, scorch) |
| clean UI; closer and wider views; situational styles like FF7; kill the button wall | done (Phase A) | `county.rs`, `claim.rs`, `town.rs`, `walk.rs`, `scene.rs`, `ui.rs` |
| action and violence, and how to avoid it; shows of force; stealth; assassinations; minigames | **next** | **Phase B** |
| stockpiles, weapons, ammo, materials, crafting | planned | **Phase C** |
| assign tasks to family/faction/hired hands; mercenaries; bounty hunters (be one or run from one) | planned | **Phase D** |
| voice: 1850s humor, self-aware, Deadwood-ornate, ~75% Milch / 25% Heidecker–Turkington | planned | **Phase E** |
| language quirks that spread through families and friends and evolve with the world | planned | **Phase E** |
| a portion of Crusader Kings 3: schemes, dynasty, traits and stress, emergent narrative | planned | **Phase F** |
| folklore: omens, will-o'-wisps, spiritualism, wakes | planned | **Phase G** |

## Phase A — the view, rebuilt (FF7-style scales and scenes) — DONE

Shipped: county/claim/town screens, walking and spots, blue command
windows, scenes, top bar, Tab status, N paper, night on foot. Left for
later: collisions, tweened transitions between screens, a sprite for each
kin by name, lamps in town windows.

Goal: the player lives in places, not in a button panel.

1. **Screens as Bevy `States`**: `County` (the map, zoom and pan with the
   wheel and drag), `Homestead` (your claim up close, walkable), `Town`
   (Lawrence, Franklin, Lecompton street scenes), `Raid` (Phase B).
2. **Homestead**: 32px local tiles; the avatar walks (WASD); hotspots —
   cabin, barn, fields (plowed/sprouting/tall/stubble by season), fence
   (broken segments = `1 - fences`), rick, creek, woods (stumps as they're
   cut), road, build pads. Kin wander. Walk up and press E: a command window.
3. **Towns**: a street of engraved facades; doors are hotspots. Lawrence:
   church, hotel (ruin after May 1856), post office, barbershop, land agent,
   lyceum. Franklin: Dunmore's, the Jack of Hearts. Lecompton: land office,
   justice, polls on election day.
4. **FF7 command windows**: blue gradient box, bone border, a pointer
   cursor, arrows + Enter or mouse. The only verbs shown are what the thing
   in front of you offers today (reuse `available`).
5. **Scenes** (situational, letterboxed, big engraving, clock paused): the
   knock at the door, Dunmore's petition, the muster call, election day, a
   bee tonight, a letter, a death at home, riders at the gate (Phase B).
6. **HUD**: a thin top bar (date, weather, moon, spirits, cash, food); Tab
   opens a full-screen status menu (household, life, market, county,
   nations, the paper); N opens the broadsheet.
7. Needs: facade and tree/stump sprites in `tools/woodcut.py`; a camera per
   screen (one camera, repositioned; `Projection::Orthographic` scale).

## Phase B — action, and how to avoid it

1. **Standoffs** (sim hook): when a `Retaliation` against the player comes
   due and the player is home, the sim parks it as `pending_standoff`
   instead of resolving it. The scene offers:
   - *Talk them down* — oratory, their temper, your standing; success cools
     the grudge and clears the plot.
   - *Show of force* — courage, marksmanship, arms on hand (Phase C), kin
     and hired men at your side (Phase D); success puts fear in them.
   - *Draw* — a quick-draw timing game; your reaction vs. their
     marksmanship. Win: they're wounded (or dead, if you aim to kill).
     Lose: you are.
   - *Back down* — they do what they came to do; you keep your life.
   Player-initiated: *Confront* a neighbor from the county or town.
2. **Stealth raids** (`Raid` screen): the target's claim at night, from
   the same layout generator as the homestead. Lantern cones from the cabin,
   a dog on a round, the household's waking eyes (alertness); cone size
   scales with moonlight. Crawl (Shift) to shrink your profile. Objectives:
   the barn (burn), the well (foul), the pen (shoot a cow / drive one off),
   the fence (cut), the stack (wet), the window (listen: learn a secret).
   Anyone who sees you becomes a true `Witnessed` belief; nobody seeing you
   means the victim reads it through attribution as usual.
3. **Ambush**: lie in wait on a road at night; a steadiness/aim minigame;
   hit quality decides wounded vs killed; a bright moon makes witnesses.
4. The papers still decide whether you were a patriot or a terrorist.
   No bombings: arson, ambush, raids and cannon were the violence here.

## Phase C — stockpiles and crafting

- Goods: rifles (Sharps — "Beecher's Bibles", shipped in crates marked as
  books), shotguns, powder, lead, caps, cartridges, rails, shingles, iron.
- Crafting at the cabin: cast balls from lead, roll paper cartridges,
  split rails, shave shingles, mend harness.
- **Arms caches**: hide rifles in the loft or under the floor; a search
  (after a raid, a muster, a warrant) can find them → a charge.
- Shows of force, standoffs and musters scale with arms on hand.

## Phase D — hands, hired guns, warrants and bounties

- **Assign** kin to tasks (fields, stock, woods, watch the road at night:
  a sentry lowers raid success against you).
- **Hire hands** by the month (wages; loyalty; they gossip).
- **Mercenaries**: Buford's Southerners and Missouri men; Lane's Free-State
  companies. Rent them for a show of force or a raid; they cost money and
  they talk, and the other side's paper finds out.
- **Warrants**: violent acts that are believed (not proven) can produce a
  territorial warrant; a bounty goes up. Posses and bounty hunters come
  looking. Run (leave the county for a while; the farm suffers), hide, fight,
  or turn yourself in before the pro-slavery justice.
- **Be one**: take warrants for wanted men (the Pottawatomie killers had
  prices on their heads); ride with a posse. No slave-catching career: the
  railroad's turn-in choice is where that darkness already lives.

## Phase E — voice and speech

- **Voice pass** on chronicle, scenes and dialogue: ornate, profane-poetic
  frontier diction with a self-aware wink; about three parts Milch to one
  part deadpan Heidecker–Turkington. Original lines only, no quotes.
  Update the CLAUDE.md style rule ("period-plain and short" becomes
  "period-ornate and short").
- **Idiolects that spread**: each person carries a few turns of phrase
  (oaths, sayings, pet words). Phrases pass along opinion and proximity —
  kin, friends, pew-mates, drinking partners — and mutate with events (a
  fire, a hanging, a revival, the Panic). Phrases also carry faction
  (a Free-Stater picking up a Missourian's oath is noticed). Used in
  gossip lines, scene dialogue, and as a lens (`lab speech`).

## Phase F — a portion of Crusader Kings 3

- **Schemes** with progress and discovery: seduce, befriend, sway, murder,
  fabricate a claim, turn a hired man. Agents (kin, friends, hands)
  contribute; secrecy vs. power.
- **Hooks**: secrets and favors as spendable leverage (strong/weak), unified
  with blackmail, debts to Dunmore, and favors owed after barn raisings.
- **House legacy**: the Ashby name accrues renown and infamy across
  generations (children who come of age already inherit oaths, shame and
  paths); a heir-and-succession moment when you die.
- **Traits and stress**: acting against temperament (a pious man shooting a
  neighbor's cow) builds stress; stress breaks into drinking, rage,
  melancholy — each already a mechanic.
- **Emergent narrative lens**: `lab story` stitches cascade trees into
  short chapter summaries per family.

## Phase G — folklore (from task #11)

- **Omens**: an owl near the sick, a dog howling — the credulous grieve
  early, fear rises; the skeptical shrug.
- **Will-o'-wisps**: a light on the bottoms at the dark of the moon; follow
  it and sometimes find a lost cow, a cache, or a grave.
- **Wakes**: a gathering at the house of the dead; neighbors (even
  enemies) come, drink, talk; gossip and reconciliation run hot.
- **Spiritualism**: 1850s séances; a fraudulent medium's "messages from the
  dead" push beliefs about who killed whom.

## Known issues to fix along the way

- Fires ~12 per seed-two-years; grievance peaks over 100 in the first
  winter; hay runs short in March a lot.
- Map labels overlap the smaller buildings.
- The Phase 2 gate counts only accusations over thefts (fire feuds swamp
  the winter signal); revisit with more seeds.
