# THROWBACK manual

Every control in THROWBACK, what it does, and its range and default. Lanes A and B have identical controls; the tables list them once.

- [Signal flow](#signal-flow)
- [Vocal throws: what to automate](#vocal-throws-what-to-automate)
- [Global and routing](#global-and-routing)
- [ORIGINAL strip (PARALLEL off)](#original-strip-parallel-off)
- [Lane strip](#lane-strip)
- [Edit panel tabs](#edit-panel-tabs): TYPE, TIME, FEEDBACK, MOD, PITCH, DIFFUSE, DUCK, RHYTHM
- [Echo types](#echo-types)
- [Latency](#latency)

## Signal flow

```
PARALLEL ON  (default):  out = DRY × your sound (untouched)            + MIX × (lane A + lane B)  ─► SAFE
PARALLEL OFF (INSERT):   out = DRY × ORIGINAL strip(your sound)        + MIX × (lane A + lane B)  ─► SAFE
```

The lanes always hear your untouched input, so the echoes are the same in both modes. PARALLEL only decides whether your own sound is left alone or shaped by the ORIGINAL strip. At its default settings the ORIGINAL strip does nothing, so switching PARALLEL off changes nothing until you turn one of its knobs.

**SIGNAL NOW** (bottom of the ROUTING panel) shows one row per path with a lamp: ORIGINAL (CLEAN or INSERT, and the dry level), lane A and lane B (ON, OFF or MUTED, their time and how many repeats you hear).

## Vocal throws: what to automate

Load **VOCAL - The Throw (automate A SEND)**. It is a dark ping-pong tape echo on quarter notes that ducks under the vocal, feeding a small diffuse space (A › B), so only the thrown words get space.

| Automate | How |
|---|---|
| **Lane A › SEND** | The main one. Keep it at OFF, and push it to 0 dB on the last word or syllable of a line, then back to OFF. Only what goes in while SEND is up gets echoed, and the echoes keep ringing after you pull it down. |
| Lane A › FEEDBACK | Raise it on the last throw before a drop or section change for a longer tail (stay under 100% unless you want it to build). |
| Lane A › HIGH CUT | Pull it down over a tail to make the throw fade into the distance. |
| Lane A › FREEZE | Hold a throw forever for a transition, then release it. |
| Global MIX | Fade every echo in or out at once without touching the sends. |

In FL Studio: right-click a knob › **Create automation clip**, or **Link to controller** to ride it with a fader.

## Global and routing

| Control | Range (default) | What it does |
|---|---|---|
| **MIX** (top strip) | 0 to 100% (100%) | Level of both lanes together. It never changes your dry sound. |
| **Power** (top strip) | on / off | Bypass. Crossfades to the untouched input. |
| **PARALLEL** | on / off (on) | ON: your sound passes untouched. OFF (INSERT): your sound runs through the ORIGINAL strip. |
| **LANES A + B** | SIDE BY SIDE, A › B, CROSS-FEED (SIDE BY SIDE) | How the lanes connect. SIDE BY SIDE: both hear your input. A › B: lane B hears lane A's echoes (echo into reverb-like space). CROSS-FEED: each lane's repeats also feed the other lane's loop. |
| **DRY** | OFF to +6 dB (0 dB) | Level of your own sound. |
| **DRY: ON / OFF** | on / off (on) | OFF = 100% wet, for a send/return channel. |
| **CROSS** | 0 to 100% (30%) | How much each lane feeds the other in CROSS-FEED. Greyed out otherwise. |
| **TAP** | | Tap twice or more to set the edited lane's time in free milliseconds. |
| **OVERSAMPLING** (footer) | OFF, 2×, 4×, 8× (OFF) | Oversamples the lanes' DRIVE stage only. No added latency. |
| **SAFE** (footer) | on / off (on) | Output limiter: the plugin never goes over 0 dBFS. |

## ORIGINAL strip (PARALLEL off)

Your own sound, with the same tone controls as a lane and no repeats. Greyed out while PARALLEL is on.

| Control | Range (default) | What it does |
|---|---|---|
| **DRIVE** | 0 to 24 dB (0 dB) | Soft saturation. Level-compensated. |
| **LOW CUT** | 20 Hz to 2 kHz (20 Hz = off) | 12 dB/oct high-pass. |
| **HIGH CUT** | 1 kHz to 20 kHz (20 kHz = off) | 12 dB/oct low-pass. |
| **WIDTH** | 0 to 200% (100%) | 0% = mono, 100% = as is, 200% = extra wide. |
| **PAN** | L100 to R100 (C) | Balance. The far side fades out, the near side stays at 0 dB. |
| **OUTPUT** | OFF to +6 dB (0 dB) | Level of the shaped sound (DRY still applies on top). |

## Lane strip

| Control | Range (default) | What it does |
|---|---|---|
| **ON** (lit key) | on / off (on) | Turns the lane on. Lit amber (A) or pink (B) while it runs. |
| **MUTE** | on / off | Silences the lane's output; it keeps running so the echoes come back where they were. |
| **SOLO** | on / off | Hear only soloed lanes (the dry stays). |
| **Ø** | on / off | Flips the lane's polarity. |
| **EDIT ›** | | Shows this lane in the edit panel below. |
| **SYNC / FREE ms** | | Time follows the host tempo (note values) or is set in milliseconds. |
| **FREEZE** | on / off | Holds the repeats forever and lets nothing new in. Click again to let go. |
| **SEND** | OFF to +6 dB (0 dB) | How much of your input goes into the lane. **The knob to automate for throws.** |
| **TIME** | 1/64 to 2/1, triplets and dotted (A 1/8D, B 1/2), or 1 to 4000 ms | Delay time. |
| **FEEDBACK** | 0 to 110% (A 38%) | How many repeats. Over 100% builds up; SAFE keeps the output under 0 dB. |
| **PITCH** | -24 to +24 semitones (0) | Pitch shift of the repeats (mode on the PITCH tab). |
| **MOD** | 0 to 100% (15%) | Modulation depth (shape and target on the MOD tab). |
| **DRIVE** | 0 to 24 dB (A 6 dB) | Saturation of what enters the lane. |
| **DIFFUSE** | 0 to 100% (0%) | Smears the repeats (where and how on the DIFFUSE tab). |
| **LOW CUT** | 20 Hz to 2 kHz (A 150 Hz) | Removes low end from the echoes. |
| **HIGH CUT** | 1 kHz to 20 kHz (A 6 kHz) | Darkens the echoes. |
| **WIDTH** | 0 to 200% (A 100%) | Stereo width of the echoes. |
| **PAN** | L100 to R100 (C) | Places the lane's echoes. |
| **OUTPUT** | OFF to +6 dB (A -6 dB) | Lane level. The meter beside it shows the lane's peak. |

## Edit panel tabs

### TYPE

| Control | Range (default) | What it does |
|---|---|---|
| **Echo type** | 20 types (A TAPE ECHO) | The character of the repeats. See [Echo types](#echo-types). |
| **Style** | SINGLE, DUAL, PING-PONG, RATIO, RHYTHM (A PING-PONG) | SINGLE: one echo. DUAL: separate left and right times. PING-PONG: repeats bounce left and right. RATIO: the right side runs at L/R RATIO of the left. RHYTHM: the 16-step pattern on the RHYTHM tab sets each step's level. |

Type-specific controls (only the ones for the current type are shown):

| Control | Types | Range (default) | What it does |
|---|---|---|---|
| **Bits** | Digital | 4 to 24-bit (24) | Bit depth of the repeats. |
| **Sample Rate** | Digital | 2 to 48 kHz (48 kHz) | Sample-rate reduction of the repeats. |
| **Bandwidth** | Digital | 2 to 20 kHz (20 kHz) | Top-end limit. |
| **Digital Noise** | Digital | 0 to 100% (0%) | Converter noise. |
| **Stutter** | GLITCH | 0 to 100% (50%) | How often repeats skip and re-trigger. |
| **Wow** | Tape | 0 to 100% (25%) | Slow pitch wobble. |
| **Flutter** | Tape | 0 to 100% (20%) | Fast pitch wobble. |
| **Tape Saturation** | Tape | 0 to 100% (35%) | Tape compression and warmth on each pass. |
| **Head Bump** | Tape | 0 to 6 dB (2.4 dB) | Low-mid lift typical of tape heads. |
| **Hiss** | Tape | 0 to 100% (10%) | Tape noise. |
| **Clock Noise** | Analog | 0 to 100% (15%) | Bucket-brigade clock whine. |
| **Compander** | Analog | 0 to 100% (40%) | The pumping of analog delay noise reduction. |
| **Darkness** | Analog | 0 to 100% (50%) | How fast repeats lose their top end. |
| **Age** | Analog | 0 to 100% (20%) | Wear: more noise and instability. |
| **Cloud Density** | CLOUD | 0 to 100% (60%) | How many grains. |
| **Grain** | CLOUD | 10 to 200 ms (86 ms) | Grain length. |
| **Grain Spread** | CLOUD | 0 to 100% (60%) | Stereo scatter of grains. |
| **Detune** | GHOST | 0 to 50 cents (10) | Detune added each pass. |
| **Reverse Window** | REVERSE | 20 to 1000 ms (510 ms) | Length of each backwards slice. |

### TIME

| Control | Range (default) | What it does |
|---|---|---|
| **Time** | notes or ms | Same as the strip's TIME. |
| **L/R Ratio** | 0.25× to 2× (0.75×) | Right-side time relative to the left (RATIO style). |
| **R Offset** | -50 to +50 ms (0) | Pushes the right side earlier or later (Haas width). |
| **Groove** | 50% to 75% swing (50%) | Swings every other repeat. |
| **Feel** | -20 to +20 ms (0) | Pushes all repeats ahead of or behind the beat. |
| **When Time Changes** | TAPE SLIDE, CROSSFADE, JUMP (TAPE SLIDE) | TAPE SLIDE: pitch glides like a tape machine. CROSSFADE: smooth blend. JUMP: instant. |
| **Glide** | 0 to 2000 ms (600 ms) | How long the slide or crossfade takes. |

### FEEDBACK

| Control | Range (default) | What it does |
|---|---|---|
| **Feedback** | 0 to 110% | Same as the strip's FEEDBACK. |
| **L/R Cross** | 0 to 100% (0%) | Feeds each side's repeats into the other side. |
| **Filter Slope** | 6, 12, 24 dB/oct (12) | Steepness of LOW CUT and HIGH CUT. |
| **Filter In Loop** | on / off (on) | On: the filters act on every repeat, so echoes get thinner each pass. Off: they act once on the output. |
| **Resonance** | 0 to 100% (15%) | Peak at the filter corners. |
| **Tilt** | -6 to +6 dB (0) | Darker (−) or brighter (+) each pass. |
| **Loop Drive** | 0 to 18 dB (3.6 dB) | Saturation inside the loop, building on each repeat. |
| **Freeze** | on / off | Same as the strip's FREEZE. |

### MOD

| Control | Range (default) | What it does |
|---|---|---|
| **Mod** | 0 to 100% (15%) | Depth (same as the strip's MOD). |
| **Mod Shape** | SINE, TRIANGLE, RANDOM, TAPE WOW, ENVELOPE (TAPE WOW) | Movement shape. ENVELOPE follows your input's level. |
| **Mod Target** | TIME, FILTER, PAN, PITCH (TIME) | What moves. |
| **Mod Rate** | 0.1 to 10 Hz (0.6 Hz) | Speed. |
| **Mod Stereo Phase** | 0 to 180° (90°) | Offset between left and right movement. |
| **Mod Env Sens** | 0 to 100% (50%) | How strongly ENVELOPE reacts to your input. |
| **Mod Drift** | 0 to 100% (20%) | Random slow drift on top of the shape. |

### PITCH

| Control | Range (default) | What it does |
|---|---|---|
| **Pitch** | -24 to +24 st (0) | Same as the strip's PITCH. |
| **Pitch Fine** | -100 to +100 cents (0) | Fine tune. |
| **Pitch Mode** | ONCE, EVERY REPEAT, UP / DOWN (ONCE) | ONCE: shifted once going in. EVERY REPEAT: each repeat climbs further. UP / DOWN: left repeats go up, right ones go down. |
| **Pitch Mix** | 0 to 100% (100%) | Shifted vs unshifted. |
| **Pitch Grain** | 10 to 200 ms (60 ms) | Shifter window: short = tighter, long = smoother. |
| **Pitch L/R Detune** | 0 to 30 cents (0) | Opposite detune left and right for width. |

### DIFFUSE

| Control | Range (default) | What it does |
|---|---|---|
| **Diffuse** | 0 to 100% (0%) | Amount (same as the strip's DIFFUSE). |
| **Diffuse Position** | BEFORE THE DELAY, IN THE LOOP | BEFORE: soft-edged echoes. IN THE LOOP: each repeat smears more, turning into reverb. |
| **Diffuse Size** | 5 to 200 ms (90 ms) | Size of the smear. |
| **Diffuse Density** | 2 to 12 stages (8) | More stages = smoother. |
| **Diffuse Motion** | 0 to 100% (30%) | Slow movement inside the diffuser (less metallic). |
| **Diffuse Spread** | 0 to 100% (70%) | Stereo spread of the smear. |

### DUCK

The echoes get quieter while the source plays and come back in the gaps.

| Control | Range (default) | What it does |
|---|---|---|
| **Duck Listens To** | DRY INPUT, SIDECHAIN, OTHER LANE (DRY INPUT) | What pushes the echoes down. |
| **Duck Amount** | 0 to 40 dB (A 14 dB) | How far they go down. 0 = off. |
| **Duck Threshold** | -60 to 0 dB (-24 dB) | Level where ducking starts. |
| **Duck Attack** | 0 to 100 ms (20 ms) | How fast they go down. |
| **Duck Release** | 10 to 2000 ms (600 ms) | How fast they come back. |

### RHYTHM

16 steps over one bar. Click a step to cycle OFF, -6 dB, -2 dB, 0 dB (right-click = OFF). Used by the RHYTHM style; FEEDBACK repeats the whole bar.

## Echo types

| Family | Type | Sound |
|---|---|---|
| Digital | STUDIO CLEAN | Exact copies. Nothing added, nothing lost. |
| | RACK 12-BIT | 80s rack unit: 12-bit, a little gritty, rolled off at 12 kHz. |
| | LO-FI CRUSH | Bit and sample-rate reduction, dirtier as it fades. |
| | RADIO | 300 Hz to 3 kHz with light distortion. Phone and radio throws. |
| | GLITCH | Repeats skip, stutter and re-trigger on the beat grid. |
| Tape | TAPE ECHO | One head: warm saturation, gentle wow and flutter, darker each pass. |
| | WORN TAPE | Stretched tape: heavier wobble, hiss, the odd dropout. |
| | MULTI-HEAD | Three heads at 1×, 2× and 3× the time. Space-echo patterns. |
| | DRUM ECHO | Spinning magnetic drum: dark and wobbly. |
| | SLAPBACK | One or two short tape slaps. |
| Analog | BUCKET | Bucket-brigade chip: dark, soft repeats. |
| | DIRTY BUCKET | Pushed bucket-brigade: clock noise and compander pumping. |
| | CHORUS ECHO | Every repeat is chorused. |
| | DUB SPACE | Resonant filter in the loop, built for long dub tails. |
| | FILTER ECHO | A swept filter moves through the repeats. |
| Space | DIFFUSE | Repeats smear into each other; with feedback, a reverb. |
| | GHOST | Diffused and slightly detuned. |
| | REVERSE | Each repeat plays backwards and swells in. |
| | SHIMMER | Pitch shift in the loop: every repeat climbs. |
| | CLOUD | Granular: repeats scatter into tiny grains. |

## Latency

Zero, in both PARALLEL and INSERT. Oversampling only runs on the lanes' DRIVE stage, and its delay is taken out of the delay time.
