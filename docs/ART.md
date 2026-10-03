# Art, sound and look: the delivery guide

This guide covers four pieces of work. Three go into the game (sound,
figures, look); the last is the trailer, made outside it:

1. **The score.** Four stems; the county mixes them.
2. **The figures.** Drawn parts on the rig's skeleton.
3. **The look.** One printed-matter pass over the whole frame.
4. **The plates.** Blender Grease Pencil engravings for the scenes.
5. **The trailer.** Later.

The engine side of 1–4 is built. Drop files into `assets/` and they show up
in the game; nothing needs code. Anything missing falls back to what's
there now. Sections 3 and 4 hold the creative choices; 1 and 2 are mostly
delivery contracts.

---

## The one idea under all of it

The game's thesis: nobody sees causes, only effects, and history is whatever
got printed. So the game should look like **what got printed**. That is
not a nostalgia filter. It is the argument, made in the form.

Every choice below comes back to one question: *is this how the county
would have been put on paper in 1856, by someone who wasn't there?*

- Harper's Weekly and Frank Leslie's ran engravings of Kansas made in New
  York from correspondents' letters.
- The artists drew what they were told, composed for drama, and got the
  details wrong.

That is the eye these images should have.

---

## 1. The score (Studio One → `assets/audio/`)

**Files:** these four names, each exactly the same number of bars, 32 is
good. All the same tempo, key and length. Bounce from bar 1, with no
reverb tail cut off at the loop point: let the tails live inside the 32
bars.

```
assets/audio/stem_drone.ogg
assets/audio/stem_folk.ogg
assets/audio/stem_fuzz.ogg
assets/audio/stem_drums.ogg
```

Export OGG Vorbis, 44.1k, quality 6 or above, stereo.

**How the game mixes them** (`src/audio.rs`). Each stem eases about four
seconds to its target; nothing ever cuts.

| stem | level | so write it as |
|---|---|---|
| drone | 0.55 always, up to 0.8 on a moonless night | the floor: Jupiter-8 pad or bowed open-tuned guitar, J37 on the bus; it must sound fine alone |
| folk | full in a quiet county, gone as tension passes ~0.8, quieter at night | dry fingerpicked Ample guitar plus fiddle, small-room IR, Carver-plain |
| fuzz | comes in from tension 0.45, full by 0.85 | doubled fuzz, panned L/R, 12 ms offset, Abbey Road Saturator; it should feel like weather arriving |
| drums | in at the edge of war (tension 0.8–1.1), or up to 0.8 for a week after any killing | SD3 room mics crushed with CLA-76 all-buttons, in parallel under the dry kit |

**What that means for writing it.** Any combination of the four has to
work, including folk plus drums with no fuzz (a killing in a quiet
county). Write the drums so they make sense over the folk part. That
pairing, a funeral beat under a front-porch tune, is the most Bleeding
Kansas sound in the game.

**Testing:** run with `BK_MUTE=1` to silence it.

---

## 2. The figures (Illustrator → `assets/rig/`)

**The contract:**
- One PNG per part, drawn **standing up**.
- The part's first joint sits at the **top center** of the image, its
  second joint at the **bottom center**.
- The game hangs the image between those two joints and takes the width
  from the image's own proportions.
- Draw it in black ink with **white fill**. The game tints the white to the
  outfit's color (coat, trousers, hat), and the ink stays black. A colored
  fill would break the wardrobe, and the witnesses who read your shirt
  color.
- Export at 4×, transparent background.

| file | top joint → bottom joint | note |
|---|---|---|
| `upper_arm_l`, `upper_arm_r` | shoulder → elbow | `_l` is the far arm, drawn behind |
| `forearm_l`, `forearm_r` | elbow → wrist | sleeve included |
| `hand_l`, `hand_r` | wrist → fingertips | mitten hands; fingers suggested, not drawn |
| `thigh_l`, `thigh_r` | hip → knee | |
| `shin_l`, `shin_r` | knee → ankle | |
| `boot_r` | ankle top → sole | toe pointing **screen-left** |
| `boot_l` | ankle top → sole | toe pointing **screen-right** |
| `torso` | neck → pelvis | the coat; a shirt strip is drawn over it |
| `skirt` | waist → hem | replaces the legs for women |
| `neck` | collar → jaw | |
| `head` | crown → chin | men; hat sits on top |
| `head_woman` | crown → chin | used with the skirt |
| `hat` | crown top → brim | the **whole** hat, brim included; replaces both slabs |
| `rifle` | muzzle → butt | the whole gun, drawn vertical; tinted light wood |
| `belt`, `holster`, `knife` | as named | small; can wait |

**Order to draw in:** torso, head, upper arm, forearm, hand, thigh, shin,
boot. With those eight the man at the gate is drawn. Anything missing
stays a slab, so you can check each part in the game as it lands:
`BK_PLAY=gate cargo run`.

**Style:**
- Pencil tool, not pen: keep the wobble.
- Hatch the shadow side at 45°, with the same spacing as
  `tools/woodcut.py`, so the figures match the buildings.
- 3–4 pt outline, 1 pt interior lines.

---

## 3. The look: the printed sheet (`assets/look.txt`)

One pass runs over everything on screen: map, sprites, menus and type. It
runs after the UI is drawn, so the ink sits on the whole page.

- **Where it lives:** the numbers are in `assets/look.txt`. The game
  re-reads the file within a second of a save, so tune it with the game
  running, the way you'd tune an adjustment layer against a held frame.
- **Turning it off:** `BK_LOOK=0`.

### The decision that shapes everything: rag paper, hand-colored

There are two wrong roads, and it matters which one we're not on.

- **Not risograph.** Riso is the default "lo-fi print" look right now, and
  it reads as 2020s zine. Wrong century.
- **Not "old yellowed newspaper".** 1850s American papers were printed on
  **rag paper**: cotton, strong, close to white. The brown, brittle,
  yellow newsprint everyone pictures is wood pulp, which arrived in the
  1870s. A heavy sepia says 1890 and says "old". We want *then*, not
  *old*.

What 1856 actually looked like:
- **Black letterpress and wood engraving** on near-white rag.
- **Color added by hand.** A colorist's brush or stencil, the way Currier
  & Ives prints were colored by a line of women at tables. Color in this
  period was not printed in register. It was applied, and it missed.

So the look has two layers: a black key that is crisp, and accents
(oxblood, brass) that wander off the line. Every knob serves one or the
other.

### The knobs

Defaults first, then the range worth trying, and what each one says.

**Paper**

| knob | default | range | what it is |
|---|---|---|---|
| `grain` | 0.14 | 0.08–0.25 | tooth of the paper |
| `grain_size` | 1.6 | 1–3 | px per grain cell |
| `boil_fps` | 8 | 6–12 | how often the grain changes |
| `fiber` | 0.07 | 0–0.12 | long rag fibers, faint horizontal streaks |

- **`grain`:** lives in the midtones and paper, almost none in the solid
  blacks (ink is ink).
  - Below 0.1, the frame goes digital-clean.
  - Above 0.25 it reads as film. Wrong medium: the camera wasn't there,
    the printer was.
- **`grain_size`:** 1 is fine paper; 3 is the cheap job-printer stock the
  Squatter Sovereign would have used.
- **`boil_fps`: the most important creative call in this file.** At 24–60
  fps the grain looks like video noise, a TV. At 6–12 it looks like the
  page holds still and *breathes* in printed frames, the way hand-drawn
  animation boils.
  - Keep it at or below 12.
  - Match the plates' boil (6 fps) if you want the whole screen to share
    one heartbeat.
- **`fiber`:** subtle by design. Past 0.12 it turns to linen, and linen
  reads as a texture pack.

**Ink and color**

| knob | default | range | what it is |
|---|---|---|---|
| `bleed` | 0.15 | 0.05–0.25 | dark ink spreading a pixel into the paper (letterpress squash) |
| `misreg` | 1.5 | 0.5–3 | px the hand-colored accents miss the line |
| `halftone` | 0.0 | 0 (see below) | screen dots in the shadows |

- **`bleed`:** keep it low.
  - Measured on the claim screen: at 0.35 the hatching on the tree crowns
    closes up into grey slabs, and the engraving dies.
  - The fine hatch is the whole charm of the sprites. Protect it.
- **`misreg`:**
  - Only the warm, saturated colors move: oxblood, brass, a red shirt.
    Greens, river blue and ink stay put.
  - It reads as a colorist's brush, not a misaligned plate.
  - More than 3 px and it becomes a glitch effect. Glitch is digital and
    wrong.
- **`halftone`:**
  - **Keep it at 0 in normal play.** Halftone photoengraving is an 1880s
    technology.
  - That anachronism is a tool (see "The epilogue" below).

**Age and light**

| knob | default | range | what it is |
|---|---|---|---|
| `desat` | 0.12 | 0.08–0.18 | saturation below "correct" |
| `sepia` | 0.22 | 0.1–0.3 | warmth in the greys and whites |
| `vignette` | 0.35 | 0.2–0.45 | the edges of the sheet |

- **`desat`:** lived-in, not drained. Above 0.2 the oxblood stops reading
  as blood.
- **`sepia`:** think lamplight and tobacco smoke, not age. Rag paper
  doesn't yellow; the room does.
- **`vignette`:**
  - It starts well out from center so the HUD text at the top stays
    legible.
  - Above 0.45 the top bar's date and money go muddy. Check the HUD
    after every change.

**The county moves these**

| knob | default | range | what happens |
|---|---|---|---|
| `tension_misreg` | 1.5 | 0.5–3 | extra px off the line at full tension |
| `tension_grain` | 0.08 | 0.03–0.12 | extra grain at full tension |
| `night_vignette` | 0.3 | 0.1–0.4 | extra vignette on a moonless night |

- **`tension_misreg`:** as the county frays, the colorist's hand gets
  worse. Nobody will notice consciously. Everyone will feel the image come
  apart as the county does. The **form degrading with the content** is the
  Kendrick move: the system under the surface. Don't push it past +3.
- **`tension_grain`:** the paper gets cheaper in a crisis. A blockade
  year: rag was scarce, so the stock got coarse.
- **`night_vignette`:** lamplight at the center of the sheet, dark at the
  edges.

### Four moods to tune toward

Set each one up in the game and save a screenshot as a reference. When
the four feel like one newspaper on four different days, you're done.

1. **A quiet May afternoon** (tension low, full moon tonight).
   - The baseline. It should look almost clean, the print barely there.
   - Restraint is the point: the look has to *earn* its moments.
2. **The Sack of Lawrence** (tension at max).
   - Color off the line, coarse paper, but still a newspaper.
   - If it looks like a horror filter, back off `tension_misreg`.
3. **A moonless night at your door.**
   - Vignette deep, grain in the little light there is.
   - The knock should feel like reading by a lamp.
4. **The epilogue** (not built yet; see below).

### The epilogue: a deliberate anachronism

When the player dies, or 1857 ends, the closing screens could switch to
`halftone 0.4, sepia 0.35, misreg 0`. That is a photoengraved page from the
1880s, the county's story **retold by a later paper**:
- clean, registered, mechanical;
- confident, and wrong.

It's the Teraoka collision: the right technique from the wrong decade, used
to say *this is history now, which means it's been rewritten*.

This needs a few lines of code: one preset swap on the end scene. Say the
word.

### Tuning in After Effects, if you'd rather

1. Take screenshots with
   `BK_SEED=15 BK_START_DAYS=200 BK_SCREEN=claim cargo run` (and `county`,
   `lawrence`, `BK_PLAY=gate`, `BK_PLAY=door`). Run them **with
   `BK_LOOK=0`** so you get the clean frame.
2. Build the look as one rig on your MASTER_CTRL, then read its sliders
   back into `look.txt`:

   | in AE | knob in `look.txt` |
   |---|---|
   | S_Grain amount / size | `grain` / `grain_size` |
   | Posterize Time on the grain layer | `boil_fps` |
   | a Minimum (1 px) layer blended at opacity | `bleed` |
   | a key on the warm hues, offset with Transform | `misreg` |
   | Hue/Sat master saturation, as a fraction | `desat` |
   | a Tint layer at opacity | `sepia` |
   | S_VignetteMask amount | `vignette` |

3. The shader's math is close to those effects, not identical. Do a last
   pass in the game.

### What not to add, and why

- **Chromatic aberration across the whole frame:** that's a lens. No lens
  was there.
- **Scanlines, VHS, glitch:** wrong medium.
- **Film burns, gate weave:** film is wrong too. This is paper.
- **Motion blur:** engravings freeze the moment. That's the point of them.

---

## 4. The plates: engravings for the scenes (Blender → `assets/plates/`)

When the county forces a moment on you (the knock, the riders, the muster,
the grave), the frame letterboxes and a scene card comes up. Today that
card is a small sprite. A plate fills the frame behind the card:
- a slow 6% push in over 24 seconds, drifting up and left;
- the line boiling if you give it frames;
- tinted by the light of the moment.

### The creative brief: the engraving that ran in the paper

Every plate is **the cut a New York weekly would have run of this moment,
made by an artist who wasn't there.** That one rule decides a lot:
- **Composed for drama**, from a distance, slightly wrong.
  - Harper's and Leslie's engravings of Kansas have Missourians who all
    look alike and settlers who all look noble.
  - Lean into that generality.
- **No faces on the people who did it.** The rider at the gate is a hat,
  a coat and a torch. The player never sees truth, and neither did the
  engraver. Faces are for victims and witnesses, who the papers *did*
  draw.
- **The moment just before or just after, never the act.** The door
  before it opens; the barn after it's burned. Carver: what's left out is
  the subject.
- **Reduction.** A plate is Charley Harper's few shapes plus an
  engraver's hatching, not a painting.
  - The silhouette must read at a glance behind a menu.
  - If you have to look to see what it is, cut shapes.

### The sixteen plates

Each scene looks for its own file. These are listed in priority order;
the first five carry the game.

1. **`riders_torch`: riders at the gate, with a torch.**
   - Two or three mounted silhouettes beyond your rail fence, one torch
     held high. All the light comes from the torch and only touches hats
     and horse necks. Moonlit.
2. **`riders_rifle`: riders at the gate, rifles.**
   - Same as above, no torch, rifles across the saddles; the moon behind
     them, so they're shapes.
3. **`knock`: a knock after dark.**
   - The inside of your door, closed, a lamp on a stool, a wet bootprint
     under the door. Never show who's outside. Lamplit.
4. **`law`: the law at the gate.**
   - A man on foot holding a paper up, posse behind, all faceless; the
     paper is the brightest thing in the frame. Moonlit.
5. **`grave`: a death in the house.**
   - A fresh mound, a board cross, prairie to the horizon, one figure
     with a shovel walking away. Cold.
   - The hatching can go McBess-dense here, and nowhere else.
6. **`door`: you at their door.**
   - Their house, their step, their door opening a crack: a sliver of
     light and a hand on the frame. You are the camera.
7. **`bounty`: you at their door, with a paper.**
   - As `door`, but the paper in your hand in the foreground, big. You
     are the law now, and the composition should feel it.
8. **`fallen`: down in the dirt.**
   - A hat in the grass and a boot. That's all. Restraint is the only
     decent choice.
9. **`muster`: the muster is called.**
   - A line of men with rifles at the gate, one holding a list. Repeated
     figures, the Haring-style rhythm of a crowd as a pattern.
10. **`election`: election day.**
    - A window with a ballot box on the sill. Through it, a crowd too big
      for the county: the Missourians.
11. **`petition`: Dunmore's petition.**
    - The store counter: a ledger, a pen, a paper, Dunmore's hands only.
    - Cornellà contrast: the sweetest, tidiest plate in the game, for the
      ugliest bargain.
12. **`letter`: a letter.**
    - A folded letter on the table by a lamp, a black border if it's bad
      news. (The plate can't know which; draw it plain.)
13. **`bee`: a bee tonight.**
    - A barn interior, lanterns, a fiddler, dancers as Mucha-ish ornament
      rhythms. The one warm plate. Let it be warm.
14. **`paper`: a paper with your name on it.**
    - A wanted bill nailed to a post, the name not legible (it's yours;
      the player knows).
15. **`end`: the end.**
    - The claim from a distance, empty, the cabin roofless. The county
      goes on. Cold.
16. **`wagon`: the wagon to the river.**
    - Reserved for the freight scene, not wired yet. An ox team on the
      Westport road, small under a big sky.

### The tint (`src/scene.rs` `plate_tint`)

The game colors the white of the plate by the light of the moment. Draw
in black and white and let the code light it:

| plates | light |
|---|---|
| knock, riders, law, door, fallen | moonlight blue (0.74, 0.80, 0.92) |
| letter, bee, petition | lamplight amber (1.00, 0.88, 0.70) |
| grave, end | cold grey (0.80, 0.80, 0.78) |
| the rest | paper (0.95, 0.90, 0.80) |

If a plate needs its own light, tell me and it gets a line.

### The frame you're drawing into

- **Canvas:** 1920 × 1080, PNG, RGB. Ink black on **white**; no gray
  wash. Value comes from hatching density, as in a real engraving.
- **Letterbox:**
  - The top and bottom 96 px (at 1080) are covered by black bars.
  - Leave them empty or let them bleed; nothing important there.
- **Title band:**
  - 96–230 px from the top carries the scene title in white type with a
    shadow.
  - Keep it to sky or plain wall; no detail fighting the words.
- **The card:**
  - The choice box sits center-bottom, roughly x 570–1350 and y 640–915.
  - **Put the subject in the left or right third, upper two-thirds.**
  - The classic engraving layout of subject off-center, sky over it, is
    exactly what the frame needs.
- **The push:**
  - The plate scales up 6% toward the upper left over 24 seconds.
  - Keep 60 px of non-essential margin on every edge so nothing important
    slides off.

### Grease Pencil, the fast way for someone who isn't a draftsman

- **The trick: don't draw the contours, model them.**
  - Block the scene out in plain 3D boxes and cylinders (a cabin is a box
    and a prism, a rider is a capsule on a box).
  - Add a **Grease Pencil Line Art** modifier and let Blender draw the
    engraving's contour lines for you.
  - In AE terms: it's Auto-trace on a 3D precomp, live.
- **Hatching:** draw it by hand on a GP layer, or fill the GP shapes with
  a hatch-texture material made from `tools/woodcut.py` output. Hand
  hatching on the important shapes and texture fill on the rest is the
  real engraver's split anyway: the master cut the faces, the apprentices
  cut the sky.
- **Camera:**
  - A long lens, 85–135 mm, for the compressed, slightly flat perspective
    engravings have.
  - No depth of field: engravings are in focus edge to edge.
  - Ortho is too flat; 100 mm is right.
- **Render:**
  - EEVEE.
  - **Color Management → View Transform: Standard.** *Not* AgX or Filmic:
    they grey the whites and the tint will look muddy. This is the gotcha
    that'll cost you an afternoon.
  - World color pure white. Film not transparent.
- **The boil (optional, worth it for the top five):**
  - Add a **Noise** modifier to the GP object at low strength.
  - Render frames 1–3 with a different seed or offset each.
  - Save them as `riders_torch_1.png`, `riders_torch_2.png`,
    `riders_torch_3.png`.
  - The game cycles them at 6 fps. Up to 8 frames work; 3 is the classic
    hand-drawn boil.

### What not to do

- **No color in the plate.** The tint is the color; the accents belong to
  the UI. A colored plate under a tinted pass turns to mud.
- **No photographic textures.** A scanned wood grain at the bottom of a
  plate reads as Photoshop instantly. Hatch it.
- **No dramatic lighting from Blender lamps.** Line Art doesn't care, and
  baked shading fights the hatch. Light is drawn, as hatching density.

### Checking a plate in the game

```bash
BK_PLAY=door cargo run     # 'door' plate: you at their door
BK_PLAY=gate cargo run     # riders, through the standoff
```

The other scenes come up as they happen: the knock, the muster, the vote,
a letter. `BK_START_DAYS` jumps the calendar, and `docs/screenshots/`
shows what each scene looks like today, without plates.

---

## 5. The trailer (later)

The game can shoot itself:

```bash
BK_PLAY=gate|door|raid|ambush|posse|paper BK_LOOK=0 cargo run
```

- Grab clean frames and comp them in AE through the same look rig as §3.
- For type, use the survey-plat rig (your surveillance-UI rig retuned:
  plat lines, section numbers).
- The copy is already written: the chronicle says things like *"a fever
  took him, not the ball"*. Pull lines from
  `cargo run --bin headless -- --seed 15`.
