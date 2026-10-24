//! Multi-Channel Matrix Routing, Crosspoint Matrix & Output Mixer

use super::equalizer::Equalizer;
use super::effects::{RealBass, TrebleSparkle, RealReverb, BrickwallLimiter};

pub const NUM_OUTPUT_CHANNELS: usize = 3;
pub const NUM_INPUT_SOURCES: usize = 4;

#[derive(Debug, Clone)]
pub struct OutputChannelStrip {
    pub name: String,
    pub endpoint_name: String,
    pub format_desc: String,
    pub fader_gain_db: f32, // -inf (-60dB) to +6 dB, 0.0 dB default
    pub linear_gain: f32,
    pub is_muted: bool,
    pub is_solo: bool,
    pub phase_inverted: bool,
    pub individual_dsp: bool,

    // Per-channel dedicated DSP processing pipeline
    pub equalizer: Equalizer,
    pub real_bass: RealBass,
    pub treble_sparkle: TrebleSparkle,
    pub reverb: RealReverb,
    pub limiter: BrickwallLimiter,

    // Live output telemetry
    pub meter_peak_db: f32,
    pub meter_rms_db: f32,
}

impl OutputChannelStrip {
    pub fn new(name: &str, endpoint: &str, format: &str, sample_rate: f32) -> Self {
        Self {
            name: name.to_string(),
            endpoint_name: endpoint.to_string(),
            format_desc: format.to_string(),
            fader_gain_db: -1.4,
            linear_gain: 10.0_f32.powf(-1.4 / 20.0),
            is_muted: false,
            is_solo: false,
            phase_inverted: false,
            individual_dsp: true,
            equalizer: Equalizer::new(sample_rate),
            real_bass: RealBass::new(sample_rate),
            treble_sparkle: TrebleSparkle::new(sample_rate),
            reverb: RealReverb::new(sample_rate),
            limiter: BrickwallLimiter::new(),
            meter_peak_db: -60.0,
            meter_rms_db: -60.0,
        }
    }

    pub fn set_gain_db(&mut self, db: f32) {
        self.fader_gain_db = db.clamp(-60.0, 6.0);
        self.linear_gain = if self.fader_gain_db <= -59.5 {
            0.0
        } else {
            10.0_f32.powf(self.fader_gain_db / 20.0)
        };
    }
}

#[derive(Debug, Clone)]
pub struct InputSourceNode {
    pub id: String,
    pub name: String,
    pub subtitle: String,
    pub trim_db: f32,
    pub linear_trim: f32,
    pub phase_inverted: bool,
    pub peak_db: f32,
}

impl InputSourceNode {
    pub fn new(id: &str, name: &str, subtitle: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            subtitle: subtitle.to_string(),
            trim_db: 0.0,
            linear_trim: 1.0,
            phase_inverted: false,
            peak_db: -14.2,
        }
    }

    pub fn set_trim(&mut self, db: f32) {
        self.trim_db = db.clamp(-24.0, 12.0);
        self.linear_trim = 10.0_f32.powf(self.trim_db / 20.0);
    }
}

#[derive(Debug, Clone)]
pub struct AudioMatrixRouter {
    pub channels: Vec<OutputChannelStrip>,
    pub sources: Vec<InputSourceNode>,
    // 4 sources x 3 output channels routing crosspoint grid (true = connected)
    pub matrix_grid: [[bool; NUM_OUTPUT_CHANNELS]; NUM_INPUT_SOURCES],

    // Master bus
    pub master_gain_db: f32,
    pub master_linear_gain: f32,
    pub master_muted: bool,
    pub dsp_sync_active: bool, // when true, DSP settings sync across all channels

    // Master DSP instances (used when dsp_sync_active is true)
    pub master_eq: Equalizer,
    pub master_bass: RealBass,
    pub master_treble: TrebleSparkle,
    pub master_reverb: RealReverb,
    pub master_limiter: BrickwallLimiter,

    // Master telemetry
    pub master_peak_l: f32,
    pub master_peak_r: f32,
    pub master_rms_l: f32,
    pub master_rms_r: f32,
}

impl AudioMatrixRouter {
    pub fn new(sample_rate: f32) -> Self {
        let ch0 = OutputChannelStrip::new(
            "Studio Monitors",
            "USB DAC - Focusrite Scarlett 2i2",
            "48kHz • 24-bit",
            sample_rate,
        );
        let mut ch1 = OutputChannelStrip::new(
            "Headphone Out",
            "Realtek High Definition Audio",
            "48kHz • 24-bit",
            sample_rate,
        );
        ch1.set_gain_db(-3.8);

        let mut ch2 = OutputChannelStrip::new(
            "Virtual Broadcast Cable",
            "OBS Studio / Discord Routing Aux",
            "48kHz • 24-bit",
            sample_rate,
        );
        ch2.set_gain_db(-6.0);

        let channels = vec![ch0, ch1, ch2];

        let sources = vec![
            InputSourceNode::new("win-loopback", "Windows System Loopback", "Src 01 • Master OS Stream"),
            InputSourceNode::new("spotify-session", "Spotify / Media Session", "Src 02 • High-Res Lossless"),
            InputSourceNode::new("discord-comms", "Discord / Communications", "Src 03 • Voice Call Sink"),
            InputSourceNode::new("mic-input", "Microphone / Analog Line In", "Src 04 • Intercept Node"),
        ];

        // Default routing:
        // Src 0 (Loopback) -> All 3 outputs
        // Src 1 (Media) -> Monitors & Headphones
        // Src 2 (Discord) -> Headphones & Virtual Cable
        // Src 3 (Mic) -> Virtual Cable
        let mut matrix_grid = [[false; NUM_OUTPUT_CHANNELS]; NUM_INPUT_SOURCES];
        matrix_grid[0] = [true, true, true];   // Loopback -> Out 0, 1, 2
        matrix_grid[1] = [true, true, false];  // Spotify -> Out 0, 1
        matrix_grid[2] = [false, true, true];  // Discord -> Out 1, 2
        matrix_grid[3] = [false, false, true]; // Mic -> Out 2

        Self {
            channels,
            sources,
            matrix_grid,
            master_gain_db: -0.4,
            master_linear_gain: 10.0_f32.powf(-0.4 / 20.0),
            master_muted: false,
            dsp_sync_active: true,
            master_eq: Equalizer::new(sample_rate),
            master_bass: RealBass::new(sample_rate),
            master_treble: TrebleSparkle::new(sample_rate),
            master_reverb: RealReverb::new(sample_rate),
            master_limiter: BrickwallLimiter::new(),
            master_peak_l: -60.0,
            master_peak_r: -60.0,
            master_rms_l: -60.0,
            master_rms_r: -60.0,
        }
    }

    pub fn set_master_gain(&mut self, db: f32) {
        self.master_gain_db = db.clamp(-60.0, 6.0);
        self.master_linear_gain = if self.master_gain_db <= -59.5 {
            0.0
        } else {
            10.0_f32.powf(self.master_gain_db / 20.0)
        };
    }

    pub fn set_matrix_crosspoint(&mut self, src_idx: usize, ch_idx: usize, connected: bool) {
        if src_idx < NUM_INPUT_SOURCES && ch_idx < NUM_OUTPUT_CHANNELS {
            self.matrix_grid[src_idx][ch_idx] = connected;
        }
    }

    /// Process an input sample frame across all matrix channels and outputs
    /// Returns per-output stereo audio frames: Vec<[left, right]>
    #[inline(always)]
    pub fn process_frame(&mut self, in_l: f32, in_r: f32) -> (f32, f32, Vec<[f32; 2]>) {
        if self.master_muted {
            let zero_outs = vec![[0.0, 0.0]; self.channels.len()];
            return (0.0, 0.0, zero_outs);
        }

        // Apply master DSP chain if sync is active
        let (mut sum_l, mut sum_r) = if self.dsp_sync_active {
            let eq_l = self.master_eq.process_sample(0, in_l);
            let eq_r = self.master_eq.process_sample(1, in_r);
            let (b_l, b_r) = self.master_bass.process_stereo(eq_l, eq_r);
            let (t_l, t_r) = self.master_treble.process_stereo(b_l, b_r);
            let (rv_l, rv_r) = self.master_reverb.process_stereo(t_l, t_r);
            self.master_limiter.process_stereo(rv_l, rv_r)
        } else {
            (in_l, in_r)
        };

        sum_l *= self.master_linear_gain;
        sum_r *= self.master_linear_gain;

        // Check if any channel has Solo active
        let any_solo = self.channels.iter().any(|c| c.is_solo);

        let mut output_frames = Vec::with_capacity(self.channels.len());

        for ch in &mut self.channels {
            if ch.is_muted || (any_solo && !ch.is_solo) {
                output_frames.push([0.0, 0.0]);
                continue;
            }

            let mut out_l = sum_l;
            let mut out_r = sum_r;

            // Per-channel DSP if not synced
            if !self.dsp_sync_active && ch.individual_dsp {
                let eq_l = ch.equalizer.process_sample(0, out_l);
                let eq_r = ch.equalizer.process_sample(1, out_r);
                let (b_l, b_r) = ch.real_bass.process_stereo(eq_l, eq_r);
                let (t_l, t_r) = ch.treble_sparkle.process_stereo(b_l, b_r);
                let (rv_l, rv_r) = ch.reverb.process_stereo(t_l, t_r);
                let (lim_l, lim_r) = ch.limiter.process_stereo(rv_l, rv_r);
                out_l = lim_l;
                out_r = lim_r;
            }

            // Channel Fader Gain
            out_l *= ch.linear_gain;
            out_r *= ch.linear_gain;

            // Phase Invert
            if ch.phase_inverted {
                out_l = -out_l;
                out_r = -out_r;
            }

            output_frames.push([out_l, out_r]);
        }

        (sum_l, sum_r, output_frames)
    }
}
