# Zone Events

## Overview

The Zone is unpredictable. Events are defined in JSON config files and rolled daily by the simulation. Each event has a spawn weight, duration range, and parameters. World state (escalation, faction tensions, security level) modifies probabilities at runtime. Events without a `spawn_weight` are quest-only — they never roll from the daily scheduler and only fire from quest consequences.

## Event Types

### Environmental

| Event | Effect | Duration |
|-------|--------|----------|
| **Surge** | All outdoor activity halted. Runners in the field may die. NPCs seek shelter (potential customers). Relics shift/appear. | 1 day |
| **Blowout** | Severe surge. Major casualties. Relic positions completely reset. Prices spike across the board. | 1 day, aftermath 2-3 days |
| **Creature Swarm** | Dangerous creatures flood a sector. Runners at extreme risk. Weapon/ammo demand spikes. Some sectors inaccessible. | 2-3 days |
| **Hazard Shift** | Hazard fields move. New relic locations. Old safe routes become deadly. Maps become outdated. | Permanent until next shift |
| **Intelligent Creature** | A creature of unusual intelligence appears in a sector. It avoids patrols, sets traps, and stalks runners. Extremely dangerous but carries or guards rare relics. Well-equipped runners may survive an encounter. Others don't come back. | 3-7 days |

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
| **Garrison Commander Visit** | The bribeable Garrison commander visits. Pay him off to reduce inspection frequency and get tip-offs about upcoming patrols. Refusing or underpaying worsens Garrison relations. | 1 day |
| **Garrison Inspector Visit** | The strict Garrison commander shows up instead of the bribeable one. Cannot be bribed. Conducts a thorough inspection. Bad news. | 1 day |

### Bunker

| Event | Effect | Duration |
|-------|--------|----------|
| **Raid** | Thugs or hostile faction attacks your bunker. Outcome depends on guards and defenses. Can lose stock, credits, runners. Barricades and reinforced door reduce severity. | 1 day |
| **Inspection** | The Garrison or the Order "checks" your goods. Contraband is confiscated unless you have a secret compartment and alarm system to hide it in time. | 1 day |
| **Power Outage** | Laptop goes down, refrigeration fails (food spoils faster), electronic upgrades offline. Generator upgrade prevents this entirely. | 1-2 days |
| **Visitor** | Someone asks for shelter during a surge. Help costs resources (food, meds, space). Refusing costs reputation. Helping may gain a loyal customer or runner candidate. | 1 day |
| **Infestation** | Vermin or creatures get into storage. Items damaged. | 1 day |
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
| **Information Seller** | A traveling information seller arrives. Sells intel on sector conditions, faction movements, upcoming events, and market shifts. Prices vary. Information may be outdated or false. | 1 day |

## Event System Design

Events are data-driven: each event is defined in a JSON config file with an optional spawn weight, duration range, eligible sectors/factions, earliest day, and chain events. The sim rolls daily for each eligible event that carries a spawn weight.

### Scheduling

Each day, the sim iterates over all event definitions:
1. Skip events without a `spawn_weight` (quest-only)
2. Check if the event is eligible (earliest day, stackability)
3. Compute probability: `spawn_weight × escalation_multiplier`
4. Roll. If it fires, create an `ActiveEvent` with a rolled duration and resolved parameters (which factions, which sector)

### Event Chains

Events can trigger follow-ups via `chain_events` in the config:
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
- Bribe the Garrison commander to reduce inspections

### Escalation

As days progress, event probabilities increase:
- Early game: mild weather, small trades, local drama
- Mid game: faction conflicts, economic swings, moral dilemmas
- Late game: Zone-wide crises, faction wars, existential threats, intelligent creatures
