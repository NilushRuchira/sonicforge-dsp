# SonicForge DSP - Windows Audio Matrix & DSP Control Center

**SonicForge DSP** is a production-grade, standalone Windows desktop application engineered for high-performance audio routing, multi-device mixing, 10/20-band graphic/parametric equalization, real-time audio effects processing, and sub-millisecond audio visualization.

Built with **Rust**, **WASAPI Core Audio APIs (`cpal`)**, and a **Windows 11 Fluent Studio Dark Aesthetic**, it adheres to real-time audio safety standards with lock-free atomic queues and non-blocking I/O.

---

## 1. Visual Verification & Interface Showcase

| Multi-Track Matrix | Precision Equalizer & FFT |
|:---:|:---:|
| ![Multi-Track Matrix](walkthrough_assets/sonicforge_matrix.png) | ![Precision Equalizer](walkthrough_assets/sonicforge_equalizer.png) |

| DSP FX Rack | Audio Routing Map Patchbay |
|:---:|:---:|
| ![DSP FX Rack](walkthrough_assets/sonicforge_fx_rack.png) | ![Audio Routing Map](walkthrough_assets/sonicforge_routing_map.png) |

### Design System Compliance
- **Backgrounds**: Deep Carbon (`#0F1117`), secondary cards (`#181B24`), high-contrast subtle borders (`#262B3A`).
- **Accents**:
  - **Electric Cyan (`#00E5FF`)**: Active workspaces, waveform traces, master meters, and peaking filters.
  - **Studio Emerald (`#10B981`)**: Reverb & system status indicators, low-distortion status.
  - **Warm Amber (`#F59E0B`)**: Dynamic saturation, tube warmth, and sub-harmonic drive.
  - **Spatial Violet (`#8B5CF6`)**: Schroeder reverberation and acoustic ambience.

---

## 2. Mathematical Foundations & DSP Implementation

### A. Direct Form II Transposed Biquad Filter
The equalizer and shelf filters are implemented using normalized Direct Form II Transposed topology:

$$y[n] = b_0 x[n] + s_1[n-1]$$
$$s_1[n] = b_1 x[n] - a_1 y[n] + s_2[n-1]$$
$$s_2[n] = b_2 x[n] - a_2 y[n]$$

- **Stability & Denormal Immunity**: All pole locations are verified strictly within the unit circle ($|a_2| < 1.0$, $|a_1| < 1.0 + a_2$). Inputs and delay states are clamped to $[-8.0, +8.0]$ and $[-32.0, +32.0]$, and subnormal floats below $10^{-15}$ are flushed to prevent CPU denormal execution penalties.

### B. Real Bass Analog Soft Saturation
Simulates vintage tube/transformer warmth using normalized cubic soft saturation:

$$f(x) = \begin{cases} x - \frac{x^3}{3} & |x| < 1 \\ \frac{2}{3} \operatorname{sgn}(x) & |x| \ge 1 \end{cases}$$

Accompanied by a 120 Hz Linkwitz-Riley mono collapsing crossover and a flip-flop sub-octave divider.

### C. Brickwall True Peak Limiter
Ensures broadcast compliance at `-0.1 dBTP` with zero inter-sample clipping:

$$L_{\text{linear}} = 10^{\frac{-0.1}{20}} \approx 0.98855$$
$$y[n] = \frac{x[n]}{\max(1.0, |x[n]| / L_{\text{linear}})}$$

---

## 3. Test Verification & Telemetry Results

### A. Automated Unit Tests (`cargo test`)
All 9 core DSP and hardware enumeration tests passed with zero failures:

```
running 9 tests
test test_brickwall_limiter_ceiling ... ok
test test_biquad_nan_inf_immunity ... ok
test test_biquad_unit_circle_stability ... ok
test test_treble_sparkle_and_tube_warmth ... ok
test test_real_bass_soft_saturation ... ok
test test_equalizer_modes_and_presets ... ok
test test_matrix_router_dispatch ... ok
test test_reverb_schroeder_stability ... ok
test test_device_enumeration ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
```

### B. Hardware Device Discovery & Telemetry Execution (`--verify-pipeline`)
```
============================================================
 SonicForge DSP - Windows Audio Matrix & DSP Control Center 
 Core Version: 2.4.12-x64 (Windows 11 Fluent Studio)       
============================================================
[Audio Core] Enumerated 22 active WASAPI endpoints:
  [0] [RENDER] Voicemeeter In 1 (VB-Audio Voicemeeter VAIO) - 48000Hz/2ch
  [1] [RENDER] Voicemeeter VAIO3 Input (VB-Audio Voicemeeter VAIO) - 48000Hz/2ch
  [2] [RENDER] Voicemeeter In 4 (VB-Audio Voicemeeter VAIO) - 48000Hz/2ch
  [3] [RENDER] Voicemeeter In 5 (VB-Audio Voicemeeter VAIO) - 48000Hz/2ch
  [4] [RENDER] Speakers (Realtek(R) Audio) - 48000Hz/2ch
  [5] [RENDER] Line 1 (Virtual Audio Cable) - 48000Hz/2ch
  [6] [RENDER] Headphones (Tronsmart T7) - 48000Hz/2ch (DEFAULT)
  [7] [RENDER] Voicemeeter In 2 (VB-Audio Voicemeeter VAIO) - 48000Hz/2ch
  [8] [RENDER] Voicemeeter AUX Input (VB-Audio Voicemeeter VAIO) - 48000Hz/2ch
  [9] [RENDER] Voicemeeter In 3 (VB-Audio Voicemeeter VAIO) - 48000Hz/2ch
  [10] [RENDER] Voicemeeter Input (VB-Audio Voicemeeter VAIO) - 48000Hz/2ch
  [11] [CAPTURE] Microphone Array (Intel® Smart Sound Technology for Digital Microphones) - 48000Hz/2ch
  [12] [CAPTURE] Voicemeeter Out B2 (VB-Audio Voicemeeter VAIO) - 48000Hz/2ch
  [21] [CAPTURE] Line 1 (Virtual Audio Cable) - 48000Hz/2ch (DEFAULT)
[Verify] Running audio pipeline verification in headless mode...
[Verify] Sample Rate: 48000 Hz
[Verify] Buffer Size: 64 samples (1.33 ms)
[Verify] Peak L: -6.92 dBFS, Peak R: -7.22 dBFS
[Verify] RMS L: -14.63 dBFS, RMS R: -15.03 dBFS
[Verify] Spectrum 32-bins: 32 items, FFT 72-bins: 72 items
[Verify] Pipeline successfully validated with 0 dropouts!
```

---

## 4. How to Run

### Run Native Desktop Application
```bash
cargo run --
```

### Run Headless Pipeline Telemetry Verification
```bash
cargo run -- --verify-pipeline
```

### Run Unit Tests
```bash
cargo test
```
