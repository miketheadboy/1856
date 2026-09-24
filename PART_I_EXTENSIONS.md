# PART I — THE SUBSTRATE (EXTENSIONS)

---

## §14 — TEMPORAL DRIFT (DESYNCHRONIZED REALITY)
Reality in the simulation does not update uniformly. Different systems “learn” about the world at different speeds, and they do not reconcile automatically.

The result is a world where multiple versions of the same event coexist.

### 14.1 The principle

> There is no single “current state.” There are overlapping, lagging interpretations of state.

---

### 14.2 System update clocks

| System | Update cadence | Notes |
| --- | --- | --- |
| Direct witnesses | Immediate | Raw perception (often wrong, §11) |
| Gossip network | Daily | Mutates during spread |
| Institutions (church, tavern) | Weekly | Reinforces narratives |
| Newspapers | Discrete publication | Freezes a version permanently |
| Legal system | Delayed / batch | May be wrong or outdated |
| Faction stance | Weekly–monthly | Aggregated, slow-moving |

---

### 14.3 Divergence model

```text
Event occurs at T0

T0: Witnesses form beliefs (biased, incomplete)
T+1 day: Gossip spreads mutated versions
T+3 days: Tavern amplifies a dominant interpretation
T+7 days: Church reinforces moral framing
T+10 days: Newspaper prints version (locks it)
T+14 days: Legal ruling contradicts or lags
```

---

### 14.4 Consequences

- An NPC can be:
  - Legally innocent
  - Socially guilty
  - Factionally useful as a villain
- Clearing your name in one system does not propagate
- Old narratives persist because:
  - Institutions reinforce them
  - Newspapers fossilize them
  - Memory is sticky (§14, §13)

---

### 14.5 Player-facing outcome

- “Proving innocence” is not a solution, it is one vector in a multi-system conflict
- Timing becomes strategic:
  - Act before a narrative stabilizes
  - Or bury it under a larger event (§15)

---

## §15 — NARRATIVE SCARCITY (ATTENTION AS A RESOURCE)
Information does not just spread. It competes.

Communities can only sustain a limited number of dominant narratives at once. New events must displace existing ones to matter.

---

### 15.1 The attention cap

```text
Settlement {
    active_narratives: PriorityQueue<Event>  // max size N (e.g. 5–10)
}
```

Each narrative has:

```text
priority =
    severity
    + recency
    + faction amplification
    + institutional reinforcement
```

---

### 15.2 Replacement rule

```text
if new_event.priority > weakest_existing:
    replace weakest
else:
    event is ignored / forgotten
```

---

### 15.3 Consequences

- A murder during a cholera outbreak may never “exist” socially
- A minor theft during a quiet period becomes defining
- The player can:
  - Bury actions under larger events
  - Or be destroyed by bad timing

---

### 15.4 System interactions

- Works directly with:
  - §11 Attribution Engine (which narratives survive)
  - §12 Fire (large events overwrite smaller ones)
  - §8 Disease (high-priority narrative generator)

---

## §16 — IDENTITY FRAGMENTATION (CONTRADICTORY SELVES)
NPCs are not coherent actors. They contain misaligned internal systems that resolve differently under pressure.

---

### 16.1 The three-layer model

```text
NPC {
    public_ideology     // what they say
    private_belief      // what they think
    survival_drive      // what they do
}
```

---

### 16.2 Action resolution

```text
action = weighted_sum(
    ideology,
    fear,
    incentives,
    recent_trauma,
    social_pressure
)
```

Weights shift dynamically.

---

### 16.3 Consequences

- “Hypocrisy” emerges naturally:
  - Anti-slavery NPC reports fugitives under threat
  - Betrayal does not require special traits
  - Loyalty is unstable under stress

---

### 16.4 System interactions

- §10 (resource scarcity) increases survival weighting
- §8 (disease, trauma) shifts decisions toward fear
- §11 (attribution) misreads contradictions as malice

---

## §17 — MORAL MOMENTUM (PATH DEPENDENCE)
Actions change the probability of future actions.

The first violation is difficult. The second is easier. The third is normal.

---

### 17.1 Resistance decay

```text
if action A is taken:
    resistance[A] -= delta
    justification_bias[A] += delta
```

---

### 17.2 Memory reinforcement

- NPCs reinterpret past behavior to align with current identity
- “I had to” becomes “it was right”

---

### 17.3 Consequences

- The player does not choose a role—they drift into one
- Atrocities are not spikes—they are trajectories

---

### 17.4 Interaction with §13 (ratchets)
Moral momentum acts as a psychological ratchet:

- Not irreversible
- But increasingly costly to reverse

---

## §18 — STRUCTURAL FALSEHOODS (THE WORLD LIES)
Not all incorrect beliefs come from bias. Some come from incorrect data produced by the world itself.

---

### 18.1 Sources of false data

- Store ledgers (miscounted, manipulated)
- Census records (missing or duplicated people)
- Weather reporting lag
- Distance/time misperception
- Witness error (visibility, stress)

---

### 18.2 Mechanic

```text
ObservedData = Truth × NoiseFactor
```

Noise increases with:

- Stress
- Distance
- Low visibility (§7 sightlines)
- Time delay

---

### 18.3 Consequences

- Even a rational attribution system (§11) produces incorrect conclusions
- “Truth” may never exist in any accessible form

---

## §19 — INVERTED DEPENDENCIES (POLITICS SHAPES PHYSICS)
Not all causality flows upward from the substrate. Political decisions can reshape the substrate itself.

---

### 19.1 Examples

- Ban on controlled burns →
  → vegetation buildup →
  → catastrophic fire risk (§12)
- Overhunting as faction signaling →
  → ecological collapse →
  → famine (§10)
- Land policy →
  → settlement clustering →
  → disease spread (§8)

---

### 19.2 Consequences

- Ideology produces physical consequences
- Bad policy creates delayed disasters
- Players may not connect cause and effect

---

## §20 — SILENCE AS SIGNAL
Absence is observable and meaningful.

---

### 20.1 Tracked absences

- Not attending church (§9)
- Avoiding tavern
- Not traveling known paths (§7.1)
- Breaking routine

---

### 20.2 Mechanic

```text
if expected_presence && absence:
    suspicion += delta
    social_trust -= delta
```

---

### 20.3 Consequences

- Doing nothing becomes risky
- Hiding creates evidence
- Safety behaviors generate suspicion

---

## §21 — ASYMMETRIC LEGIBILITY
The player and NPCs perceive different layers of reality.

---

### 21.1 Player advantages

- Sees patterns across systems
- Understands probabilities
- Recognizes long-term drift

---

### 21.2 Player blind spots

- Cannot see private beliefs (§16)
- Cannot access true cause (§11)
- Cannot fully predict narrative dominance (§15)

---

### 21.3 Consequences

- The player is not omniscient
- Only differently informed
- Mistakes feel earned, not random

---

## §22 — EMOTIONAL CONTAGION
Emotion spreads through populations similarly to disease.

---

### 22.1 Model

```text
Emotion {
    type: FEAR | ANGER | GRIEF | ZEAL
    spread_rate ∝ proximity × social ties × recent events
}
```

---

### 22.2 Effects

- Fear → risk-averse decisions
- Anger → retaliation probability ↑
- Grief → productivity ↓, volatility ↑
- Zeal → ideology hardening

---

### 22.3 Consequences

- Violence is not required for instability
- Emotional states create second-order events

---

## §23 — SYNTHESIS: REALITY AS SURVIVOR
This section formalizes the interaction between:

- §11 Attribution Engine
- §14 Temporal Drift
- §15 Narrative Scarcity

---

### 23.1 The principle

> Reality is not what happened. It is what survived long enough to be believed.

---

### 23.2 Process

```text
Event occurs
→ Multiple interpretations generated (§11)
→ Interpretations spread with mutation
→ Compete for attention (§15)
→ Reinforced by institutions (§9)
→ Locked by publication (§14)
→ Persist as memory (§13)
```

---

### 23.3 Result

- The world converges on a version of reality
- That version may be wrong
- It becomes functionally true

---

### 23.4 Player implication

- You cannot control truth
- You can only influence:
  - timing
  - visibility
  - narrative competition
  - institutional amplification

---

## Final note (design-level, not in-doc)
These sections don’t add systems—they destabilize the ones you already built.

That’s the difference between:

- a simulation that runs
- and a simulation that turns on itself
