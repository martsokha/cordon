# Cordon

A management/trading sim set in a quarantined exclusion zone. You run a bunker, trade with scavengers, hire squads, and navigate faction politics — all from behind your counter.

**Genre:** Management / Trading Sim / Narrative
**Tone:** Gritty, tense, darkly humorous: post-apocalyptic exclusion zone meets Papers, Please
**Perspective:** First-person, fixed location (your bunker in the Zone)
**Engine:** Bevy 0.18 (Rust)
**Platform:** Desktop (macOS, Windows, Linux)

## Premise

You are a trader operating out of a bunker deep in the Zone — a quarantined, anomaly-riddled wasteland. Scavengers come to you to buy, sell, and barter. You never leave — the Zone comes to you.

Your goal: survive, profit, and navigate the politics of the Zone's factions. Every deal you make ripples outward. Sell cheap meds to a wounded drifter? He might come back with rare relics. Gouge a soldier from the Garrison? His squad might "inspect" your bunker tomorrow.

## Core Fantasy

You are not the hero — you're the guy everyone needs. You sit behind your counter, size up whoever walks through the door, and decide what they're worth to you. The tension isn't in gunfights: it's in the deal.

## Inspirations

- **Papers, Please**: moral weight in mundane transactions
- **S.T.A.L.K.E.R.**: setting, tone, factions, lore
- **Darkest Dungeon**: risk management, sending others into danger

## Running

```bash
cargo run                          # default (with Steam features)
cargo run --no-default-features    # without Steam
```

## Project structure

```
crates/
  cordon-core/    Pure data types, primitives, item/quest/faction defs.
                  No Bevy dependency: the serialisable layer.
  cordon-data/    Asset loading, game data catalog, JSON loaders.
  cordon-sim/     World simulation: squads, combat, economy, quests,
                  day cycle, payroll. Runs in Bevy's Update schedule.
  cordon-app/    The game client: 3D bunker, FPS camera, laptop UI,
                  map, fog of war, audio, particles, dialogue.

assets/
  audio/          SFX (footsteps, doors, rack, pills, CCTV) and music.
  data/           JSON game data (items, factions, quests, upgrades, NPCs).
  locale/         Fluent (.ftl) localisation files.
  models/         GLB props (interior furniture, atomic pack, storage).
  textures/       ambientCG PBR texture sets (concrete, metal).
```

## License

All rights reserved.
