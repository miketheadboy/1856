# Bleeding Kansas — build log and plan

A running record of everything asked for, what exists, and what comes next.
Section refs (§) point at the master plan and `PART_I_EXTENSIONS.md`.

## Where it stands

- 38 sim modules, 162 sim tests + 5 view tests, clippy/fmt clean, two simulated years in ~25 ms.
- Balance (50 seeds, two years): a war nobody started in ~18/50, ~1 violent
  death per seed-year, harsh winters measurably worse (Phase 2 gate).
- The Bevy view: three scales (county map, your claim on foot, town
  streets), command windows on things, letterboxed scenes, a status screen
  and the paper. Phase B's action games: standoffs at the gate, raids at
  night, ambush on the road. Phase C's armories, the bench, crates of
  "books" and posse searches. Phase D's doombringers: hands and the night
  watch, companies for hire, the justice's papers, posses and bounty
  hunters. Next: Phase E, voice.

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
| action and violence, and how to avoid it; shows of force; stealth; assassinations; minigames (fun, skill based, a little twitchy) | done (Phase B) | `action`, `duel.rs`, `raid.rs` |
| stockpiles, weapons, ammo, materials, crafting | done (Phase C) | `arms`, the bench in `duel.rs` |
| assign tasks to family/faction/hired hands; mercenaries; bounty hunters (be one or run from one) | done (Phase D) | `hands`, `warrant` |
| voice: 1850s humor, self-aware, Deadwood-ornate, ~75% Milch / 25% Heidecker–Turkington | **next** | **Phase E** |
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

## Phase B — action, and how to avoid it — DONE

Shipped: plots against you park at the gate (`action::park`) when you're
home; four answers, three of them played by hand. *Talk down* reads his
mood off him (grief, rage, fear, greed) over three timed rounds; *face him
down* is a needle to hold in a calm zone whose width is your courage, kin
and powder, against shoves sized by his anger and his riders; *draw* is a
reaction test against his hand (0.30–0.75 s) and then a swaying aim (high
kills, low wounds; whiskey sways you). A failed talk or stare goes to guns.
*Confront* rides to their door. *Raids*: lantern cones sweep from the ones
sitting up, sleepers get up on a rhythm, a dog walks its round and chases,
crawl to shrink, hold E to burn/foul/shoot/drive/cut/wet/listen; whoever's
light holds you becomes a true Witnessed belief (staged into perception),
everyone else guesses. *Ambush*: riders cross the road after hoofbeats;
Shift holds your breath; at a dark moon you can't tell who's who, and at a
bright one they can see you. Misses are `ShotAt`, still an attack, and they
may shoot back. `lab standoff` shows the odds. Left for later: the cow you
drive off following you on screen, sounds, arms on hand (Phase C) and
hired men at your side (Phase D) in the stare.

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

## Phase C — stockpiles and crafting — DONE

Shipped: every house has an armory (Sharps, old guns, lead, balls,
cartridges, rails) and a hiding place. The bench is a timing minigame (a
needle and a moving sweet spot; knack widens it): cast balls from lead,
roll cartridges from balls and powder, split rails from timber. Rails mend
fence three times as fast. Every shot in a draw or an ambush spends a
round; no round, no shot. Rifles steady your sights, stiffen your nerve,
and lean harder on you when they're in someone else's saddle. Send east
from the Lawrence post office for a crate of books ($20): due on a
Wednesday, seized at Lexington landing in a blockade. The Emigrant Aid men
ship rifles to bold Free-State houses in 1855–56; Missouri arms the other
side. Three days after a pro-slavery muster the posse searches Free-State
houses; the hiding place decides what it finds (buried guns are safe and
useless at the gate). `lab arms` shows it. Left for later: shotguns as a
separate kind, caps, harness.

- Goods: rifles (Sharps — "Beecher's Bibles", shipped in crates marked as
  books), shotguns, powder, lead, caps, cartridges, rails, shingles, iron.
- Crafting at the cabin: cast balls from lead, roll paper cartridges,
  split rails, shave shingles, mend harness.
- **Arms caches**: hide rifles in the loft or under the floor; a search
  (after a raid, a muster, a warrant) can find them → a charge.
- Shows of force, standoffs and musters scale with arms on hand.

## Phase D — the doombringers: hands, hired guns, warrants and bounties — DONE

Shipped:
- `hands`: kin and hired hands take one job each (fields, stock, timber,
  the night watch). A watch turns riders back at the fence before they reach
  the gate, and cows them for a season; neighbors in a feud post one too,
  which brought fires down from ~34 to ~30 per two years. Hands come by the
  month ($12) from hungry households, wages to their people; an unpaid hand
  walks and talks, and a sour one of the other side tells where the guns are
  (a search finds them as if over the door until you move them). Companies:
  the Kickapoo Rangers, Buford's Southerners, the Stubbs, Lane's Army of the
  North, each in the county when it was, and only for its own side. Rent one
  to sit on your place three nights, or to ride on a neighbor's tonight
  (barn, else stock); nobody sees your face, and the other side's paper may
  print who paid.
- `warrant`: complaints from beliefs (an eyewitness is an oath, a loss most
  of one, a neighbor's word a little; the paper nothing; one household one
  voice), weighted by the justice's side. Enough and he writes a paper, with
  a bounty. Riders go out: the sheriff's posse, or a greedy man with a steady
  hand if the price is worth it. The accused comes in, runs to the States, or
  fights (shoot the law and the price doubles, and the shooting is its own
  crime). Trials at Lecompton; convicted, jail and costs. Held men don't
  plot, muster, vote or work. Blood spilled serving a paper writes no paper.
- You: a scene when word comes; give yourself up (tried that day), lie low a
  week in the timber, light out for the States six weeks, or meet them at the
  gate (the same standoff game; stand aside and it's Lecompton). Or take a
  paper at the justice's and ride after the man (paid on delivery, half for a
  body), or ride with the sheriff's men for a share.
- Lenses: `lab law` (every paper, and the truth beside it), `lab hands`
  (a watch and a hand against nobody), the survey's law line, `explain_npc`.
- Survey (30 seeds, two years): 24/30 wars from nothing, 4.3 violent deaths,
  30 fires, 5.6 papers per seed, ~80% on the wrong man, ~1 shot serving them.

Left for later: neighbors hiring companies, a hand's own grudges, the
federal marshal's papers after Geary, and the view's walk to the man's door.

The plan, as written:

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
