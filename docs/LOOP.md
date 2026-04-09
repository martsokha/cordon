# Core Gameplay Loop

## The Day Cycle

Each in-game day is one "session." The player's day follows this structure:

```
MORNING          MIDDAY              EVENING             NIGHT
─────────────────────────────────────────────────────────────────
Prepare stock    Customers arrive    Last deals          Manage bunker
Check prices     Buy / Sell / Trade  Runners return      Upgrade, plan
Send runners     Handle events       Faction visits       End-of-day report
Read intel       Negotiate           Resolve threats
```

### 1. Morning: Preparation

- Review your inventory and stash
- Check the market ticker on your laptop (price fluctuations, shortages, rumors)
- Dispatch runners to specific sectors to scavenge or deliver goods
- Set prices for the day (or leave them dynamic)

### 2. Midday: Trading

The core of the game. Scavengers arrive one at a time (like Papers, Please).

For each customer:
- They approach your counter with a request (buy, sell, or barter)
- You see their appearance, faction patch, gear, demeanor
- You can: **accept**, **counter-offer**, **refuse**, or **negotiate**
- Some bring gossip, warnings, or jobs
- Some are lying, desperate, or dangerous

**Trading mechanics:**
- Drag items to/from the counter
- Adjust prices with sliders or manual input
- Read NPC mood/trust indicators
- Optional: inspect items for quality

### 3. Evening: Consequences

- Runners return (or don't) with scavenged goods
- Faction representatives may visit — demands, threats, offers
- Random events resolve: raids, surges, supply drops
- NPCs you helped or screwed may return with consequences

### 4. Night: Management

- Upgrade your bunker (see BUNKER.md)
- Review the day's profit/loss
- Read intercepted documents or intel
- Plan tomorrow's strategy
- Save/continue

## Economic Loop

```
BUY LOW ──→ HOLD ──→ SELL HIGH
   ↑                      │
   │    RISK / EVENTS      │
   │    can destroy stock   │
   └──────────────────────┘
```

- Prices are driven by: base value, supply in your area, demand from scavengers, faction control, zone events
- Hoarding is risky — raids can destroy stock, surges can corrupt relics
- Moving goods through runners adds time and danger but accesses better markets

## Reputation Loop

```
DEAL ──→ NPC REMEMBERS ──→ WORD SPREADS ──→ NEW OPPORTUNITIES / THREATS
```

- Fair dealing builds trust → better prices, tips, loyal customers
- Gouging/scamming builds profit → but erodes trust, invites hostility
- Faction alignment opens doors but closes others
