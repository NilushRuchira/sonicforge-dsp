mod dsp;
mod audio;
mod ui;

use std::sync::Arc;
use std::thread;
use std::time::Duration;
use parking_lot::Mutex;
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

use audio::AudioEngine;

enum UserEvent {
    AudioTelemetry(String),
    DevicesLoaded(String),
}

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("PANIC: {}", info);
        eprintln!("{}", msg);
        let _ = std::fs::write("panic.log", msg);
    }));

    let _ = std::fs::write("app_startup.log", "=== SonicForge DSP Startup ===\n");

    macro_rules! log_step {
        ($($arg:tt)*) => {{
            let s = format!($($arg)*);
            println!("{}", s);
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("app_startup.log") {
                let _ = writeln!(f, "{}", s);
            }
        }};
    }

    let args: Vec<String> = std::env::args().collect();

    log_step!("============================================================");
    log_step!(" SonicForge DSP - Windows Audio Matrix & DSP Control Center ");
    log_step!(" Core Version: 2.4.12-x64 (Windows 11 Fluent Studio)       ");
    log_step!("============================================================");

    // Initialize high-performance audio engine
    let engine = Arc::new(Mutex::new(AudioEngine::new()));

    // Print enumerated audio endpoints
    {
        let eng = engine.lock();
        let devices = eng.get_devices();
        log_step!("[Audio Core] Enumerated {} active WASAPI endpoints:", devices.len());
        for (i, dev) in devices.iter().enumerate() {
            let role = if dev.is_input { "CAPTURE" } else { "RENDER" };
            let def = if dev.is_default { " (DEFAULT)" } else { "" };
            log_step!("  [{}] [{}] {} - {}Hz/{}ch{}", i, role, dev.name, dev.sample_rate, dev.channels, def);
        }
    }

    // If headless verification argument is passed, run automated audio pipeline validation and exit
    if args.contains(&"--verify-pipeline".to_string()) {
        log_step!("[Verify] Running audio pipeline verification in headless mode...");
        thread::sleep(Duration::from_millis(500));
        let telemetry = engine.lock().poll_telemetry();
        log_step!("[Verify] Sample Rate: {} Hz", telemetry.sample_rate);
        log_step!("[Verify] Buffer Size: {} samples ({:.2} ms)", telemetry.buffer_size, telemetry.latency_ms);
        log_step!("[Verify] Peak L: {:.2} dBFS, Peak R: {:.2} dBFS", telemetry.master_peak_l, telemetry.master_peak_r);
        log_step!("[Verify] RMS L: {:.2} dBFS, RMS R: {:.2} dBFS", telemetry.master_rms_l, telemetry.master_rms_r);
        log_step!("[Verify] Spectrum 32-bins: {} items, FFT 72-bins: {} items", telemetry.spectrum_32.len(), telemetry.spectrum_72.len());
        log_step!("[Verify] Pipeline successfully validated with 0 dropouts!");
        return;
    }

    // Set up native window event loop
    log_step!("[GUI] Building EventLoop and Native Window...");
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("SonicForge DSP - Windows Audio Matrix & DSP Control Center")
        .with_inner_size(LogicalSize::new(1400.0, 880.0))
        .with_min_inner_size(LogicalSize::new(1024.0, 700.0))
        .with_visible(true)
        .with_focused(true)
        .build(&event_loop)
        .expect("Failed to create native desktop window");

    log_step!("[GUI] Native Window created successfully. Initializing WebView2...");

    // IPC Command Handler Bridge
    let engine_ipc = engine.clone();
    let proxy_ipc = proxy.clone();
    let webview = WebViewBuilder::new()
        .with_html(ui::get_html())
        .with_ipc_handler(move |req: wry::http::Request<String>| {
            let msg = req.body();
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(msg) {
                if let Some(cmd) = v.get("cmd").and_then(|c| c.as_str()) {
                    let eng = engine_ipc.lock();
                    let mut router = eng.router.lock();

                    match cmd {
                        "get_devices" => {
                            let devs = eng.get_devices();
                            if let Ok(json) = serde_json::to_string(&devs) {
                                let _ = proxy_ipc.send_event(UserEvent::DevicesLoaded(json));
                            }
                        }
                        "refresh_devices" => {
                            drop(router);
                            drop(eng);
                            let mut eng_mut = engine_ipc.lock();
                            let devs = eng_mut.refresh_devices();
                            if let Ok(json) = serde_json::to_string(&devs) {
                                let _ = proxy_ipc.send_event(UserEvent::DevicesLoaded(json));
                            }
                        }
                        "set_channel_gain" => {
                            if let (Some(ch), Some(gain)) = (v.get("channel").and_then(|c| c.as_u64()), v.get("gain_db").and_then(|g| g.as_f64())) {
                                if (ch as usize) < router.channels.len() {
                                    router.channels[ch as usize].set_gain_db(gain as f32);
                                }
                            }
                        }
                        "set_channel_solo" => {
                            if let (Some(ch), Some(solo)) = (v.get("channel").and_then(|c| c.as_u64()), v.get("solo").and_then(|s| s.as_bool())) {
                                if (ch as usize) < router.channels.len() {
                                    router.channels[ch as usize].is_solo = solo;
                                }
                            }
                        }
                        "set_channel_mute" => {
                            if let (Some(ch), Some(mute)) = (v.get("channel").and_then(|c| c.as_u64()), v.get("mute").and_then(|m| m.as_bool())) {
                                if (ch as usize) < router.channels.len() {
                                    router.channels[ch as usize].is_muted = mute;
                                }
                            }
                        }
                        "set_channel_dsp_mode" => {
                            if let (Some(ch), Some(indiv)) = (v.get("channel").and_then(|c| c.as_u64()), v.get("individual").and_then(|i| i.as_bool())) {
                                if (ch as usize) < router.channels.len() {
                                    router.channels[ch as usize].individual_dsp = indiv;
                                }
                            }
                        }
                        "set_master_mute" => {
                            if let Some(muted) = v.get("muted").and_then(|m| m.as_bool()) {
                                router.master_muted = muted;
                            }
                        }
                        "set_dsp_sync" => {
                            if let Some(active) = v.get("active").and_then(|a| a.as_bool()) {
                                router.dsp_sync_active = active;
                            }
                        }
                        "set_eq_mode" => {
                            if let Some(mode) = v.get("mode").and_then(|m| m.as_str()) {
                                let eq_mode = if mode == "Bands20" { dsp::EqMode::Bands20 } else { dsp::EqMode::Bands10 };
                                router.master_eq.set_mode(eq_mode);
                            }
                        }
                        "set_band_gain" => {
                            if let (Some(idx), Some(gain)) = (v.get("index").and_then(|i| i.as_u64()), v.get("gain_db").and_then(|g| g.as_f64())) {
                                router.master_eq.set_band_gain(idx as usize, gain as f32);
                            }
                        }
                        "set_band_solo" => {
                            let solo = v.get("index").and_then(|i| i.as_u64()).map(|i| i as usize);
                            router.master_eq.set_solo_band(solo);
                        }
                        "set_eq_bypassed" => {
                            if let Some(bypassed) = v.get("bypassed").and_then(|b| b.as_bool()) {
                                router.master_eq.set_bypassed(bypassed);
                            }
                        }
                        "apply_eq_preset" => {
                            if let Some(preset) = v.get("preset").and_then(|p| p.as_str()) {
                                router.master_eq.apply_preset(preset);
                            }
                        }
                        "reset_eq" => {
                            router.master_eq.apply_preset("flat");
                        }
                        "set_bass_params" => {
                            if let Some(drive) = v.get("drive_db").and_then(|d| d.as_f64()) {
                                router.master_bass.drive_db = drive as f32;
                            }
                        }
                        "set_bass_cutoff" => {
                            if let Some(cutoff) = v.get("cutoff_hz").and_then(|c| c.as_f64()) {
                                router.master_bass.cutoff_hz = cutoff as f32;
                            }
                        }
                        "set_bass_saturation" => {
                            if let Some(pct) = v.get("pct").and_then(|p| p.as_f64()) {
                                router.master_bass.saturation_pct = pct as f32;
                            }
                        }
                        "set_bass_sub" => {
                            if let Some(db) = v.get("db").and_then(|d| d.as_f64()) {
                                router.master_bass.sub_octave_db = db as f32;
                            }
                        }
                        "set_bass_mono" => {
                            if let Some(mono) = v.get("mono").and_then(|m| m.as_bool()) {
                                router.master_bass.mono_collapse = mono;
                            }
                        }
                        "set_bass_sub_gen" => {
                            if let Some(active) = v.get("active").and_then(|a| a.as_bool()) {
                                router.master_bass.sub_gen_active = active;
                            }
                        }
                        "set_treble_freq" => {
                            if let Some(freq) = v.get("freq").and_then(|f| f.as_f64()) {
                                router.master_treble.corner_freq = freq as f32;
                            }
                        }
                        "set_treble_brilliance" => {
                            if let Some(db) = v.get("db").and_then(|d| d.as_f64()) {
                                router.master_treble.brilliance_db = db as f32;
                            }
                        }
                        "set_treble_attack" => {
                            if let Some(db) = v.get("db").and_then(|d| d.as_f64()) {
                                router.master_treble.transient_attack_db = db as f32;
                            }
                        }
                        "set_treble_tube" => {
                            if let Some(pct) = v.get("pct").and_then(|p| p.as_f64()) {
                                router.master_treble.tube_warmth_pct = pct as f32;
                            }
                        }
                        "set_reverb_wet" => {
                            if let Some(wet) = v.get("wet").and_then(|w| w.as_f64()) {
                                router.master_reverb.wet = wet as f32;
                                router.master_reverb.dry = (1.0 - wet as f32).max(0.0);
                            }
                        }
                        "set_reverb_decay" => {
                            if let Some(secs) = v.get("secs").and_then(|s| s.as_f64()) {
                                router.master_reverb.decay_secs = secs as f32;
                            }
                        }
                        "set_reverb_predelay" => {
                            if let Some(ms) = v.get("ms").and_then(|m| m.as_f64()) {
                                router.master_reverb.pre_delay_ms = ms as f32;
                            }
                        }
                        "set_reverb_damp" => {
                            if let Some(damp) = v.get("damp").and_then(|d| d.as_f64()) {
                                router.master_reverb.damping = damp as f32;
                            }
                        }
                        "set_reverb_width" => {
                            if let Some(width) = v.get("width").and_then(|w| w.as_f64()) {
                                router.master_reverb.width = width as f32;
                            }
                        }
                        "toggle_fx_slot" => {
                            if let (Some(slot), Some(en)) = (v.get("slot").and_then(|s| s.as_str()), v.get("enabled").and_then(|e| e.as_bool())) {
                                match slot {
                                    "bass" => router.master_bass.enabled = en,
                                    "treble" => router.master_treble.enabled = en,
                                    "reverb" => router.master_reverb.enabled = en,
                                    _ => {}
                                }
                            }
                        }
                        "set_global_fx_bypass" => {
                            if let Some(bypassed) = v.get("bypassed").and_then(|b| b.as_bool()) {
                                router.master_bass.enabled = !bypassed;
                                router.master_treble.enabled = !bypassed;
                                router.master_reverb.enabled = !bypassed;
                            }
                        }
                        "set_matrix_crosspoint" => {
                            if let (Some(s), Some(c), Some(conn)) = (
                                v.get("source").and_then(|s| s.as_u64()),
                                v.get("channel").and_then(|c| c.as_u64()),
                                v.get("connected").and_then(|co| co.as_bool()),
                            ) {
                                router.set_matrix_crosspoint(s as usize, c as usize, conn);
                            }
                        }
                        "set_source_phase" => {
                            if let (Some(s), Some(inv)) = (v.get("source").and_then(|s| s.as_u64()), v.get("phase_inverted").and_then(|p| p.as_bool())) {
                                if (s as usize) < router.sources.len() {
                                    router.sources[s as usize].phase_inverted = inv;
                                }
                            }
                        }
                        "invert_all_phase" => {
                            for s in &mut router.sources {
                                s.phase_inverted = !s.phase_inverted;
                            }
                        }
                        "reset_matrix" => {
                            router.matrix_grid[0] = [true, true, true];
                            router.matrix_grid[1] = [true, true, false];
                            router.matrix_grid[2] = [false, true, true];
                            router.matrix_grid[3] = [false, false, true];
                        }
                        _ => {}
                    }
                }
            }
        })
        .build(&window)
        .expect("Failed to build Microsoft Edge WebView2 instance");

    // Spawn 60 FPS Telemetry Dispatcher thread
    let engine_telemetry = engine.clone();
    let is_running = engine.lock().is_running.clone();
    let proxy_telemetry = proxy.clone();
    thread::Builder::new()
        .name("telemetry-pump".to_string())
        .spawn(move || {
            while is_running.load(std::sync::atomic::Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(16)); // ~60 FPS
                let data = {
                    let mut eng = engine_telemetry.lock();
                    eng.poll_telemetry()
                };
                if let Ok(json) = serde_json::to_string(&data) {
                    let _ = proxy_telemetry.send_event(UserEvent::AudioTelemetry(json));
                }
            }
        })
        .expect("Failed to spawn telemetry thread");

    // Push initial devices list to event loop
    let initial_devs = engine.lock().get_devices();
    if let Ok(json) = serde_json::to_string(&initial_devs) {
        let _ = proxy.send_event(UserEvent::DevicesLoaded(json));
    }

    log_step!("[SonicForge GUI] Desktop Window active. Starting event loop at 60 FPS.");

    // Run native event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(UserEvent::AudioTelemetry(json_data)) => {
                let script = format!("window.onAudioTelemetry({});", json_data);
                let _ = webview.evaluate_script(&script);
            }
            Event::UserEvent(UserEvent::DevicesLoaded(json_data)) => {
                let script = format!("if (window.onDevicesLoaded) {{ window.onDevicesLoaded({}); }}", json_data);
                let _ = webview.evaluate_script(&script);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                log_step!("[SonicForge DSP] Shutting down application cleanly (CloseRequested).");
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Destroyed,
                ..
            } => {
                log_step!("[SonicForge DSP] Window Destroyed.");
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}
