# Cordon

A management/trading sim set in a quarantined exclusion zone. You run a bunker, trade with scavengers, hire squads, and navigate faction politics — all from behind your counter.

**Engine:** Bevy 0.18 (Rust)
**Platform:** Desktop (macOS, Windows, Linux)

## Premise

You are a trader operating out of a bunker deep in the Zone — a quarantined, anomaly-riddled wasteland. Scavengers come to you to buy, sell, and barter. You never leave — the Zone comes to you.

Your goal: survive, profit, and navigate the politics of the Zone's factions. Every deal you make ripples outward. Sell cheap meds to a wounded drifter? He might come back with rare relics. Gouge a soldier from the Garrison? His squad might "inspect" your bunker tomorrow.

## Core Fantasy

You are not the hero — you're the guy everyone needs. You sit behind your counter, size up whoever walks through the door, and decide what they're worth to you. The tension isn't in gunfights; it's in the deal.

## Inspirations

- **Papers, Please** — moral weight in mundane transactions
- **S.T.A.L.K.E.R.** — setting, tone, factions, lore
- **Darkest Dungeon** — risk management, sending others into danger

## Running

```bash
cargo run                          # default (with Steam features)
cargo run --no-default-features    # without Steam
```

### Debug keys

| Key | Action |
|-----|--------|
| F1 | Toggle world inspector |
| F2 | Push test visitor |
| F3 | Toggle fog of war |
| F4 | Cycle time scale (1x/4x/16x/64x) |
| F5 | Toggle map edge-scroll |

### Profiling

```bash
# Terminal 1: start Tracy capture
tracy-capture -o /tmp/cordon.tracy

# Terminal 2: run with profiling
cargo run --features profile --no-default-features

# After capture, open in Tracy GUI
tracy-profiler /tmp/cordon.tracy
```

## Project structure

```
crates/
  cordon-core/    Pure data types, primitives, item/quest/faction defs.
                  No Bevy dependency — the serialisable layer.
  cordon-data/    Asset loading, game data catalog, JSON loaders.
  cordon-sim/     World simulation: squads, combat, economy, quests,
                  day cycle, payroll. Runs in Bevy's Update schedule.
  cordon-bevy/    The game client: 3D bunker, FPS camera, laptop UI,
                  map, fog of war, audio, particles, dialogue.

assets/
  audio/          SFX (footsteps, doors, rack, pills, CCTV) and music.
  data/           JSON game data (items, factions, quests, upgrades, NPCs).
  locale/         Fluent (.ftl) localisation files.
  models/         GLB props (interior furniture, atomic pack, storage).
  textures/       ambientCG PBR texture sets (concrete, metal).
```

## Architecture highlights

- **`Time<Sim>`** — custom Bevy time context decoupled from `Time<Virtual>`. Sim systems read `Time<Sim>` so we can accelerate the game clock (sleep, fast-forward) without exploding the FixedMain physics loop.
- **`PropPlacement` observer** — rooms declare props declaratively; an `OnAdd` observer loads the scene, applies feet-centre correction, and spawns colliders.
- **`PlayerSquadRoster`** — squad ownership keyed by `Uid<Squad>` (save-stable). The `Owned` ECS marker is a derived cache kept in sync.
- **Player split** — `PlayerIdentity`, `PlayerStandings`, `PlayerUpgrades`, `PlayerStash` as separate Bevy resources for parallel system access.
- **Rack storage** — per-slot entities with `Interactable`, physical take/place/swap loop with carried-item visual.
- **Daily expenses** — payroll, garrison bribe, syndicate interest on debt. Itemised `DailyExpenseReport` displayed on the Trade tab.

## License

All rights reserved.
