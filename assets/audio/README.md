# Audio assets

## Layout

- `sfx/` — short effects (footsteps, UI clicks, impacts, weapon fire). **WAV** for zero-decode-latency playback.
- `music/` — background music tracks. **OGG** (Vorbis).
- `ambient/` — room tones, looping atmosphere (wind, hum, generator rumble). **OGG**.
- `voice/` — dialogue and barks. **OGG**.

## Format choice

| Kind                     | Format | Why                                                                 |
| ------------------------ | ------ | ------------------------------------------------------------------- |
| Short SFX (≤1 s)         | WAV    | No decode cost → sample-accurate playback.                          |
| Loops / music / voice    | OGG    | ~10× smaller than WAV; Bevy decodes natively, no extra deps.        |

Avoid MP3 (needs Bevy's `mp3` feature flag, and OGG is strictly better for speech + small file size).
Avoid FLAC unless shipping a lossless soundtrack.

## Target specs

- Sample rate: 44.1 kHz or 48 kHz.
- Channels: mono for positional SFX, stereo for music / UI / screen glow.
- OGG encoding quality: `-q 4` to `-q 6` (ffmpeg `-qscale:a 4`).
- Keep SFX ≤ 1 MB per file; anything longer should be OGG.
