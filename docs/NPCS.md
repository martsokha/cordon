# NPCs & Interaction

## NPC System

Every scavenger who walks up to your counter is a character, not just a transaction. NPCs have persistent state that evolves based on your interactions.

### NPC Attributes

```
NAME:        Viper                    [visible]
FACTION:     The Collective           [visible — patches, gear, mannerisms]
RANK:        Soldier                  [visible — gear quality, demeanor, how others address them]
GEAR:        Worn leather jacket,     [visible — what they're carrying]
             damaged rifle
TRUST:       Low                      [hidden — built over repeat interactions]
WEALTH:      Poor                     [hidden — inferred from gear/offers]
NEED:        Desperate (wounded)      [hidden — may or may not be obvious]
PERSONALITY: Cautious, honest         [hidden — revealed through behavior over time]
PERKS:       ???                      [hidden — revealed through actions/missions]
```

**Visible attributes** (what the player sees in the interface):
- **Name** — their name or alias
- **Faction** — who they belong to. Every NPC has a faction, including Drifters. Read from patches, gear, mannerisms.
- **Rank** — their standing within their faction. Affects what they can offer, what they'll accept, and how their faction reacts if you wrong them. A grunt and a lieutenant are very different customers.
- **Gear** — what they're carrying and wearing. Gives clues about wealth, competence, and faction.

**Hidden attributes** (player never sees these directly — only infers through behavior):
- **Trust** — how much they trust you personally. Built over repeat interactions.
- **Wealth** — how much they can actually afford. Not always obvious — a ragged Drifter might have a fortune stashed.
- **Need** — what they actually need vs. what they say they want. A desperate person may hide their desperation.
- **Personality** — affects haggling style, honesty, patience. Emerges over multiple encounters.
- **Perks** — special traits, revealed through actions (see below).

### Rank

Rank matters. It tells you who you're dealing with.

| Rank | Description | Gameplay effect |
|------|-------------|-----------------|
| **Grunt** | Bottom rung. Runs errands, does grunt work. | Small trades, low funds, faction doesn't care much if you screw them |
| **Regular** | Reliable member. Trusted with real tasks. | Standard trades, moderate funds, faction notices mistreatment |
| **Veteran** | Experienced, respected. Has pull within the faction. | Better goods, larger deals, faction reacts strongly to disrespect |
| **Officer** | Leadership. Speaks with faction authority. | Can offer faction deals/contracts, carries faction resources, wronging them = faction incident |
| **Elite** | Inner circle. Named characters. | Unique items, faction-level negotiations, killing/robbing one is an act of war |

Drifters and Mercenaries use looser rank equivalents (Rookie → Seasoned → Hardened → Veteran → Legend).

The Devoted use religious ranks (Pilgrim → Acolyte → Keeper → Prophet → Ascended).

### Hidden Perks

NPCs (and especially potential hires) have **perks** — traits that are invisible until revealed through actions. You don't get a stat sheet when someone walks in. You learn who they are by working with them.

**How perks are revealed:**
- A runner completes a mission → you discover they have "Scavenger's Eye" (finds better loot)
- A runner barely survives a dangerous sector → you discover they have "Hard to Kill"
- An NPC sells you a fake relic → you realize they have "Silver Tongue"
- A runner skims goods from a haul → you discover they have "Sticky Fingers"
- A guard stands firm during a raid → you discover they have "Ironwall"

**Example perks:**
| Perk | Type | Effect | Revealed when... |
|------|------|--------|-------------------|
| Scavenger's Eye | Positive | Better loot quality on scavenge missions | They return with unusually good finds |
| Hard to Kill | Positive | Significantly higher survival rate | They survive a mission that should have killed them |
| Pathfinder | Positive | Faster mission completion, avoids hazards | They return early from a dangerous route |
| Haggler | Positive | Gets better prices when doing deliveries/trades | A delivery earns more than expected |
| Ghost | Positive | Excels in hostile faction territory | They succeed in a sector you expected them to fail |
| Ironwall | Positive | Exceptional at guard duty. Raids are less effective. | A raid that should have broken through gets repelled |
| Intimidating | Positive | Better intimidation outcomes when on guard duty | NPCs back down more easily during negotiations |
| Sticky Fingers | Negative | Skims a % of goods from hauls | You notice inventory doesn't match expected returns |
| Coward | Negative | Higher chance of aborting dangerous missions / fleeing during raids | They come back empty-handed from a risky run, or vanish during a raid |
| Big Mouth | Negative | Leaks your business to factions/rivals | A faction learns something they shouldn't know |
| Grudge Holder | Negative | If mistreated, will eventually betray you | Sudden betrayal after a history of bad treatment |
| Lucky | Neutral | Occasional jackpot finds, occasional disasters | Unpredictable mission outcomes |

**Design intent:** You're hiring people, not stat blocks. You have to invest in someone (pay, gear, missions) before you know what you're getting. This creates attachment and tension — your best runner might have a hidden flaw you haven't discovered yet.

### NPC Types

| Type | Description | Frequency |
|------|-------------|-----------|
| **Drifters** | Independent scavengers, varied needs | Very common |
| **Faction Soldiers** | Buy/sell on behalf of faction, rank varies | Common |
| **Job Seekers** | NPCs looking for work — potential runners or guards | Occasional |
| **Faction Reps** | Officers or elites delivering demands, offers, ultimatums | Periodic |
| **Scammers** | Try to sell you fakes or rob you | Occasional |
| **Desperate** | Wounded, starving, broke — moral test | Occasional |
| **Informants** | Sell intel, rumors, tips | Rare |
| **Special** | Story NPCs, quest givers, unique encounters | Scripted |

## Recruitment & Hired Hands

You can employ up to **2 people** at any time — split however you want between runners and guards. They're paid daily. Hiring is not an upgrade; it's an ongoing expense.

**Who you can recruit:** Only Drifters, Syndicate members, and Mercenaries. Faction-aligned NPCs from the Order, Collective, Garrison, Institute, or Devoted won't abandon their cause to work for a trader. You're hiring the unaffiliated, the opportunistic, and the for-hire.

- Drifters are cheap but unpredictable — you have no idea what you're getting
- Syndicate types are street-smart but may have loyalty issues or be running from something
- Mercenaries are competent but expensive — they know their worth and won't tolerate bad deals

### How recruitment works

1. **NPC arrives looking for work** — they show up at your counter asking if you need anyone. You see their appearance, demeanor, and gear condition — but no stats, no perks.
2. **You decide** — hire them, turn them away, or negotiate terms (pay rate, upfront equipment).
3. **You can also offer jobs** — if a regular Drifter, Syndicate, or Mercenary customer seems capable, you can propose they work for you. Trust level affects whether they accept.
4. **Assign a role** — runner (goes into the Zone) or guard (stays at the bunker).
5. **Trial by fire** — their first mission or first raid reveals what they're actually made of. Perks emerge over time.

### What you know at hire time

- Their name and appearance
- Their faction — visible from patches, gear, mannerisms
- Their rank — gives a rough sense of competence
- What gear they're carrying (gives hints — a well-armed NPC probably knows how to fight)
- What they *say* about themselves (which may or may not be true)

### What you DON'T know

- Their actual perks (hidden)
- Their loyalty threshold (how much abuse they'll tolerate)
- Whether they're a plant from a rival faction
- Whether they'll crack under pressure or rise to the occasion

### Roles

**Runners** — your hands in the Zone. They go out, scavenge, deliver, and gather intel. Your primary way to acquire goods beyond what walks through your door.

**Guards** — your muscle at the bunker. They deter theft, enable intimidation during negotiations, and defend against raids. A guard with the right perks can make your bunker nearly untouchable — but you won't know until a raid tests them.

You can reassign roles between days. Firing someone is instant but burns trust — they may come back as a hostile customer, or badmouth you to their faction.

## Runners

### Sending Runners

Each morning you can dispatch available runners:

1. **Choose a runner** from your roster
2. **Choose a destination** — sectors vary in risk and reward. Reachable sectors depend on your Radio/Antenna upgrade level.
3. **Choose a mission type:**
   - **Scavenge** — bring back whatever they find
   - **Targeted search** — look for a specific item category (costs more, less total loot)
   - **Delivery** — bring goods to a buyer in another sector (guaranteed sale, transit risk)
   - **Recon** — gather intel on a sector (no loot, but information)
4. **Equip them** — better gear = better survival odds. You pay for what they carry.

### Mission Outcomes

Runners return in the evening (or after 1-2 days for distant sectors):

| Outcome | Base Chance | Result |
|---------|-------------|--------|
| **Success** | 60% | Full loot, runner returns healthy |
| **Partial success** | 20% | Some loot, runner may be wounded |
| **Failure** | 10% | No loot, runner wounded or lost gear |
| **Runner lost** | 5% | Runner doesn't return. Presumed dead. Gear lost. |
| **Jackpot** | 5% | Exceptional find — rare relic, stash, intel |

Chances are modified by the runner's hidden perks, their equipment, the sector's danger level, and current zone events. You won't know the exact odds — you're making gut calls based on what you've observed.

### Sector Destinations

| Sector | Risk | Reward | Radio Required | Notes |
|--------|------|--------|----------------|-------|
| **The Threshold** | Low | Low | Level 1 | Safe, mostly picked clean. Good for new runners. |
| **The Scrapyard** | Low-Med | Low-Med | Level 1 | Syndicate territory. Stealth helps. |
| **The Hollows** | Medium | Medium | Level 2 | Syndicate stronghold. Dangerous but good loot. |
| **The Crossroads** | Low | Medium | Level 2 | Safe hub. Good for intel and trade. |
| **The Tangles** | Medium | High | Level 3 | Hazard-rich. Relics common. |
| **The Depot** | High | High | Level 3 | Garrison and Collective clash. Great gear. |
| **The Deep Woods** | Very High | Very High | Level 4 | Deep Zone. Rare relics. Many don't return. |
| **The Core** | Extreme | Extreme | Level 5 (Relay Network) | Endgame sector. Legendary finds. Near-suicidal. |

### Runner Management

- **Pay** — runners expect payment. Underpay and they leave or steal.
- **Equipment** — you can give runners weapons, armor, meds, and detectors from your inventory.
- **Loyalty** — invisible stat. Builds over time with fair pay and good equipment. Drops with underpayment or sending them on suicide missions.
- **Permadeath** — dead runners are gone. Their gear is gone. Your investment is lost.

## Guards

Guards stay at the bunker and provide:

- **Raid defense** — guards fight off attackers. More/better guards = better outcomes. Without a guard, thugs can rob you freely.
- **Intimidation** — during negotiations, having a guard enables the Intimidate action. The guard's hidden perks affect how effective it is.
- **Deterrence** — NPCs with hostile intent are less likely to try something if they see armed guards.

Guards use the same attribute and perk system as runners. A guard with "Ironwall" is worth two without it — but you won't know until a raid happens. A guard with "Coward" might flee when you need them most.

Guards need daily pay and equipment, same as runners. They can be wounded or killed during raids.

## Strategic Depth

Your 2 hire slots create constant tension:
- 2 runners, 0 guards = maximum scavenging, zero protection
- 1 runner, 1 guard = balanced, but half the scavenging output
- 0 runners, 2 guards = fortress mode, but no field income
- Your best runner gets wounded — do you fire the guard to hire a replacement, leaving you unprotected?
- A Mercenary veteran shows up looking for work, but both slots are full. Do you fire someone?

## Negotiation System

When a scavenger approaches, you enter a negotiation:

### Player Actions

1. **View offer** — see what they want to buy/sell and at what price
2. **Counter-offer** — adjust price, swap items, add conditions
3. **Inspect** — check item quality (if you have the tools)
4. **Read NPC** — observe body language, gear, faction tells
5. **Accept** — take the deal
6. **Refuse** — send them away (has consequences)
7. **Intimidate** — force a better deal (risky, depends on your guards and their rank)
8. **Gift** — give something for free (builds trust, costs money)
9. **Offer job** — propose they work for you as a runner or guard (if trust is sufficient, faction allows it)

### NPC Reactions

NPCs respond dynamically:
- **Grateful** — you gave a fair or generous deal → trust up, may return with better goods
- **Satisfied** — standard deal → neutral outcome
- **Resentful** — you gouged them → trust down, may badmouth you
- **Hostile** — you refused or insulted them → could escalate (threats, theft, faction complaint — severity depends on their rank)
- **Desperate acceptance** — they took a bad deal because they had no choice → moral weight

### Haggling Flow

```
NPC ARRIVES
    │
    ▼
PRESENTS OFFER ◄──────────────────┐
    │                              │
    ▼                              │
PLAYER RESPONDS                    │
  ├─ Accept ──→ DEAL DONE         │
  ├─ Refuse ──→ NPC REACTS        │
  ├─ Counter ──→ NPC CONSIDERS ───┘
  ├─ Intimidate ──→ RISK CHECK
  ├─ Inspect ──→ REVEAL INFO
  └─ Offer Job ──→ RECRUITMENT
```

## Reputation & Memory

NPCs remember:
- Whether you gave them fair deals
- Whether you helped them when desperate
- Whether you sold them fakes or junk
- Whether you sided with their faction's enemies

This memory persists and spreads:
- Direct memory: "You charged me double for bandages"
- Word of mouth: "I heard you sell good weapons" (reputation)
- Faction reports: "Our scouts say you're trading with the Collective"

## Recurring Characters

Some NPCs become regulars with mini-storylines:
- The rookie who keeps coming back, slowly getting better gear
- The thug who's secretly trying to leave his gang
- The researcher who needs increasingly dangerous relics
- The deserter hiding from his unit

These create narrative threads that weave through the trading gameplay.
