# THROWBACK manual

## Signal flow

```
out = DRY (level only, never touched) + MIX × (lane A + lane B) ─► SAFE
```

- **PARALLEL:** both lanes hear the input.
- **SERIAL A›B:** lane B hears lane A's output.
- **CROSS-FEED:** each lane's repeats also feed the other lane's loop (CROSS sets how much).

**DRY** sets the dry level. Turn DRY OFF for 100 % wet on a send/return channel. **TAP** sets the edited lane's time in free milliseconds.

## Lane strip

Each lane strip has ON, MUTE, SOLO, Ø (phase invert), EDIT (opens the lane in the edit panel), SYNC, its knobs, OUTPUT and a meter.

## Edit panel tabs

| Tab | What's there |
|---|---|
| **TYPE** | Echo type (below) and style: SINGLE, DUAL, PING-PONG, RATIO, RHYTHM. |
| **TIME** | Note value (1/64 to 2/1, triplets and dotted) or ms; ratio, offset, groove, feel; how time changes: TAPE SLIDE, CROSSFADE or JUMP. |
| **FEEDBACK** | Feedback, cross-feedback, low/high cut (6 / 12 / 24 dB), resonance, tilt, drive in the loop, filter in loop, FREEZE. |
| **MOD** | Shape (SINE, TRIANGLE, RANDOM, TAPE WOW, ENVELOPE), target (TIME, FILTER, PAN, PITCH), rate, phase, envelope, drift. |
| **PITCH** | Semitones and fine; ONCE, EVERY REPEAT or UP / DOWN; mix, grain, spread. |
| **DIFFUSE** | Before the delay or in the loop; size, stages, motion, spread. |
| **DUCK** | Hears the DRY INPUT, the SIDECHAIN or the OTHER LANE; amount, threshold, attack, release. |
| **RHYTHM** | A 16-step level pattern (OFF, -6 dB, -2 dB, 0 dB) for the repeats. |

## Echo types

| Family | Types |
|---|---|
| Digital | STUDIO CLEAN, RACK 12-BIT, LO-FI CRUSH, RADIO, GLITCH |
| Tape | TAPE ECHO, WORN TAPE, MULTI-HEAD, DRUM ECHO, SLAPBACK |
| Analog | BUCKET, DIRTY BUCKET, CHORUS ECHO, DUB SPACE, FILTER ECHO |
| Space | DIFFUSE, GHOST, REVERSE, SHIMMER, CLOUD |

## FREEZE

Holds the repeats forever and lets nothing new in. Click it again to let go.

## Latency

Zero. Oversampling (footer) only runs on the DRIVE stage, and its delay is taken out of the delay time.
