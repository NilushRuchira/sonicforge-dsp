# SonicForge DSP - Windows Audio Matrix & DSP Control Center

[![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%2011%20%7C%20WASAPI-0078D6.svg?style=flat-square&logo=windows11)](https://microsoft.com)
[![License](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)

**SonicForge DSP** is a production-grade, standalone Windows desktop application for sound routing, multi-device mixing, 10/20-band graphic/parametric EQ, and master/per-channel audio effects with a real-time waveform oscilloscope visualizer.

Engineered with **Rust**, **WASAPI Core Audio APIs (`cpal`)**, and a **Windows 11 Fluent Studio Dark Aesthetic**, it adheres to real-time audio safety standards with lock-free atomic queues (`rtrb`) and non-blocking I/O.

---

## 📸 Interface Showcase

| Multi-Track Matrix | Precision Equalizer & FFT |
|:---:|:---:|
| ![Multi-Track Matrix](walkthrough_assets/sonicforge_matrix.png) | ![Precision Equalizer](walkthrough_assets/sonicforge_equalizer.png) |

| DSP FX Rack | Audio Routing Map Patchbay |
|:---:|:---:|
| ![DSP FX Rack](walkthrough_assets/sonicforge_fx_rack.png) | ![Audio Routing Map](walkthrough_assets/sonicforge_routing_map.png) |

---

## ✨ Features

- **Device Enumeration & Multi-Output Dispatch**:
  - Auto-enumerates all active Windows WASAPI playback endpoints (Speakers, Headphones, USB DACs, Virtual Audio Cables).
  - Parallel dispatch to 2 or more physical devices with independent volume faders (-inf to +6 dB), Solo, Mute, and Phase Inversion.
  - Windows Loopback capture (`AUDCLNT_STREAMFLAGS_LOOPBACK`) with fallback synthetic audio generator.
- **10-Band & 20-Band Graphic/Parametric Equalizer**:
  - Cascaded Direct Form II Transposed peaking IIR biquad filters.
  - Switchable 10-Band ISO Graphic EQ ($Q \approx 1.414$) and 20-Band Half-Octave Precision EQ ($Q \approx 2.87$).
  - High-Pass / Low-Cut and Low-Pass / High-Cut filters with selectable 12 dB/oct and 24 dB/oct slopes.
  - Built-in calibrated acoustic presets (`Flat`, `Bass Punch`, `Vocal Air`, `Loudness`).
- **Precision DSP Effects Rack**:
  - **Real Bass**: Low-shelf filter + normalized cubic soft saturation ($x - x^3 / 3$) + sub-octave flip-flop generator + 120 Hz mono collapse.
  - **Treble Sparkle**: Baxandall high-shelf exciter + 12AX7 triode tube harmonic saturation.
  - **Real Reverb**: Algorithmic Schroeder/Freeverb reverberator (8 parallel comb filters + 4 series allpass network).
  - **Brickwall Limiter**: True Peak limiting with calibrated `-0.1 dBTP` broadcast ceiling.
  - **DSP Sync Mode**: Synchronize DSP rack settings globally or unlock for per-device processing.
- **Real-Time Visualizer & Telemetry**:
  - 60 FPS continuous time-domain oscilloscope & dual FFT spectrum analyzer.
  - 32-bar and 72-bar ISO frequency spectrum monitors.
  - Dual-channel LED peak and RMS decibel meters.
  - Real-time buffer latency and dropout/underrun telemetry counters.
- **Thread Safety**:
  - Audio callback is 100% non-blocking: zero heap allocations, zero mutexes, lock-free SPSC ring buffers (`rtrb`).

---

## 🛠️ System Architecture

```mermaid
flowchart TD
    subgraph AudioIngestion["1. Audio Ingestion Layer"]
        A1["WASAPI Desktop Loopback (AUDCLNT_STREAMFLAGS_LOOPBACK)"]
        A2["Virtual Audio Cable (Line 1 Input)"]
        A3["Internal Signal Generator (48 kHz)"]
    end

    subgraph MatrixRouter["2. Lock-Free Audio Engine"]
        M["4x3 Audio Interconnect Matrix (AudioMatrixRouter)"]
        RTRB["Lock-Free SPSC Ring Buffer (rtrb)"]
        TEL["60 FPS Telemetry Pump (Peak, RMS, FFT, Osc)"]
    end

    subgraph DSPPipeline["3. Precision DSP Pipeline"]
        EQ["10 / 20-Band Biquad Direct Form II Transposed"]
        RB["Real Bass (Cubic Soft Saturation + Sub-Harmonic Gen)"]
        TS["Treble Sparkle (High Shelf + 12AX7 Triode Exciter)"]
        RV["Real Reverb (Schroeder 8-Comb + 4-Allpass Matrix)"]
        LIM["Brickwall Limiter (-0.1 dBTP Ceiling)"]
    end

    subgraph PhysicalOutputs["4. Multi-Output Dispatch"]
        O1["Studio Monitors (Focusrite USB DAC)"]
        O2["Headphone Out (Realtek HD Audio)"]
        O3["Virtual Broadcast Cable (OBS / Discord)"]
    end

    subgraph NativeGUI["5. Windows 11 Fluent UI"]
        WEB["Microsoft Edge WebView2 Engine"]
        CANVAS["60 FPS Canvas Visualizer (Spline Curve + FFT)"]
        IPC["Bi-Directional JSON IPC Bridge"]
    end

    AudioIngestion --> M
    M --> EQ --> RB --> TS --> RV --> LIM
    LIM --> PhysicalOutputs
    LIM --> RTRB --> TEL
    TEL --> IPC --> NativeGUI
    NativeGUI --> IPC --> MatrixRouter
```

---

## 🚀 Getting Started

### Prerequisites
- Windows 10/11 (64-bit)
- [Rust & Cargo](https://rustup.rs/) (1.85+)
- Microsoft Edge WebView2 Runtime (pre-installed on Windows 10/11)

### Build & Run

```bash
# Clone the repository
git clone https://github.com/<username>/sonicforge-dsp.git
cd sonicforge-dsp

# Run native desktop application
cargo run --release

# Run headless pipeline verification
cargo run -- --verify-pipeline

# Run unit tests
cargo test
```

---

## 🧪 Testing

SonicForge DSP includes comprehensive automated unit tests covering mathematical stability, denormal immunity, limiter bounds, and device discovery:

```bash
cargo test
```

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

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
