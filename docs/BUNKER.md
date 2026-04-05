# Base: Bunker & Camp

## Overview

Your base has two parts:

- **Bunker**: the interior. This is where you trade, store goods, analyze items, and manage operations. You never leave.
- **Camp**: the area around the bunker. Defenses, antenna, watchtower, outdoor structures. Visible to anyone passing through — upgrades here affect how others perceive you.

Both start bare and are built up through upgrades.

## Upgrades

Upgrades are defined in config files. Each has prerequisites, a cost, and a source (purchasable, faction-gated, or quest reward). There are no hardcoded chains — sequential upgrades like `"radio_1"` → `"radio_2"` → `"radio_3"` are modeled by each upgrade requiring the previous one.

### Bunker: Laptop

| ID | Name | Source | Requires | Effect |
|----|------|--------|----------|--------|
| **laptop_1** | Cracked Screen | Purchasable | — | Basic price ticker. Yesterday's prices. |
| **laptop_2** | Patched Laptop | Purchasable | laptop_1 | Real-time prices. Basic zone news feed. |
| **laptop_3** | Modified Terminal | Purchasable | laptop_2 | Price predictions (1-day forecast). Faction activity tracker. |
| **laptop_4** | Network Hub | Faction (Institute) | laptop_3 | Intercept communications. Track runner locations. Access black market listings. |
| **laptop_5** | Command Center | Quest | laptop_4 | Zone-wide intel. Surge predictions. Faction movement forecasts. Remote trading. |

### Bunker: Counter & Trade

| ID | Name | Source | Requires | Effect |
|----|------|--------|----------|--------|
| **counter_1** | Wooden Plank | Purchasable | — | Basic buy/sell. No inspection. |
| **counter_2** | Proper Counter | Purchasable | counter_1 | Organized display. +5% NPC trust on first visit. |
| **counter_3** | Trader's Setup | Purchasable | counter_2 | Display cases for premium items. Reputation bonus. |
| **magnifying_lens** | Magnifying Lens | Purchasable | counter_2 | Detect obvious fakes (broken weapons, expired meds). |
| **relic_scanner** | Relic Scanner | Faction (Institute) | counter_2 | Detect fake relics. Identify relic stability. |
| **advanced_toolkit** | Advanced Toolkit | Purchasable | magnifying_lens | Detect all fakes. Full item analysis. |
| **geiger_counter** | Geiger Counter | Purchasable | — | Check radiation levels on items and people. |
| **scale** | Merchant's Scale | Purchasable | counter_2 | Weigh items for accurate pricing. Detect underweight ammo boxes. |
| **ledger** | Ledger | Purchasable | — | Track profit/loss per day. Trade history with NPCs. |

### Bunker: Storage

| ID | Name | Source | Requires | Effect |
|----|------|--------|----------|--------|
| **storage_1** | Stash Box | Purchasable | — | 20 item slots. No protection. |
| **storage_2** | Reinforced Locker | Purchasable | storage_1 | 40 slots. Basic raid protection. |
| **storage_3** | Warehouse Shelves | Purchasable | storage_2 | 80 slots. Weapon racks (condition preserved). |
| **fridge** | Fridge | Purchasable | — | Food spoils 2x slower. |
| **freezer** | Freezer | Purchasable | fridge | Food almost never spoils. Extends med expiry. |
| **relic_containment** | Containment Unit | Faction (Institute) | relic_scanner | Safe storage for relics. Prevents instability. |
| **secret_compartment** | Hidden Compartment | Purchasable | storage_2 | 10 hidden slots invisible during inspections. |
| **climate_control** | Climate Control | Purchasable | storage_3 | All items preserved at optimal conditions. |
| **relic_stabilizer** | Relic Stabilizer | Faction (Institute) | relic_containment | Stabilize unstable relics. |

### Bunker: Quality of Life

| ID | Name | Source | Requires | Effect |
|----|------|--------|----------|--------|
| **generator** | Diesel Generator | Purchasable | — | Reliable power. Prevents outages. Required for electronics. |
| **backup_generator** | Backup Generator | Purchasable | generator | Power survives raids and sabotage. |
| **cot** | Army Cot | Purchasable | — | Wounded runners/NPCs can rest here. Builds trust. |
| **lockbox** | Lockbox | Purchasable | — | Protects credits during raids. |
| **stove** | Gas Stove | Purchasable | — | Cook food, brew coffee. Slight morale bonus for visitors. |
| **medical_station** | Medical Station | Faction (Institute) | cot | Heal wounded squad members faster. |
| **workbench** | Workbench | Purchasable | — | Lets visiting mechanics work. Required for repair services. |

### Camp: Radio & Communications

| ID | Name | Source | Requires | Effect |
|----|------|--------|----------|--------|
| **radio_1** | Handheld Radio | Purchasable | — | Contact runners in nearby areas. Receive local news. |
| **radio_2** | Mounted Radio | Purchasable | radio_1 | Reach mid-range areas. Pick up faction broadcasts. |
| **radio_3** | Boosted Antenna | Purchasable | radio_2 | Reach far areas. Intercept some faction comms. Contact other traders. |
| **radio_4** | Signal Tower | Faction (Garrison) | radio_3 | Reach deep areas. Eavesdrop on encrypted channels. |
| **radio_5** | Relay Network | Quest | radio_4 | Full signal coverage. Contact anyone with a radio. |
| **signal_booster** | Signal Booster | Faction (Mercenaries) | radio_3 | Clearer signal. Fewer missed transmissions. Better runner tracking. |
| **scrambler** | Frequency Scrambler | Faction (Syndicate) | radio_2 | Encrypt your outgoing communications. Harder to eavesdrop on. |

### Camp: Defenses

| ID | Name | Source | Requires | Effect |
|----|------|--------|----------|--------|
| **reinforced_door** | Reinforced Door | Purchasable | — | Small raids fail. Buys time during larger raids. |
| **alarm_system** | Alarm System | Purchasable | — | Early warning. Time to hide contraband. |
| **barricades** | Barricades | Purchasable | reinforced_door | Only major assaults get through. |
| **sandbag_wall** | Sandbag Wall | Purchasable | — | Basic perimeter defense. Guards are more effective. |
| **watchtower** | Watchtower | Purchasable | sandbag_wall | See approaching threats earlier. Bonus to guard effectiveness. |
| **spotlight** | Spotlight | Purchasable | generator, watchtower | Illuminates camp at night. Deters break-ins. |
| **razor_wire** | Razor Wire | Purchasable | barricades | Slows raiders. Reduces raid damage. |
| **escape_tunnel** | Escape Tunnel | Purchasable | storage_2 | Emergency exit. If a raid overwhelms defenses, you survive and keep hidden storage. |

### Camp: Intel & Surveillance

| ID | Name | Source | Requires | Effect |
|----|------|--------|----------|--------|
| **zone_map** | Zone Map | Purchasable | — | Basic area map. Static — outdated after hazard shifts. |
| **map_board** | Map Board | Purchasable | zone_map | Pin notes and intel. Track runner locations visually. |
| **map_subscription** | Map Subscription | Purchasable | radio_2 | Daily map updates from other traders. Reflects hazard shifts. |
| **threat_tracker** | Threat Tracker | Faction (Garrison) | radio_3, map_board | Creature activity and hostile patrols on the map. |
| **deep_scanner** | Deep Scanner | Faction (Institute) | threat_tracker, laptop_4 | Creature migration patterns. Danger heatmap. |
| **intel_network** | Intel Network | Faction (Mercenaries) | radio_3, laptop_3 | Auto-bought intel reports. Daily briefing on everything. |
| **decryption_software** | Decryption Software | Faction (Institute) | laptop_3 | Decrypt encoded PDAs and intercepted messages. |
| **faction_dossiers** | Faction Dossiers | Purchasable | laptop_2 | Track faction standing in detail. |
| **listening_post** | Listening Post | Faction (Collective) | radio_4 | Overhear NPC conversations. Tips about incoming visitors. |
| **camp_camera** | Camp Camera | Purchasable | generator, laptop_2 | Live camera feed of your camp on the laptop. See who's approaching, what your guards are doing, and spot threats before they reach the door. |

### Camp: Infrastructure

| ID | Name | Source | Requires | Effect |
|----|------|--------|----------|--------|
| **fire_pit** | Fire Pit | Purchasable | — | Basic camp amenity. Runners rest here between missions. |
| **rain_shelter** | Rain Shelter | Purchasable | — | Protects outdoor equipment from weather. |
| **landing_pad** | Landing Pad | Faction (Garrison) | radio_3 | Enables supply deliveries. Access to Garrison trade goods. |
| **trade_sign** | Trade Sign | Purchasable | — | More NPCs notice your shop. Slightly increases visitor count. |
| **faction_flag** | Faction Flag | Faction (any) | standing 50+ | Fly a faction's flag. Increases visitors from that faction. Deters enemies. |
| **solar_panel** | Solar Panel | Faction (Institute) | generator | Reduces power outage events. Backup for backup generator. |
| **water_collector** | Water Collector | Purchasable | — | Free clean water supply. Reduces food/drink costs. |
| **dog** | Camp Dog | Quest | — | Warns of creatures, deters petty theft. Morale boost. |

## Upgrade Costs

Upgrades cost credits and prerequisites. No crafting. Some require faction standing or quest completion to even appear as an option.

Purchasable upgrades are always available in the upgrade menu if prerequisites are met. Faction and quest upgrades only appear after the source condition is satisfied.
