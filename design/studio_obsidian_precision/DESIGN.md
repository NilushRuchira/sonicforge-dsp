---
name: Studio Obsidian Precision
colors:
  surface: '#111319'
  surface-dim: '#111319'
  surface-bright: '#373940'
  surface-container-lowest: '#0c0e14'
  surface-container-low: '#191b22'
  surface-container: '#1e1f26'
  surface-container-high: '#282a30'
  surface-container-highest: '#33343b'
  on-surface: '#e2e2eb'
  on-surface-variant: '#bac9cc'
  inverse-surface: '#e2e2eb'
  inverse-on-surface: '#2e3037'
  outline: '#849396'
  outline-variant: '#3b494c'
  surface-tint: '#00daf3'
  primary: '#c3f5ff'
  on-primary: '#00363d'
  primary-container: '#00e5ff'
  on-primary-container: '#00626e'
  inverse-primary: '#006875'
  secondary: '#4edea3'
  on-secondary: '#003824'
  secondary-container: '#00a572'
  on-secondary-container: '#00311f'
  tertiary: '#ffe9d3'
  on-tertiary: '#472a00'
  tertiary-container: '#ffc681'
  on-tertiary-container: '#7e4e00'
  error: '#ffb4ab'
  on-error: '#690005'
  error-container: '#93000a'
  on-error-container: '#ffdad6'
  primary-fixed: '#9cf0ff'
  primary-fixed-dim: '#00daf3'
  on-primary-fixed: '#001f24'
  on-primary-fixed-variant: '#004f58'
  secondary-fixed: '#6ffbbe'
  secondary-fixed-dim: '#4edea3'
  on-secondary-fixed: '#002113'
  on-secondary-fixed-variant: '#005236'
  tertiary-fixed: '#ffddb8'
  tertiary-fixed-dim: '#ffb95f'
  on-tertiary-fixed: '#2a1700'
  on-tertiary-fixed-variant: '#653e00'
  background: '#111319'
  on-background: '#e2e2eb'
  surface-variant: '#33343b'
typography:
  display-lg:
    fontFamily: Geist
    fontSize: 32px
    fontWeight: '600'
    lineHeight: 40px
    letterSpacing: -0.02em
  headline-lg:
    fontFamily: Geist
    fontSize: 24px
    fontWeight: '600'
    lineHeight: 32px
    letterSpacing: -0.015em
  headline-sm:
    fontFamily: Geist
    fontSize: 18px
    fontWeight: '500'
    lineHeight: 24px
    letterSpacing: -0.01em
  body-lg:
    fontFamily: Geist
    fontSize: 14px
    fontWeight: '400'
    lineHeight: 20px
  body-sm:
    fontFamily: Geist
    fontSize: 12px
    fontWeight: '400'
    lineHeight: 16px
  mono-metric-lg:
    fontFamily: JetBrains Mono
    fontSize: 18px
    fontWeight: '600'
    lineHeight: 24px
    letterSpacing: -0.03em
  mono-metric-md:
    fontFamily: JetBrains Mono
    fontSize: 12px
    fontWeight: '500'
    lineHeight: 16px
    letterSpacing: 0em
  mono-metric-sm:
    fontFamily: JetBrains Mono
    fontSize: 10px
    fontWeight: '500'
    lineHeight: 12px
    letterSpacing: 0.02em
  label-caps:
    fontFamily: Geist
    fontSize: 11px
    fontWeight: '600'
    lineHeight: 14px
    letterSpacing: 0.06em
rounded:
  sm: 0.125rem
  DEFAULT: 0.25rem
  md: 0.375rem
  lg: 0.5rem
  xl: 0.75rem
  full: 9999px
spacing:
  gutter: 0.5rem
  gutter-md: 0.75rem
  gutter-lg: 1rem
  margin: 0.75rem
  margin-desktop: 1.25rem
  space-2xs: 0.125rem
  space-xs: 0.25rem
  space-sm: 0.5rem
  space-md: 0.75rem
  space-lg: 1rem
  space-xl: 1.5rem
---

## Brand & Style

This design system targets master audio engineers, mixing professionals, and DSP algorithm designers operating in high-density multi-display environments. The aesthetic marries the ergonomic architectural principles of Windows 11 Fluent design (mica subtleties, restrained translucent layers, precise edge treatments) with high-end rack-mounted mastering console hardware.

Visual and functional themes:
- **Atmospheric Focus:** An ultra-dark, low-fatigue slate base minimizes visual strain in dim studio environments while reserving dynamic range for glowing meters and audio telemetry.
- **Instrument Precision:** Every control communicates micro-calibration. Physical audio references (rotary detents, stepped dB scales, LED ladder segments) are reimagined with flat, ultra-sharp vector rendering.
- **Functional Luminescence:** High-contrast neons are not decorative; they denote DSP functional domains (Cyan for spectral/routing, Emerald for reference gain, Amber for analog drive, Violet for space/acoustics).

## Colors

The palette employs a disciplined structural hierarchy where dark neutrals establish depth and vivid chromatic accents signify live audio dynamics.

### Foundation & Surfaces
- **Canvas Base (`#0F1117`):** Deep carbon slate base for global rack frames and empty canvas backgrounds.
- **Card / Module Surface (`#181B24`):** Elevated channel strips, plugin chassis, and routing containers.
- **Sub-module / Recessed Surface (`#13151D`):** Recessed fader wells, parameter input backgrounds, and FFT display viewports.
- **Structural Border (`#262B3A`):** Low-contrast technical delineation separating channel modules and meter segments.
- **Hover/Interactive Border (`#3A4156`):** Highlights active channels, hovered rotary knobs, and focused parameters.

### Domain-Specific Signal Accents
- **Primary / Electric Cyan (`#00E5FF`):** Oscilloscopes, FFT real-time curve peaks, phase correlation meters, active patch routings, and primary selection focus.
- **Secondary / Studio Emerald (`#10B981`):** Master headroom indicators, standard VU nominal thresholds (-18 to 0 dBFS safe zones), and master output stages.
- **Tertiary / Warm Amber (`#F59E0B`):** Tube drive, harmonic saturation, pre-clipping warning stages (-3 dBFS to 0 dBFS), and compressor gain-reduction cues.
- **Spatial Violet (`#8B5CF6`):** Reverb impulse engines, binaural panning, room decay representations, and acoustic simulation telemetry.
- **Clip Crimson (`#EF4444`):** Strictly reserved for true peak digital overs (> 0 dBFS clip points), buffer dropouts, and fatal clock sync errors.

## Typography

The typographic hierarchy prioritizes rapid scannability, vertical compacting, and unambiguous numerical clarity.

- **Geist (Primary Sans):** Deployed for channel labeling, dialog titles, navigation headers, and DSP mode selectors. Its geometric proportions and compact vertical metrics optimize interface density.
- **JetBrains Mono (Metric Readout):** Dedicated to decibel levels (`-12.4 dB`), frequency tags (`4.2 kHz`), Q-factors (`0.71`), buffer latencies (`64 spl / 1.4ms`), and timecode indices. Tabular figures prevent layout jumping during continuous value adjustment.
- **Display Restraint:** Font sizes cap at `32px` to maximize screen real estate for spectral curves and mixing strips. Labels on hardware emulations use uppercase track-spaced styling (`0.06em`) for industrial readability at miniature scales.

## Layout & Spacing

A compact, modular dock grid aligns hardware modules across Windows 11 studio workstations. 

- **Density & Scale:** Optimized for 4K and ultra-wide studio configurations with an 8px base rhythm and an internal 4px sub-grid for meter segments and control ticks.
- **Channel Strip Consistency:** Audio strips conform to fixed-ratio horizontal slots (e.g., 72px collapsed, 120px standard, 240px expanded routing views).
- **Docking Structure:** Multi-window docked panels snap to a fluid horizontal layout separated by 8px gutters, allowing horizontal scrolling mixer channels with anchored master monitoring columns.
- **Responsive Adaptations:**
  - **Single Screen / Laptop (<1440px):** Channel meter banks collapse to dynamic popovers; master section anchors to a collapsible bottom drawer; auxiliary plugin routing stacks into tabbed panels.
  - **Multi-Monitor Studio Rig (>=1440px):** Full simultaneous visualization of mixer channels, DSP spectral analyzer, and surround spatial field with minimum 12px outer canvas margins.

## Elevation & Depth

Visual depth reflects precision desktop instrumentation through translucent layering, recessed well cavities, and focused luminescent bloom rather than soft drop shadows.

- **Level 0 (Main Canvas - `#0F1117`):** Structural backplane hosting docking rails and channel separator tracks.
- **Level 1 (Rack Chassis - `#181B24`):** Channel strips, EQ modules, and master bus cards. Defined by a crisp `1px` border (`#262B3A`) without ambient blur.
- **Level 2 (Active Plugin & Floating Popovers):** Elevated windows leverage Windows 11 acrylic styling: `background: rgba(24, 27, 36, 0.85)` with `backdrop-filter: blur(16px)` and an internal top-edge highlight stroke (`rgba(255, 255, 255, 0.08)`).
- **Recessed Wells (`#13151D`):** Fader tracks, FFT visualizer windows, and value displays sit inset into Level 1 panels with a `1px` inner stroke (`#000000`).
- **Signal Luminescence:** Active toggles, peaking LEDs, and oscilloscope traces emit a tight, saturated glow: `0 0 8px rgba(0, 229, 255, 0.45)` for cyan traces and `0 0 8px rgba(16, 185, 129, 0.45)` for safe peak meters.

## Shapes

The interface balances sharp industrial hardware edges with subtle curvature for interactive affordances:

- **Chassis & Enclosures:** Rigid construction using `rounded-none` or `rounded-xs` (2px) to reflect milled aluminum faceplates.
- **Fader Caps, Knobs, & Buttons:** Softened using `rounded-sm` (4px) to indicate tactile touch surfaces and grab states.
- **Rotary Dials:** Pure circles (`rounded-full`) framed by concentric calibration ticks and radial progress arcs.
- **Meters & Segmented Displays:** Crisp, unrounded rectangles (0px) for segmented LED ladders and frequency bins to preserve alignment across dense grids.

## Components

### Precision Faders
- **Track Well:** Recessed 6px-wide dark groove (`#13151D`) bordered by a `1px` outline (`#262B3A`) with center 0 dB detent markings.
- **Fader Cap:** 28px × 14px brushed slate block (`#262B3A`) with a centered 1.5px glowing accent indicator line corresponding to the channel group color.
- **Interaction:** Smooth linear dragging with Cmd/Ctrl-click reset to unity gain (`0.0 dB`).

### Brushed Radial Knobs
- **Base:** Concentric layered circle (`#181B24`) with an outer bevel ring (`#262B3A`).
- **Indicator:** Continuous vector arc drawn along the circumference. The active value track renders in domain neons (e.g., Amber for Drive, Violet for Space) terminating in a crisp 2px needle tick.
- **Center Readout:** Center text field reveals numeric readout on hover, switching from nominal label to real-time value.

### Segmented LED Ladders & FFT Spectrum Curves
- **Peak / RMS Meters:** Multi-segment vertical meters with 1px gaps between LED blocks. Segments illuminate progressively from Studio Emerald (`-inf` to `-6 dB`), through Warm Amber (`-6` to `-0.1 dB`), to Clip Crimson (`0 dB` and over). True-peak hold line persists for 1.2 seconds.
- **FFT Curve Visualizer:** High-framerate SVG/Canvas viewport rendering a 1.5px Electric Cyan stroke with an area fill gradient fading from `rgba(0, 229, 255, 0.18)` down to `transparent`. Parametric EQ band nodes feature draggable interactive circles with radiating semi-transparent handles.

### Input Fields & Value Scrubber Pills
- **Container:** Compact 22px height, `#13151D` fill, `1px` border (`#262B3A`).
- **Typography:** `JetBrains Mono` at `11px`, right-aligned with unit suffixes (`dB`, `Hz`, `ms`) tinted in 50% opacity neutral.
- **Micro-interactions:** Vertical click-drag modifies values in fine increments; double-click opens direct text input.

### Mute, Solo, & Function Toggles
- **Default State:** Inactive dark state (`#181B24`) with subtle gray text (`#8A92A6`).
- **Mute Active:** Amber alert fill with high-contrast dark text or bright Amber text with `rgba(245, 158, 11, 0.15)` background.
- **Solo Active:** Electric Cyan fill (`#00E5FF`) with `#0F1117` text and ambient outer glow.
- **Record Arm:** Clip Crimson solid fill with pulsating 1Hz indicator dot.

### Channel Strips & Cards
- **Module Structure:** Vertical card with top routing header, multi-band parametric EQ preview, aux send knobs, fader with parallel LED ladder, and bottom channel identity tag with programmable color stripes.
- **Selected State:** Complete card perimeter highlighted with a `1px` Electric Cyan stroke and subtle surface glow.