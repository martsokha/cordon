# Factions

## Overview

Factions are the political ecosystem of the Zone. Your relationship with each faction affects who comes to trade, what prices you get, how safe your bunker is, and which opportunities open or close.

You cannot join a faction — you are a trader. But you can align, betray, play sides, or stay neutral. Each choice has real consequences.

## Faction Profiles

### The Order

**Philosophy:** The Zone is a threat to civilization. It must be contained and destroyed.  
**Structure:** Military discipline, hierarchy, order.

| Aspect | Detail |
|--------|--------|
| **They buy** | Weapons, ammo, military gear, intel on creatures |
| **They sell** | Military surplus, rations, med supplies |
| **Good relations** | Stable supply of ammo/weapons, protection from thugs, military contacts |
| **Bad relations** | Trade embargo, armed "inspections," confiscation of goods |
| **They hate** | The Collective, creature sympathizers, relic hoarders |

### The Collective

**Philosophy:** The Zone belongs to everyone. Information and relics should be free.  
**Structure:** Loose, democratic, chaotic.

| Aspect | Detail |
|--------|--------|
| **They buy** | Relics, experimental tech, recreational supplies, propaganda materials |
| **They sell** | Relics, sector maps, tips on hazard fields |
| **Good relations** | Access to rare relics, Zone intel, relaxed atmosphere |
| **Bad relations** | They stop sharing intel, may steal from you, spread bad word |
| **They hate** | The Order, the Garrison, anyone who restricts Zone access |

### The Syndicate

**Philosophy:** Take what you can. Trust no one.  
**Structure:** Loose gangs, strongman leaders.

| Aspect | Detail |
|--------|--------|
| **They buy** | Weapons, spirits, anything they can flip |
| **They sell** | Stolen goods (cheap but risky — may be tracked), contraband |
| **Good relations** | They leave you alone, sell stolen goods cheap, offer "protection" |
| **Bad relations** | Raids on your bunker, robbery, kidnapping your runners |
| **They hate** | The Order, the Garrison, anyone who won't deal with them |

### The Garrison

**Philosophy:** The Zone is a restricted area under state control.  
**Structure:** Official military hierarchy, but deeply corrupt.

| Aspect | Detail |
|--------|--------|
| **They buy** | Intel, relics (for research/black market), food/comfort items |
| **They sell** | Military-grade weapons, armor, vehicle access, safe passage |
| **Good relations** | Access to restricted sectors, military-grade supplies, "official" protection |
| **Bad relations** | Raids, arrest, confiscation, blockade of supply lines |
| **They hate** | Everyone in the Zone (officially), but can be bribed |

### The Institute

**Philosophy:** Study the Zone. Understand it. Protect it for science.  
**Structure:** Academic hierarchy, funded by outside organizations.

| Aspect | Detail |
|--------|--------|
| **They buy** | Relics (pay top dollar), creature samples, lab equipment, documents |
| **They sell** | Scientific equipment, detectors, relic containers, analysis services |
| **Good relations** | Best relic prices, relic authentication, Zone forecasts (surge warnings) |
| **Bad relations** | Refuse to buy your relics, no more warnings, cold shoulder |
| **They hate** | Nobody strongly, but distrust the Syndicate and anyone who damages relics |

### Drifters

**Philosophy:** No philosophy. Just survive.  
**Structure:** No structure — loose community of independent scavengers.

| Aspect | Detail |
|--------|--------|
| **They buy** | Everything — food, meds, ammo, cheap weapons, whatever they can afford |
| **They sell** | Whatever they find — junk, scrap, the occasional lucky relic |
| **Good relations** | Steady stream of customers, word-of-mouth reputation boost, runner recruitment pool |
| **Bad relations** | They avoid your shop, warn others away, no recruitment candidates |
| **They hate** | Nobody in particular, but resent anyone who exploits the desperate |

### Mercenaries

**Philosophy:** Money talks. Everything else is noise.  
**Structure:** Professional outfits, contract-based. Disciplined but loyal only to the paycheck.

| Aspect | Detail |
|--------|--------|
| **They buy** | High-end weapons, armor, tactical gear, intel on targets |
| **They sell** | Looted gear from contracts, classified intel, "acquired" relics |
| **Good relations** | Access to elite gear, hired muscle for bunker defense, contract work (they retrieve specific items for you) |
| **Bad relations** | They take contracts against you, sell your intel to rivals |
| **They hate** | Nobody inherently — but cross them on a deal and they take it professionally (which is worse) |

### The Devoted

**Philosophy:** The Zone is sacred. It has a will. Those who listen are chosen.  
**Structure:** Fanatical, cult-like. Hierarchical around prophets and elders.

| Aspect | Detail |
|--------|--------|
| **They buy** | Relics (hoard them as holy objects), protective gear, ritual supplies |
| **They sell** | Rare and dangerous relics, Zone secrets, items from deep sectors no one else reaches |
| **Good relations** | Access to the rarest relics, deep-Zone intel, safe passage through Devoted-controlled territory |
| **Bad relations** | Violent hostility, sabotage, they send zealots to "reclaim" relics you've sold |
| **They hate** | The Institute (blasphemers who dissect the sacred), anyone who destroys relics |

## Faction Dynamics

### Relationship Web

```
       THE ORDER
        /    \
   oppose    oppose
      /        \
COLLECTIVE     GARRISON
   |    \      /    |
  ally  shaky fear  fund
   |      \ /      |
DRIFTERS  SYNDICATE  INSTITUTE
               |
            rival
               |
         MERCENARIES ── neutral ── most factions

THE DEVOTED ── hostile ── INSTITUTE
    |
  feared by ── everyone else
```

### Faction Standing Scale

```
-100 ──────── -50 ──────── 0 ──────── +50 ──────── +100
HOSTILE      UNFRIENDLY   NEUTRAL    FRIENDLY     ALLIED
Kill on      Bad prices   Default    Good prices  Best prices
sight        Threats      Normal     Protection   Exclusive
             Raids        trade      Intel        missions
```

### Shifting Alliances

Factions react to your choices:
- Sell weapons to the Collective → Order standing drops
- Report Syndicate locations to the Garrison → Syndicate standing drops sharply
- Supply the Institute with relics → Institute standing rises, Collective approves, the Devoted despise you
- Sell secret documents → whoever you sell to loves it, whoever is in the documents hates you
- Deal with Mercenaries → nobody cares much, unless the mercs were hired against someone you're friendly with
- Trade relics to the Devoted → they love you, the Institute cuts ties

### Faction Events

Factions generate events that affect gameplay:
- **Faction war**: two factions fight, supply lines disrupted, refugee customers
- **Faction patrol**: a faction "checks" your inventory for contraband
- **Faction quest**: a faction asks you to acquire something specific
- **Faction blockade**: a faction cuts off a trade route, certain goods become scarce
- **Faction coup**: leadership change, standing partially resets, new priorities
- **Mercenary contract**: mercs are hired to hit a target near you; collateral risk, or an opportunity
- **Devoted pilgrimage**: zealots move through sectors en masse; dangerous but they carry rare relics
