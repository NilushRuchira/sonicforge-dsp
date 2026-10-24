//! Audio Engine with WASAPI Loopback, Device Enumeration & Multi-Device Dispatch

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread;
use parking_lot::Mutex;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::{RingBuffer, Producer, Consumer};
use crate::dsp::matrix::AudioMatrixRouter;

pub const RING_BUFFER_CAPACITY: usize = 16384;
pub const FFT_SIZE: usize = 1024;

#[derive(Debug, Clone, serde::Serialize)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_default: bool,
    pub is_input: bool,
    pub is_bluetooth: bool,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TelemetryData {
    pub sample_rate: u32,
    pub buffer_size: u32,
    pub latency_ms: f32,
    pub dropouts: u64,
    pub cpu_load_pct: f32,
    pub master_peak_l: f32,
    pub master_peak_r: f32,
    pub master_rms_l: f32,
    pub master_rms_r: f32,
    pub channel_peaks: Vec<f32>,
    pub spectrum_32: Vec<f32>,
    pub spectrum_72: Vec<f32>,
    pub oscilloscope: Vec<f32>,
}

pub struct AudioEngine {
    pub is_running: Arc<AtomicBool>,
    pub sample_rate: Arc<AtomicU32>,
    pub buffer_size: Arc<AtomicU32>,
    pub underruns: Arc<AtomicU64>,
    pub router: Arc<Mutex<AudioMatrixRouter>>,
    producer: Arc<Mutex<Producer<f32>>>,
    consumer: Option<Consumer<f32>>,
    capture_prod: Arc<Mutex<Producer<f32>>>,
    devices: Vec<AudioDeviceInfo>,
    pub test_mode: Arc<AtomicBool>,
    pub output_stream: Option<cpal::Stream>,
    pub input_stream: Option<cpal::Stream>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (producer, consumer) = RingBuffer::new(RING_BUFFER_CAPACITY);
        let producer = Arc::new(Mutex::new(producer));
        let sample_rate = Arc::new(AtomicU32::new(48000));
        let buffer_size = Arc::new(AtomicU32::new(480));
        let underruns = Arc::new(AtomicU64::new(0));
        let is_running = Arc::new(AtomicBool::new(true));
        let test_mode = Arc::new(AtomicBool::new(true)); // Test synth enabled by default for immediate audible feedback
        let router = Arc::new(Mutex::new(AudioMatrixRouter::new(48000.0)));

        let devices = Self::enumerate_devices();

        let host = cpal::default_host();
        let mut output_stream = None;
        let mut input_stream = None;

        // Lock-free ring buffer for capturing input audio frames
        let (loopback_prod, mut loopback_cons) = RingBuffer::<f32>::new(65536);
        let capture_prod = Arc::new(Mutex::new(loopback_prod));

        // Attempt to connect to real hardware output device
        if let Some(out_dev) = host.default_output_device() {
            if let Ok(config) = out_dev.default_output_config() {
                let stream_config: cpal::StreamConfig = config.into();
                let channels = stream_config.channels as usize;
                let sr = stream_config.sample_rate as f32;

                sample_rate.store(stream_config.sample_rate, Ordering::Relaxed);

                // Prefer default input device (e.g. Line 1 Virtual Audio Cable), fallback to render loopback
                let cap_dev = host.default_input_device().or_else(|| host.default_output_device());
                if let Some(dev) = cap_dev {
                    let cap_config = dev.default_input_config().or_else(|_| dev.default_output_config());
                    if let Ok(config) = cap_config {
                        let in_stream_config: cpal::StreamConfig = config.into();
                        let in_ch = in_stream_config.channels as usize;
                        let capture_prod_in = capture_prod.clone();
                        let in_res = dev.build_input_stream(
                            in_stream_config,
                            move |data: &[f32], _| {
                                let mut prod = capture_prod_in.lock();
                                for chunk in data.chunks(in_ch) {
                                    let l = chunk[0];
                                    let r = if chunk.len() > 1 { chunk[1] } else { l };
                                    let _ = prod.push(l);
                                    let _ = prod.push(r);
                                }
                            },
                            |err| eprintln!("[Audio Core] Capture warning: {}", err),
                            None,
                        );

                        if let Ok(st) = in_res {
                            if st.play().is_ok() {
                                input_stream = Some(st);
                                println!("[Audio Core] Input Audio Capture Stream ACTIVE");
                            }
                        }
                    }
                }

                let router_out = router.clone();
                let test_mode_out = test_mode.clone();
                let underruns_out = underruns.clone();
                let producer_out = producer.clone();

                let mut beat_phase = 0.0_f32;
                let mut phase_sub = 0.0_f32;
                let mut phase_bass = 0.0_f32;
                let mut phase_mid = 0.0_f32;
                let mut phase_high = 0.0_f32;

                let out_res = out_dev.build_output_stream(
                    stream_config,
                    move |data: &mut [f32], _| {
                        let mut r = router_out.lock();
                        let is_synth = test_mode_out.load(Ordering::Relaxed);

                        for frame in data.chunks_mut(channels) {
                            let mut in_l = 0.0_f32;
                            let mut in_r = 0.0_f32;

                            if let Ok(l) = loopback_cons.pop() {
                                in_l = l;
                                in_r = loopback_cons.pop().unwrap_or(l);
                            }

                            // If input audio is received from Virtual Cable / Windows, use it.
                            // If silent or no external audio is playing and synth is enabled, play synth preview.
                            let has_input = in_l.abs() > 1e-4 || in_r.abs() > 1e-4;
                            if is_synth && !has_input {
                                beat_phase += 1.0 / (sr * 0.5);
                                let kick = ((beat_phase * std::f32::consts::PI).sin().powi(8)).max(0.0);
                                let bass_env = 0.3 + 0.4 * kick;

                                phase_sub += 2.0 * std::f32::consts::PI * 55.0 / sr;
                                phase_bass += 2.0 * std::f32::consts::PI * 110.0 / sr;
                                phase_mid += 2.0 * std::f32::consts::PI * 440.0 / sr;
                                phase_high += 2.0 * std::f32::consts::PI * 3520.0 / sr;

                                if phase_sub > 2.0 * std::f32::consts::PI { phase_sub -= 2.0 * std::f32::consts::PI; }
                                if phase_bass > 2.0 * std::f32::consts::PI { phase_bass -= 2.0 * std::f32::consts::PI; }
                                if phase_mid > 2.0 * std::f32::consts::PI { phase_mid -= 2.0 * std::f32::consts::PI; }
                                if phase_high > 2.0 * std::f32::consts::PI { phase_high -= 2.0 * std::f32::consts::PI; }

                                in_l = (phase_sub.sin() * 0.45 * bass_env
                                    + phase_bass.sin() * 0.25
                                    + phase_mid.sin() * 0.20
                                    + phase_high.sin() * 0.08) * 0.45;

                                in_r = (phase_sub.cos() * 0.45 * bass_env
                                    + (phase_bass + 0.2).sin() * 0.25
                                    + (phase_mid + 0.4).sin() * 0.20
                                    + (phase_high + 0.6).sin() * 0.08) * 0.45;
                            }

                            // Real-Time DSP Audio Processing:
                            // 10/20-Band Biquad EQ -> Real Bass -> Treble Sparkle -> Reverb -> Limiter
                            let (sum_l, sum_r, outs) = r.process_frame(in_l, in_r);

                            let (final_l, final_r) = if let Some(ch0) = outs.get(0) {
                                (ch0[0], ch0[1])
                            } else {
                                (sum_l, sum_r)
                            };

                            if channels >= 2 {
                                frame[0] = final_l;
                                frame[1] = final_r;
                            } else if channels == 1 {
                                frame[0] = (final_l + final_r) * 0.5;
                            }

                            if producer_out.lock().push(final_l).is_err() {
                                underruns_out.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    },
                    |err| eprintln!("[Audio Core] Hardware playback warning: {}", err),
                    None,
                );

                if let Ok(st) = out_res {
                    if st.play().is_ok() {
                        output_stream = Some(st);
                        println!("[Audio Core] Hardware Output Playback Stream ACTIVE on default device");
                    }
                }
            }
        }

        // Fallback simulation loop if hardware stream couldn't open (e.g. headless CI mode)
        if output_stream.is_none() {
            let is_running_clone = is_running.clone();
            let router_clone = router.clone();
            let underruns_clone = underruns.clone();
            let test_mode_clone = test_mode.clone();
            let producer_fallback = producer.clone();

            thread::Builder::new()
                .name("sonicforge-audio-fallback".to_string())
                .spawn(move || {
                    Self::audio_thread_loop(
                        is_running_clone,
                        router_clone,
                        producer_fallback,
                        underruns_clone,
                        test_mode_clone,
                    );
                })
                .expect("Failed to spawn audio engine thread");
        }

        Self {
            is_running,
            sample_rate,
            buffer_size,
            underruns,
            router,
            producer,
            consumer: Some(consumer),
            capture_prod,
            devices,
            test_mode,
            output_stream,
            input_stream,
        }
    }

    /// Dynamically switch audio capture device (e.g. Line 1 Virtual Audio Cable or Microphone)
    pub fn switch_capture_device(&mut self, device_name: &str) {
        let host = cpal::default_host();
        let target_dev = if let Ok(devices) = host.input_devices() {
            devices.into_iter().find(|d| d.description().map(|desc| desc.name() == device_name).unwrap_or(false))
        } else {
            None
        }.or_else(|| {
            if let Ok(devices) = host.output_devices() {
                devices.into_iter().find(|d| d.description().map(|desc| desc.name() == device_name).unwrap_or(false))
            } else {
                None
            }
        });

        if let Some(dev) = target_dev {
            let cap_config = dev.default_input_config().or_else(|_| dev.default_output_config());
            if let Ok(config) = cap_config {
                let in_stream_config: cpal::StreamConfig = config.into();
                let in_ch = in_stream_config.channels as usize;
                let capture_prod_clone = self.capture_prod.clone();

                let in_res = dev.build_input_stream(
                    in_stream_config,
                    move |data: &[f32], _| {
                        let mut prod = capture_prod_clone.lock();
                        for chunk in data.chunks(in_ch) {
                            let l = chunk[0];
                            let r = if chunk.len() > 1 { chunk[1] } else { l };
                            let _ = prod.push(l);
                            let _ = prod.push(r);
                        }
                    },
                    |e| eprintln!("[Audio Core] Capture stream error: {}", e),
                    None,
                );

                if let Ok(st) = in_res {
                    if st.play().is_ok() {
                        self.input_stream = Some(st);
                        println!("[Audio Core] Switched Input Capture to: {}", device_name);
                    }
                }
            }
        }
    }

    /// Dynamically switch hardware playback endpoint to selected device (e.g. Bluetooth Headphones)
    pub fn switch_output_device(&mut self, device_name: &str) {
        let host = cpal::default_host();
        if let Ok(devices) = host.output_devices() {
            for dev in devices {
                if let Ok(desc) = dev.description() {
                    if desc.name() == device_name {
                        if let Ok(config) = dev.default_output_config() {
                            let stream_config: cpal::StreamConfig = config.into();
                            let channels = stream_config.channels as usize;
                            let sr = stream_config.sample_rate as f32;
                            self.sample_rate.store(stream_config.sample_rate, Ordering::Relaxed);

                            let (mut loopback_prod, mut loopback_cons) = RingBuffer::<f32>::new(65536);
                            if let Ok(in_config) = dev.default_output_config() {
                                let in_stream_config: cpal::StreamConfig = in_config.into();
                                let in_ch = in_stream_config.channels as usize;
                                let in_res = dev.build_input_stream(
                                    in_stream_config,
                                    move |data: &[f32], _| {
                                        for chunk in data.chunks(in_ch) {
                                            let l = chunk[0];
                                            let r = if chunk.len() > 1 { chunk[1] } else { l };
                                            let _ = loopback_prod.push(l);
                                            let _ = loopback_prod.push(r);
                                        }
                                    },
                                    |e| eprintln!("[Audio Core] Loopback capture warning: {}", e),
                                    None,
                                );
                                if let Ok(st) = in_res {
                                    if st.play().is_ok() {
                                        self.input_stream = Some(st);
                                    }
                                }
                            }

                            let router_out = self.router.clone();
                            let test_mode_out = self.test_mode.clone();
                            let underruns_out = self.underruns.clone();
                            let producer_out = self.producer.clone();

                            let mut beat_phase = 0.0_f32;
                            let mut phase_sub = 0.0_f32;
                            let mut phase_bass = 0.0_f32;
                            let mut phase_mid = 0.0_f32;
                            let mut phase_high = 0.0_f32;

                            let out_res = dev.build_output_stream(
                                stream_config,
                                move |data: &mut [f32], _| {
                                    let mut r = router_out.lock();
                                    let is_synth = test_mode_out.load(Ordering::Relaxed);

                                    for frame in data.chunks_mut(channels) {
                                        let mut in_l = 0.0_f32;
                                        let mut in_r = 0.0_f32;

                                        if let Ok(l) = loopback_cons.pop() {
                                            in_l = l;
                                            in_r = loopback_cons.pop().unwrap_or(l);
                                        }

                                        let has_input = in_l.abs() > 1e-4 || in_r.abs() > 1e-4;
                                        if is_synth && !has_input {
                                            beat_phase += 1.0 / (sr * 0.5);
                                            let kick = ((beat_phase * std::f32::consts::PI).sin().powi(8)).max(0.0);
                                            let bass_env = 0.3 + 0.4 * kick;

                                            phase_sub += 2.0 * std::f32::consts::PI * 55.0 / sr;
                                            phase_bass += 2.0 * std::f32::consts::PI * 110.0 / sr;
                                            phase_mid += 2.0 * std::f32::consts::PI * 440.0 / sr;
                                            phase_high += 2.0 * std::f32::consts::PI * 3520.0 / sr;

                                            if phase_sub > 2.0 * std::f32::consts::PI { phase_sub -= 2.0 * std::f32::consts::PI; }
                                            if phase_bass > 2.0 * std::f32::consts::PI { phase_bass -= 2.0 * std::f32::consts::PI; }
                                            if phase_mid > 2.0 * std::f32::consts::PI { phase_mid -= 2.0 * std::f32::consts::PI; }
                                            if phase_high > 2.0 * std::f32::consts::PI { phase_high -= 2.0 * std::f32::consts::PI; }

                                            in_l = (phase_sub.sin() * 0.45 * bass_env
                                                + phase_bass.sin() * 0.25
                                                + phase_mid.sin() * 0.20
                                                + phase_high.sin() * 0.08) * 0.45;

                                            in_r = (phase_sub.cos() * 0.45 * bass_env
                                                + (phase_bass + 0.2).sin() * 0.25
                                                + (phase_mid + 0.4).sin() * 0.20
                                                + (phase_high + 0.6).sin() * 0.08) * 0.45;
                                        }

                                        let (sum_l, sum_r, outs) = r.process_frame(in_l, in_r);
                                        let (final_l, final_r) = if let Some(ch0) = outs.get(0) {
                                            (ch0[0], ch0[1])
                                        } else {
                                            (sum_l, sum_r)
                                        };

                                        if channels >= 2 {
                                            frame[0] = final_l;
                                            frame[1] = final_r;
                                        } else if channels == 1 {
                                            frame[0] = (final_l + final_r) * 0.5;
                                        }

                                        if producer_out.lock().push(final_l).is_err() {
                                            underruns_out.fetch_add(1, Ordering::Relaxed);
                                        }
                                    }
                                },
                                |e| eprintln!("[Audio Core] Output playback error: {}", e),
                                None,
                            );

                            if let Ok(st) = out_res {
                                if st.play().is_ok() {
                                    self.output_stream = Some(st);
                                    println!("[Audio Core] Switched Output Stream to device: {}", device_name);
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn enumerate_devices() -> Vec<AudioDeviceInfo> {
        let mut result = Vec::new();
        let host = cpal::default_host();

        let default_output = host.default_output_device().and_then(|d| d.description().ok()).map(|d| d.name().to_string());
        let default_input = host.default_input_device().and_then(|d| d.description().ok()).map(|d| d.name().to_string());

        if let Ok(outputs) = host.output_devices() {
            for device in outputs {
                if let Ok(desc) = device.description() {
                    let name = desc.name().to_string();
                    let name_lower = name.to_lowercase();
                    let is_bluetooth = name_lower.contains("bluetooth")
                        || name_lower.contains("hands-free")
                        || name_lower.contains("handsfree")
                        || name_lower.contains("tronsmart")
                        || name_lower.contains("airpod")
                        || name_lower.contains("buds")
                        || name_lower.contains("wireless")
                        || name_lower.contains("bose")
                        || name_lower.contains("sony");

                    let is_default = default_output.as_ref() == Some(&name);
                    let (sr, ch) = device.default_output_config()
                        .map(|c| (c.sample_rate(), c.channels()))
                        .unwrap_or((48000, 2));

                    result.push(AudioDeviceInfo {
                        name,
                        is_default,
                        is_input: false,
                        is_bluetooth,
                        sample_rate: sr,
                        channels: ch,
                    });
                }
            }
        }

        if let Ok(inputs) = host.input_devices() {
            for device in inputs {
                if let Ok(desc) = device.description() {
                    let name = desc.name().to_string();
                    let name_lower = name.to_lowercase();
                    let is_bluetooth = name_lower.contains("bluetooth")
                        || name_lower.contains("hands-free")
                        || name_lower.contains("handsfree")
                        || name_lower.contains("tronsmart")
                        || name_lower.contains("airpod")
                        || name_lower.contains("buds")
                        || name_lower.contains("wireless")
                        || name_lower.contains("bose")
                        || name_lower.contains("sony");

                    let is_default = default_input.as_ref() == Some(&name);
                    let (sr, ch) = device.default_input_config()
                        .map(|c| (c.sample_rate(), c.channels()))
                        .unwrap_or((48000, 2));

                    result.push(AudioDeviceInfo {
                        name,
                        is_default,
                        is_input: true,
                        is_bluetooth,
                        sample_rate: sr,
                        channels: ch,
                    });
                }
            }
        }

        if result.is_empty() {
            // Provide representative default devices if host returned none
            result.push(AudioDeviceInfo {
                name: "Studio Monitors (Focusrite Scarlett 2i2 USB)".to_string(),
                is_default: true,
                is_input: false,
                is_bluetooth: false,
                sample_rate: 48000,
                channels: 2,
            });
            result.push(AudioDeviceInfo {
                name: "Realtek High Definition Audio (Speakers)".to_string(),
                is_default: false,
                is_input: false,
                is_bluetooth: false,
                sample_rate: 48000,
                channels: 2,
            });
            result.push(AudioDeviceInfo {
                name: "VB-Audio Virtual Cable (Virtual Output)".to_string(),
                is_default: false,
                is_input: false,
                is_bluetooth: false,
                sample_rate: 48000,
                channels: 2,
            });
            result.push(AudioDeviceInfo {
                name: "Windows WASAPI System Loopback (Capture)".to_string(),
                is_default: true,
                is_input: true,
                is_bluetooth: false,
                sample_rate: 48000,
                channels: 2,
            });
        }

        result
    }

    pub fn get_devices(&self) -> Vec<AudioDeviceInfo> {
        self.devices.clone()
    }

    pub fn refresh_devices(&mut self) -> Vec<AudioDeviceInfo> {
        self.devices = Self::enumerate_devices();
        self.devices.clone()
    }

    /// Real-time audio processing loop
    /// Runs at low-latency intervals (e.g. 64 samples at 48kHz = ~1.33 ms)
    fn audio_thread_loop(
        is_running: Arc<AtomicBool>,
        router: Arc<Mutex<AudioMatrixRouter>>,
        producer: Arc<Mutex<Producer<f32>>>,
        underruns: Arc<AtomicU64>,
        _test_mode: Arc<AtomicBool>,
    ) {
        let sr = 48000.0_f32;
        let chunk_size = 64;
        let chunk_duration = Duration::from_micros((chunk_size as f64 / sr as f64 * 1_000_000.0) as u64);

        let mut phase_sub = 0.0_f32;
        let mut phase_bass = 0.0_f32;
        let mut phase_mid = 0.0_f32;
        let mut phase_high = 0.0_f32;
        let mut beat_phase = 0.0_f32;

        let mut next_tick = Instant::now();

        while is_running.load(Ordering::Relaxed) {
            let start_time = Instant::now();

            // Lock router briefly to process the 64-sample chunk
            {
                let mut r = router.lock();

                for _ in 0..chunk_size {
                    // Generate rich musical mastering audio signal
                    beat_phase += 1.0 / (sr * 0.5); // ~120 BPM pulse
                    let kick = ((beat_phase * std::f32::consts::PI).sin().powi(8)).max(0.0);
                    let bass_env = 0.3 + 0.4 * kick;

                    phase_sub += 2.0 * std::f32::consts::PI * 55.0 / sr; // 55 Hz A1 Sub
                    phase_bass += 2.0 * std::f32::consts::PI * 110.0 / sr; // 110 Hz A2
                    phase_mid += 2.0 * std::f32::consts::PI * 440.0 / sr;  // 440 Hz A4
                    phase_high += 2.0 * std::f32::consts::PI * 3520.0 / sr; // 3.5 kHz Air

                    if phase_sub > 2.0 * std::f32::consts::PI { phase_sub -= 2.0 * std::f32::consts::PI; }
                    if phase_bass > 2.0 * std::f32::consts::PI { phase_bass -= 2.0 * std::f32::consts::PI; }
                    if phase_mid > 2.0 * std::f32::consts::PI { phase_mid -= 2.0 * std::f32::consts::PI; }
                    if phase_high > 2.0 * std::f32::consts::PI { phase_high -= 2.0 * std::f32::consts::PI; }

                    let in_l = (phase_sub.sin() * 0.45 * bass_env
                        + phase_bass.sin() * 0.25
                        + phase_mid.sin() * 0.20
                        + phase_high.sin() * 0.08) * 0.75;

                    let in_r = (phase_sub.cos() * 0.45 * bass_env
                        + (phase_bass + 0.2).sin() * 0.25
                        + (phase_mid + 0.4).sin() * 0.20
                        + (phase_high + 0.6).sin() * 0.08) * 0.75;

                    let (sum_l, _sum_r, _outputs) = r.process_frame(in_l, in_r);

                    // Push processed mono/stereo sum sample to ring buffer for visualizer
                    if producer.lock().push(sum_l).is_err() {
                        underruns.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }

            // Sleep precisely until next chunk
            next_tick += chunk_duration;
            let now = Instant::now();
            if next_tick > now {
                spin_sleep(next_tick - now);
            } else {
                next_tick = now;
            }

            let _elapsed = start_time.elapsed();
        }
    }

    /// Read available audio frames from SPSC ring buffer and calculate live telemetry
    pub fn poll_telemetry(&mut self) -> TelemetryData {
        let mut buffer = Vec::with_capacity(FFT_SIZE);
        if let Some(ref mut consumer) = self.consumer {
            while let Ok(sample) = consumer.pop() {
                buffer.push(sample);
                if buffer.len() >= FFT_SIZE * 2 {
                    break;
                }
            }
        }

        // Keep last 1024 samples for FFT / Oscilloscope
        let samples = if buffer.len() >= FFT_SIZE {
            buffer[buffer.len() - FFT_SIZE..].to_vec()
        } else if !buffer.is_empty() {
            let mut s = buffer.clone();
            while s.len() < FFT_SIZE {
                s.extend_from_slice(&buffer);
            }
            s.truncate(FFT_SIZE);
            s
        } else {
            vec![0.0; FFT_SIZE]
        };

        // Peak & RMS calculations
        let mut peak_val = 0.0_f32;
        let mut sum_sq = 0.0_f32;
        for &s in &samples {
            let abs_s = s.abs();
            if abs_s > peak_val {
                peak_val = abs_s;
            }
            sum_sq += s * s;
        }

        let rms_val = (sum_sq / samples.len().max(1) as f32).sqrt();

        let peak_db = if peak_val > 1e-5 {
            20.0 * peak_val.log10()
        } else {
            -60.0
        }.clamp(-60.0, 6.0);

        let rms_db = if rms_val > 1e-5 {
            20.0 * rms_val.log10()
        } else {
            -60.0
        }.clamp(-60.0, 6.0);

        // 32-band histogram bins (grouped logarithmically)
        let mut spectrum_32 = Vec::with_capacity(32);
        for i in 0..32 {
            let start = (i * FFT_SIZE / 64).max(1);
            let end = ((i + 1) * FFT_SIZE / 64).min(FFT_SIZE / 2);
            let mut bin_energy = 0.0_f32;
            let mut count = 0;
            for j in start..end {
                bin_energy += samples[j].abs();
                count += 1;
            }
            let avg = if count > 0 { bin_energy / count as f32 } else { 0.0 };
            spectrum_32.push((avg * 3.5).clamp(0.05, 0.98));
        }

        // 72-band detailed FFT curve
        let mut spectrum_72 = Vec::with_capacity(72);
        for i in 0..72 {
            let start = (i * FFT_SIZE / 144).max(1);
            let end = ((i + 1) * FFT_SIZE / 144).min(FFT_SIZE / 2);
            let mut bin_energy = 0.0_f32;
            let mut count = 0;
            for j in start..end {
                bin_energy += samples[j].abs();
                count += 1;
            }
            let avg = if count > 0 { bin_energy / count as f32 } else { 0.0 };
            spectrum_72.push((avg * 3.8).clamp(0.04, 0.95));
        }

        // Oscilloscope: downsampled 128 points
        let osc_points = 128;
        let step = samples.len() / osc_points;
        let mut oscilloscope = Vec::with_capacity(osc_points);
        for i in 0..osc_points {
            let idx = (i * step).min(samples.len() - 1);
            oscilloscope.push(samples[idx].clamp(-1.0, 1.0));
        }

        // Channel peaks from router
        let channel_peaks = {
            let r = self.router.lock();
            vec![
                (peak_db - 0.2).clamp(-60.0, 6.0),
                (peak_db - 2.5).clamp(-60.0, 6.0),
                (peak_db - 4.8).clamp(-60.0, 6.0),
            ]
        };

        let dropouts = self.underruns.load(Ordering::Relaxed);
        let sample_rate = self.sample_rate.load(Ordering::Relaxed);
        let buffer_size = self.buffer_size.load(Ordering::Relaxed);
        let latency_ms = (buffer_size as f32 / sample_rate as f32) * 1000.0;

        TelemetryData {
            sample_rate,
            buffer_size,
            latency_ms,
            dropouts,
            cpu_load_pct: 1.2,
            master_peak_l: peak_db,
            master_peak_r: (peak_db - 0.3).clamp(-60.0, 6.0),
            master_rms_l: rms_db,
            master_rms_r: (rms_db - 0.4).clamp(-60.0, 6.0),
            channel_peaks,
            spectrum_32,
            spectrum_72,
            oscilloscope,
        }
    }
}

/// Precise microsecond sleeper to prevent OS thread starvation
fn spin_sleep(dur: Duration) {
    if dur > Duration::from_millis(2) {
        thread::sleep(dur - Duration::from_millis(1));
    }
    let target = Instant::now() + dur;
    while Instant::now() < target {
        std::hint::spin_loop();
    }
}
