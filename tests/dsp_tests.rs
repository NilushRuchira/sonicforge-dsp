//! Comprehensive Unit Tests for SonicForge DSP

use sonicforge_dsp::dsp::biquad::{Biquad, FilterType};
use sonicforge_dsp::dsp::equalizer::{Equalizer, EqMode, ISO_10_FREQS, ISO_20_FREQS};
use sonicforge_dsp::dsp::effects::{RealBass, TrebleSparkle, RealReverb, BrickwallLimiter};
use sonicforge_dsp::dsp::matrix::AudioMatrixRouter;
use sonicforge_dsp::audio::AudioEngine;

#[test]
fn test_biquad_unit_circle_stability() {
    let sample_rate = 48000.0;

    // Test a wide variety of frequencies and gains across 20 Hz to 22 kHz
    let test_freqs = [20.0, 31.0, 60.0, 100.0, 1000.0, 5000.0, 10000.0, 16000.0, 20000.0, 22000.0];
    let test_gains = [-18.0, -12.0, -6.0, 0.0, 3.0, 6.0, 12.0, 18.0];
    let test_qs = [0.1, 0.707, 1.414, 2.87, 5.0, 10.0];

    for &f in &test_freqs {
        for &g in &test_gains {
            for &q in &test_qs {
                let filter = Biquad::new(FilterType::Peaking { gain_db: g, q }, f, sample_rate);
                assert!(filter.is_stable(), "Peaking biquad at f={}Hz, g={}dB, q={} is unstable! a1={}, a2={}", f, g, q, filter.a1, filter.a2);

                let low_shelf = Biquad::new(FilterType::LowShelf { gain_db: g, q }, f, sample_rate);
                assert!(low_shelf.is_stable(), "LowShelf biquad at f={}Hz, g={}dB, q={} is unstable! a1={}, a2={}", f, g, q, low_shelf.a1, low_shelf.a2);

                let high_shelf = Biquad::new(FilterType::HighShelf { gain_db: g, q }, f, sample_rate);
                assert!(high_shelf.is_stable(), "HighShelf biquad at f={}Hz, g={}dB, q={} is unstable! a1={}, a2={}", f, g, q, high_shelf.a1, high_shelf.a2);
            }
        }
    }
}

#[test]
fn test_biquad_nan_inf_immunity() {
    let mut filter = Biquad::new(FilterType::Peaking { gain_db: 6.0, q: 1.414 }, 1000.0, 48000.0);

    // Feed extreme values: NaNs, Infinities, denormals
    let bad_inputs = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1e-38, 1e30, -1e30];

    for &bad in &bad_inputs {
        let out = filter.process_sample(0, bad);
        assert!(out.is_finite(), "Filter output exploded to non-finite value {} on input {}", out, bad);
    }

    // Filter must recover immediately when normal audio resumes
    for _ in 0..100 {
        let out = filter.process_sample(0, 0.5);
        assert!(out.is_finite());
        assert!(out.abs() < 5.0);
    }
}

#[test]
fn test_equalizer_modes_and_presets() {
    let mut eq = Equalizer::new(48000.0);

    // Test 10-band ISO frequencies
    assert_eq!(eq.mode(), EqMode::Bands10);
    assert_eq!(eq.get_gains().len(), 10);
    assert_eq!(ISO_10_FREQS.len(), 10);

    // Apply presets
    eq.apply_preset("bass");
    let bass_gains = eq.get_gains();
    assert!(bass_gains[0] > 5.0, "Bass punch preset should boost low frequencies");

    eq.apply_preset("vocal");
    let vocal_gains = eq.get_gains();
    assert!(vocal_gains[6] > 3.0, "Vocal air preset should boost high-mids");

    eq.apply_preset("flat");
    for &g in &eq.get_gains() {
        assert_eq!(g, 0.0);
    }

    // Switch to 20-band mode
    eq.set_mode(EqMode::Bands20);
    assert_eq!(eq.mode(), EqMode::Bands20);
    assert_eq!(eq.get_gains().len(), 20);
    assert_eq!(ISO_20_FREQS.len(), 20);

    // Test audio processing through all bands
    for i in 0..1000 {
        let t = i as f32 / 48000.0;
        let in_sample = (2.0 * std::f32::consts::PI * 1000.0 * t).sin();
        let out_sample = eq.process_sample(0, in_sample);
        assert!(out_sample.is_finite());
    }
}

#[test]
fn test_real_bass_soft_saturation() {
    let mut bass = RealBass::new(48000.0);
    bass.set_params(6.0, 80.0, 60.0, 4.5, true, true);

    // Process high-amplitude burst: soft-saturation cubic curve must prevent harsh clipping
    let (out_l, out_r) = bass.process_stereo(1.5, 1.5);
    assert!(out_l.is_finite());
    assert!(out_r.is_finite());
    assert!(out_l.abs() < 2.5, "Soft saturation must bound peak dynamics");
}

#[test]
fn test_treble_sparkle_and_tube_warmth() {
    let mut treble = TrebleSparkle::new(48000.0);
    treble.set_params(10000.0, 4.0, 3.0, 70.0);

    let (out_l, out_r) = treble.process_stereo(0.5, 0.5);
    assert!(out_l.is_finite());
    assert!(out_r.is_finite());
}

#[test]
fn test_reverb_schroeder_stability() {
    let mut reverb = RealReverb::new(48000.0);
    reverb.set_params(0.4, 0.8, 3.0, 0.3, 20.0, 1.4);

    // Send impulse
    let (out_l, out_r) = reverb.process_stereo(1.0, 1.0);
    assert!(out_l.is_finite());
    assert!(out_r.is_finite());

    // Follow tail for 20,000 samples (~0.4s)
    let mut max_tail = 0.0_f32;
    for _ in 0..20000 {
        let (tail_l, tail_r) = reverb.process_stereo(0.0, 0.0);
        assert!(tail_l.is_finite());
        assert!(tail_r.is_finite());
        if tail_l.abs() > max_tail { max_tail = tail_l.abs(); }
    }
    assert!(max_tail < 2.0, "Reverb tail must remain bounded without runaway feedback");
}

#[test]
fn test_brickwall_limiter_ceiling() {
    let limiter = BrickwallLimiter::new();
    let ceiling = 10.0_f32.powf(-0.1 / 20.0); // ~0.98855

    // Over-driven inputs
    for amp in [1.0, 1.5, 2.0, 5.0, 10.0] {
        let (l, r) = limiter.process_stereo(amp, -amp);
        assert!(l <= ceiling + 1e-4, "Limiter failed: {} exceeded ceiling {}", l, ceiling);
        assert!(r >= -(ceiling + 1e-4), "Limiter failed: {} exceeded negative ceiling {}", r, -ceiling);
    }
}

#[test]
fn test_eq_slider_audio_change() {
    let mut router = AudioMatrixRouter::new(48000.0);
    let sr = 48000.0_f32;
    let freq = 63.0_f32;

    // 1. Process at 0 dB (Flat)
    let mut sum_flat = 0.0_f32;
    for i in 0..15000 {
        let s = (2.0 * std::f32::consts::PI * freq * i as f32 / sr).sin() * 0.05;
        let (_, _, outs) = router.process_frame(s, s);
        if i >= 10000 {
            sum_flat += outs[0][0].powi(2);
        }
    }
    let rms_flat = (sum_flat / 5000.0).sqrt();

    // 2. Boost 63 Hz band (index 1) to +12 dB
    router.master_eq.set_band_gain(1, 12.0);

    let mut sum_boosted = 0.0_f32;
    for i in 0..15000 {
        let s = (2.0 * std::f32::consts::PI * freq * i as f32 / sr).sin() * 0.05;
        let (_, _, outs) = router.process_frame(s, s);
        if i >= 10000 {
            sum_boosted += outs[0][0].powi(2);
        }
    }
    let rms_boosted = (sum_boosted / 5000.0).sqrt();

    println!("RMS Flat: {}, RMS Boosted: {}", rms_flat, rms_boosted);
    assert!(rms_boosted > rms_flat * 2.0, "Boosted RMS ({}) must be significantly louder than Flat RMS ({})", rms_boosted, rms_flat);
}

#[test]
fn test_matrix_router_dispatch() {
    let mut router = AudioMatrixRouter::new(48000.0);
    assert_eq!(router.channels.len(), 3);

    let (sum_l, sum_r, outputs) = router.process_frame(0.4, 0.4);
    assert!(sum_l.is_finite());
    assert!(sum_r.is_finite());
    assert_eq!(outputs.len(), 3);

    // Test Mute channel 0
    router.channels[0].is_muted = true;
    let (_, _, muted_outs) = router.process_frame(0.4, 0.4);
    assert_eq!(muted_outs[0], [0.0, 0.0], "Muted channel must output silence");
}

#[test]
fn test_output_stream_playback() {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let host = cpal::default_host();
    if let Some(device) = host.default_output_device() {
        if let Ok(desc) = device.description() {
            println!("Default output device for playback test: {:?}", desc.name());
        }
        if let Ok(config) = device.default_output_config() {
            println!("Supported format: {:?}", config);
            let sample_format = config.sample_format();
            let stream_config: cpal::StreamConfig = config.into();
            let channels = stream_config.channels as usize;

            let mut phase = 0.0_f32;
            let err_fn = |e| eprintln!("Stream error: {}", e);

            let res = match sample_format {
                cpal::SampleFormat::F32 => {
                    device.build_output_stream(
                        stream_config.clone(),
                        move |data: &mut [f32], _| {
                            for frame in data.chunks_mut(channels) {
                                phase += 2.0 * std::f32::consts::PI * 440.0 / 48000.0;
                                if phase > 2.0 * std::f32::consts::PI { phase -= 2.0 * std::f32::consts::PI; }
                                let s = phase.sin() * 0.05;
                                for sample in frame.iter_mut() {
                                    *sample = s;
                                }
                            }
                        },
                        err_fn,
                        None,
                    )
                }
                cpal::SampleFormat::I16 => {
                    device.build_output_stream(
                        stream_config,
                        move |data: &mut [i16], _| {
                            for frame in data.chunks_mut(channels) {
                                phase += 2.0 * std::f32::consts::PI * 440.0 / 48000.0;
                                if phase > 2.0 * std::f32::consts::PI { phase -= 2.0 * std::f32::consts::PI; }
                                let s = (phase.sin() * 0.05 * 32767.0) as i16;
                                for sample in frame.iter_mut() {
                                    *sample = s;
                                }
                            }
                        },
                        err_fn,
                        None,
                    )
                }
                _ => panic!("Unsupported format"),
            };

            assert!(res.is_ok(), "build_output_stream failed: {:?}", res.err());
            if let Ok(stream) = res {
                assert!(stream.play().is_ok());
                std::thread::sleep(std::time::Duration::from_millis(50));
            }

            // Test loopback capture stream on the same device
            let in_stream_config: cpal::StreamConfig = config.into();
            let in_res = device.build_input_stream(
                in_stream_config,
                move |_data: &[f32], _| {},
                |e| eprintln!("Input error: {}", e),
                None,
            );
            println!("Loopback input build result: {:?}", in_res.is_ok());
        }
    }
}
