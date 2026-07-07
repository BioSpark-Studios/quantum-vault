# VaultForge UI Assets

Drop your generated art here and rebuild — the theater `assets` loader scans these
folders at startup and swaps painter-drawn placeholders for real textures when a file
is present. Nothing here is required to build or run; every slot is optional.

All images should be **PNG with alpha**. Recommended sizes noted per folder.

## Eidolon Synthesis Rack control assets — `assets/rack/`

| Folder | What goes here | Recommended size | Loader key (filename) |
|---|---|---|---|
| `rack/knobs/` | Synth control knobs (Aether Blue / Iridescent) | 128×128 | `knob_<name>.png` e.g. `knob_aether_blue.png` |
| `rack/faders/` | Vertical faders (Solar Gold / Dark rack) | 64×256 | `fader_<name>.png` e.g. `fader_solar_gold.png` |
| `rack/buttons/` | Buttons & toggle switches (Prismatic Purple) | 96×96 | `button_<name>.png` e.g. `button_toggle.png` |
| `rack/visualizers/` | Spectrogram / oscilloscope frames | 512×256 | `viz_<name>.png` e.g. `viz_flux.png` |

Reference art (optional, not loaded into the UI, kept for design reference):
- `rack/component_sheet.png` — the full HUD component sprite sheet
- `rack/hud_mockup.png` — the 16:9 HUD layout mockup

## UI icon set — `assets/icons/`

| Folder | What goes here | Recommended size | Loader key (filename) |
|---|---|---|---|
| `icons/world/` | In-world HUD icons | 64×64 | `icon_world_<name>.png` |
| `icons/editor/` | Editor / control-room nav icons | 48×48 | `icon_editor_<name>.png` |

The loader indexes icons by the `<name>` portion of the filename, so `icon_world_thermal.png`
is retrieved as `thermal`. Use lowercase, hyphen-free names.

## Colour language (safe-state)

The Rack HUD follows the research's operational colour scheme:
- **Cyan** — SYSTEM NOMINAL
- **Amber** — THROTTLING / degraded (chassis ≥ 75 °C or a PSU degraded)
- **Red** — THERMAL CRITICAL / fault (chassis ≥ 95 °C or PSU critical)
