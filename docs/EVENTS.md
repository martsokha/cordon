# Zone Events

## Overview

The Zone is unpredictable. Events happen on a semi-random schedule, driven by a combination of timers, player actions, and narrative triggers. Events create pressure, opportunity, and variety.

## Event Types

### Environmental

| Event | Effect | Duration |
|-------|--------|----------|
| **Surge** | All outdoor activity halted. Runners in the field may die. NPCs seek shelter (potential customers). Relics shift/appear. | 1 day |
| **Blowout** | Severe surge. Major casualties. Relic positions completely reset. Prices spike across the board. | 1 day, aftermath 2-3 days |
| **Creature Swarm** | Dangerous creatures flood a sector. Runners at extreme risk. Weapon/ammo demand spikes. Some sectors inaccessible. | 2-3 days |
| **Hazard Shift** | Hazard fields move. New relic locations. Old safe routes become deadly. Maps become outdated (unless you have map subscription). | Permanent until next shift |
| **Psi-Wave** | Psychic disturbance in a sector. Scavengers go erratic. Unpredictable NPC behavior. Mental protection gear demand surges. | 1-2 days |

### Economic

| Event | Effect | Duration |
|-------|--------|----------|
| **Supply Drop** | The Garrison loses a convoy. Cheap military goods flood the market. | 2-3 days |
| **Shortage** | A category of goods becomes scarce (random or triggered). Prices spike. | 3-5 days |
| **Black Market Bust** | Authorities crack down. Contraband prices skyrocket. Risky to hold illegal goods. | 2-4 days |
| **New Route** | A safe path opens to a previously dangerous sector. New goods become available. | Until next hazard shift |
| **Trader Rivalry** | Another trader undercuts your prices. Customers mention cheaper alternatives. | Ongoing until resolved |

### Faction

| Event | Effect | Duration |
|-------|--------|----------|
| **Faction War** | Two factions clash. Their soldiers buy weapons/meds urgently. Collateral damage possible. | 3-7 days |
| **Faction Truce** | Two hostile factions temporarily cooperate. New trade opportunities. | 5-10 days |
| **Coup / Leadership Change** | A faction's leader changes. Standing partially resets. New trade priorities. | Permanent |
| **Faction Mission** | A faction gives you a specific task (acquire X, refuse to sell to Y). Reward or punishment. | Timed (2-5 days) |
| **Faction Patrol** | Faction sends soldiers to "inspect" your bunker. Confiscate contraband (unless hidden in secret compartment). | 1 day |
| **Mercenary Contract** | Mercs are hired to hit a target near you. Collateral risk, or an opportunity to profit from the aftermath. | 2-3 days |
| **Devoted Pilgrimage** | Zealots move through sectors en masse. Dangerous but they carry rare relics. Trade carefully — they're volatile. | 3-5 days |

### Bunker

| Event | Effect | Duration |
|-------|--------|----------|
| **Raid** | Thugs or hostile faction attacks your bunker. Outcome depends on guards and defenses. Can lose stock, credits, runners. Barricades and reinforced door reduce severity. | 1 day |
| **Inspection** | The Garrison or the Order "checks" your goods. Contraband is confiscated unless you have a secret compartment and alarm system to hide it in time. | 1 day |
| **Power Outage** | Laptop goes down, refrigeration fails (food spoils faster), electronic upgrades offline. Generator upgrade prevents this entirely. | 1-2 days |
| **Visitor** | Someone asks for shelter during a surge. Help costs resources (food, meds, space). Refusing costs reputation. Helping may gain a loyal customer or runner candidate. | 1 day |
| **Infestation** | Vermin or creatures get into storage. Items damaged. Worse without climate control. | 1 day |
| **Sabotage** | Someone tampers with your equipment — radio jammed, locks picked, stock poisoned. Could be a faction retaliation or Syndicate play. Alarm system gives warning. | 1 day |
| **Break-In Attempt** | Someone tries to rob you overnight. Reinforced door stops casual attempts. Guards stop serious ones. Without either, you lose items from storage. | 1 day |

### Personal

| Event | Effect | Duration |
|-------|--------|----------|
| **Runner Lost** | One of your runners goes missing. Wait, search, or write them off. | Until resolved |
| **Betrayal** | A trusted NPC steals from you or feeds you false intel. | One-time |
| **Debt Collector** | Someone you owe (or who claims you owe) comes to collect. | Until resolved |
| **Wounded Stranger** | A scavenger collapses at your door. Help costs resources. Refusing costs reputation. | 1 day |
| **Old Friend** | An NPC from the past returns with a unique opportunity or request. | 1-3 days |

## Event System Design

### Scheduling

Events are generated using a weighted random system:

```
Each day:
  1. Roll for environmental event (20% base chance, modified by Zone instability)
  2. Roll for economic event (15% base chance, modified by market stability)
  3. Roll for faction event (25% base chance, modified by faction tensions)
  4. Roll for bunker event (15% base chance, modified by security level and faction standing)
  5. Roll for personal event (10% base chance, modified by narrative flags)
  6. Apply any scripted/story events for this day
```

### Event Chains

Some events trigger follow-ups:
- Surge → Relic Rush (scavengers bring new finds) → Price Crash (if many relics flood market)
- Faction War → Refugee Customers → Faction Patrol (winners "clean up")
- Runner Lost → Search Mission → Discovery (good or bad)
- Raid → Damaged upgrades need repair → Shortage of supplies while you rebuild
- Inspection → Confiscation → Faction standing drop if you were hiding contraband for their enemies

### Player Agency

Players can't prevent events, but can prepare:
- Stockpile meds before predicted surges (laptop upgrade)
- Diversify inventory to weather shortages
- Maintain faction standings to avoid hostile events
- Invest in security to survive raids
- Generator prevents power outages
- Alarm system gives time to hide contraband before inspections
- Secret compartment keeps items safe during raids and inspections
- Intel network (upgrade) gives advance warning of some events

### Escalation

As days progress, events become more frequent and severe:
- Early game: mild weather, small trades, local drama
- Mid game: faction conflicts, economic swings, moral dilemmas
- Late game: Zone-wide crises, faction wars, existential threats, endgame scenarios
