//! Audio DSP Effects Rack: Real Bass, Treble Sparkle, Real Reverb & Limiter

use super::biquad::{Biquad, FilterType};

// ============================================================================
// 1. Real Bass & Sub-Harmonic Synthesizer
// ============================================================================

#[derive(Debug, Clone)]
pub struct RealBass {
    pub enabled: bool,
    pub drive_db: f32,          // 0 to +18 dB
    pub cutoff_hz: f32,         // 60, 80, or 100 Hz
    pub saturation_pct: f32,    // 0% to 100%
    pub sub_octave_db: f32,     // 0 to +12 dB
    pub mono_collapse: bool,    // Sum to mono < 120 Hz
    pub sub_gen_active: bool,   // Enable sub-octave generator

    shelf_filter: Biquad,
    mono_filter: Biquad,
    sub_lpf: Biquad,
    sample_rate: f32,

    // Sub-octave generator state
    last_sign: [f32; 2],
    flip_flop: [f32; 2],
}

impl RealBass {
    pub fn new(sample_rate: f32) -> Self {
        let sr = if sample_rate > 0.0 { sample_rate } else { 48000.0 };
        let shelf = Biquad::new(FilterType::LowShelf { gain_db: 4.2, q: 0.707 }, 80.0, sr);
        let mono_f = Biquad::new(FilterType::HighPass { q: 0.707 }, 120.0, sr);
        let sub_lpf = Biquad::new(FilterType::LowPass { q: 0.707 }, 90.0, sr);

        Self {
            enabled: true,
            drive_db: 4.2,
            cutoff_hz: 80.0,
            saturation_pct: 45.0,
            sub_octave_db: 4.5,
            mono_collapse: true,
            sub_gen_active: true,
            shelf_filter: shelf,
            mono_filter: mono_f,
            sub_lpf,
            sample_rate: sr,
            last_sign: [0.0; 2],
            flip_flop: [1.0, 1.0],
        }
    }

    pub fn set_params(&mut self, drive_db: f32, cutoff_hz: f32, sat_pct: f32, sub_db: f32, mono: bool, sub_gen: bool) {
        self.drive_db = drive_db.clamp(0.0, 18.0);
        self.cutoff_hz = cutoff_hz.clamp(40.0, 200.0);
        self.saturation_pct = sat_pct.clamp(0.0, 100.0);
        self.sub_octave_db = sub_db.clamp(0.0, 12.0);
        self.mono_collapse = mono;
        self.sub_gen_active = sub_gen;

        self.shelf_filter.set_params(
            FilterType::LowShelf { gain_db: self.drive_db, q: 0.707 },
            self.cutoff_hz,
            self.sample_rate,
        );
    }

    /// Process stereo sample pair [left, right]
    #[inline(always)]
    pub fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        if !self.enabled {
            return (left, right);
        }

        let mut l = self.shelf_filter.process_sample(0, left);
        let mut r = self.shelf_filter.process_sample(1, right);

        // Sub-octave generation
        if self.sub_gen_active && self.sub_octave_db > 0.0 {
            let sub_gain = 10.0_f32.powf(self.sub_octave_db / 20.0) * 0.25;
            for (ch, sample) in [l, r].iter().enumerate() {
                // Zero-crossing detector
                let current_sign = if *sample >= 0.0 { 1.0 } else { -1.0 };
                if current_sign > 0.0 && self.last_sign[ch] <= 0.0 {
                    self.flip_flop[ch] = -self.flip_flop[ch];
                }
                self.last_sign[ch] = current_sign;

                let sub_raw = self.flip_flop[ch] * sample.abs();
                let sub_clean = self.sub_lpf.process_sample(ch, sub_raw);
                if ch == 0 {
                    l += sub_clean * sub_gain;
                } else {
                    r += sub_clean * sub_gain;
                }
            }
        }

        // Analog soft saturation: f(x) = x - x^3 / 3
        if self.saturation_pct > 0.0 {
            let sat_amount = self.saturation_pct / 100.0;
            l = Self::cubic_saturate(l, sat_amount);
            r = Self::cubic_saturate(r, sat_amount);
        }

        // Mono collapse under 120 Hz
        if self.mono_collapse {
            let mid = (l + r) * 0.5;
            let side = (l - r) * 0.5;
            // Highpass side to retain spatial width in mid/high while keeping bass mono
            let filtered_side = self.mono_filter.process_sample(0, side);
            l = mid + filtered_side;
            r = mid - filtered_side;
        }

        (l, r)
    }

    #[inline(always)]
    fn cubic_saturate(x: f32, amount: f32) -> f32 {
        let drive = 1.0 + amount * 1.5;
        let x_driven = (x * drive).clamp(-1.5, 1.5);
        let sat = if x_driven > 1.0 {
            2.0 / 3.0
        } else if x_driven < -1.0 {
            -2.0 / 3.0
        } else {
            x_driven - (x_driven * x_driven * x_driven) / 3.0
        };
        // Blend wet/dry saturation
        x * (1.0 - amount) + sat * amount
    }
}

// ============================================================================
// 2. Treble Sparkle & Dynamic Exciter
// ============================================================================

#[derive(Debug, Clone)]
pub struct TrebleSparkle {
    pub enabled: bool,
    pub corner_freq: f32,       // 8000, 10000, or 12000 Hz
    pub brilliance_db: f32,     // 0 to +12 dB
    pub transient_attack_db: f32, // 0 to +6 dB
    pub tube_warmth_pct: f32,   // 0% to 100%

    shelf_filter: Biquad,
    transient_hpf: Biquad,
    sample_rate: f32,
    envelope: [f32; 2],
}

impl TrebleSparkle {
    pub fn new(sample_rate: f32) -> Self {
        let sr = if sample_rate > 0.0 { sample_rate } else { 48000.0 };
        let shelf = Biquad::new(FilterType::HighShelf { gain_db: 4.2, q: 0.707 }, 10000.0, sr);
        let trans = Biquad::new(FilterType::HighPass { q: 1.0 }, 4000.0, sr);

        Self {
            enabled: true,
            corner_freq: 10000.0,
            brilliance_db: 4.2,
            transient_attack_db: 2.8,
            tube_warmth_pct: 68.0,
            shelf_filter: shelf,
            transient_hpf: trans,
            sample_rate: sr,
            envelope: [0.0; 2],
        }
    }

    pub fn set_params(&mut self, corner_freq: f32, brilliance_db: f32, attack_db: f32, tube_warmth: f32) {
        self.corner_freq = corner_freq.clamp(6000.0, 16000.0);
        self.brilliance_db = brilliance_db.clamp(0.0, 12.0);
        self.transient_attack_db = attack_db.clamp(0.0, 6.0);
        self.tube_warmth_pct = tube_warmth.clamp(0.0, 100.0);

        self.shelf_filter.set_params(
            FilterType::HighShelf { gain_db: self.brilliance_db, q: 0.707 },
            self.corner_freq,
            self.sample_rate,
        );
    }

    #[inline(always)]
    pub fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        if !self.enabled {
            return (left, right);
        }

        let mut l = self.shelf_filter.process_sample(0, left);
        let mut r = self.shelf_filter.process_sample(1, right);

        // Transient emphasis on high frequency bursts
        if self.transient_attack_db > 0.0 {
            let mult = 10.0_f32.powf(self.transient_attack_db / 20.0) - 1.0;
            for (ch, sample) in [l, r].iter().enumerate() {
                let hp = self.transient_hpf.process_sample(ch, *sample);
                let env_target = hp.abs();
                // Fast attack, medium release envelope follower
                let alpha = if env_target > self.envelope[ch] { 0.8 } else { 0.05 };
                self.envelope[ch] += alpha * (env_target - self.envelope[ch]);

                let transient_boost = hp * self.envelope[ch] * mult * 0.5;
                if ch == 0 {
                    l += transient_boost;
                } else {
                    r += transient_boost;
                }
            }
        }

        // Triode tube soft even-harmonic saturation: f(x) = x + 0.15*x^2 - 0.05*x^3
        if self.tube_warmth_pct > 0.0 {
            let warmth = self.tube_warmth_pct / 100.0 * 0.25;
            l = l + warmth * (l * l.abs() - l * l * l * 0.33);
            r = r + warmth * (r * r.abs() - r * r * r * 0.33);
        }

        (l, r)
    }
}

// ============================================================================
// 3. Real Reverb & Spatial Engine (Algorithmic Schroeder / Freeverb)
// ============================================================================

const NUM_COMBS: usize = 8;
const NUM_ALLPASSES: usize = 4;

// Classic Freeverb tuning lengths (at 44.1 kHz, scaled for 48 kHz)
const COMB_TUNINGS_44K: [usize; NUM_COMBS] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_TUNINGS_44K: [usize; NUM_ALLPASSES] = [556, 441, 341, 225];
const STEREO_SPREAD: usize = 23;

#[derive(Debug, Clone)]
struct CombFilter {
    buffer: Vec<f32>,
    size: usize,
    index: usize,
    filter_store: f32,
    damp: f32,
    feedback: f32,
}

impl CombFilter {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size],
            size,
            index: 0,
            filter_store: 0.0,
            damp: 0.2,
            feedback: 0.8,
        }
    }

    #[inline(always)]
    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.index];
        // Lowpass feedback filter for HF damping
        self.filter_store = output * (1.0 - self.damp) + self.filter_store * self.damp;
        self.buffer[self.index] = input + self.filter_store * self.feedback;
        self.index = (self.index + 1) % self.size;
        output
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.filter_store = 0.0;
        self.index = 0;
    }
}

#[derive(Debug, Clone)]
struct AllPassFilter {
    buffer: Vec<f32>,
    size: usize,
    index: usize,
    feedback: f32,
}

impl AllPassFilter {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size],
            size,
            index: 0,
            feedback: 0.5,
        }
    }

    #[inline(always)]
    fn process(&mut self, input: f32) -> f32 {
        let buf_out = self.buffer[self.index];
        let output = -input + buf_out;
        self.buffer[self.index] = input + buf_out * self.feedback;
        self.index = (self.index + 1) % self.size;
        output
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.index = 0;
    }
}

#[derive(Debug, Clone)]
pub struct RealReverb {
    pub enabled: bool,
    pub wet: f32,               // 0.0 to 1.0 (e.g. 0.35)
    pub dry: f32,               // 0.0 to 1.0 (e.g. 0.65)
    pub room_size: f32,         // 0.0 to 1.0 (maps to feedback 0.7 to 0.98)
    pub damping: f32,           // 0.0 to 1.0 (HF absorption)
    pub decay_secs: f32,        // RT60 in seconds (e.g. 2.4s)
    pub pre_delay_ms: f32,      // 0 to 120 ms
    pub width: f32,             // 0.0 (mono) to 2.0 (super-wide)

    // Parallel Comb Filters for Left and Right
    combs_l: Vec<CombFilter>,
    combs_r: Vec<CombFilter>,

    // Series All-Pass Filters for Left and Right
    allpasses_l: Vec<AllPassFilter>,
    allpasses_r: Vec<AllPassFilter>,

    // Pre-delay buffer
    predelay_buf_l: Vec<f32>,
    predelay_buf_r: Vec<f32>,
    predelay_idx: usize,
    predelay_len: usize,
}

impl RealReverb {
    pub fn new(sample_rate: f32) -> Self {
        let sr = if sample_rate > 0.0 { sample_rate } else { 48000.0 };
        let scale = sr / 44100.0;

        let mut combs_l = Vec::with_capacity(NUM_COMBS);
        let mut combs_r = Vec::with_capacity(NUM_COMBS);
        for &t in &COMB_TUNINGS_44K {
            let size_l = (t as f32 * scale).round() as usize;
            let size_r = ((t + STEREO_SPREAD) as f32 * scale).round() as usize;
            combs_l.push(CombFilter::new(size_l));
            combs_r.push(CombFilter::new(size_r));
        }

        let mut allpasses_l = Vec::with_capacity(NUM_ALLPASSES);
        let mut allpasses_r = Vec::with_capacity(NUM_ALLPASSES);
        for &t in &ALLPASS_TUNINGS_44K {
            let size_l = (t as f32 * scale).round() as usize;
            let size_r = ((t + STEREO_SPREAD) as f32 * scale).round() as usize;
            allpasses_l.push(AllPassFilter::new(size_l));
            allpasses_r.push(AllPassFilter::new(size_r));
        }

        let max_predelay = (sr * 0.15) as usize; // 150 ms max
        let predelay_buf_l = vec![0.0; max_predelay];
        let predelay_buf_r = vec![0.0; max_predelay];

        let mut rev = Self {
            enabled: true,
            wet: 0.35,
            dry: 0.65,
            room_size: 0.75,
            damping: 0.25,
            decay_secs: 2.4,
            pre_delay_ms: 18.0,
            width: 1.42,
            combs_l,
            combs_r,
            allpasses_l,
            allpasses_r,
            predelay_buf_l,
            predelay_buf_r,
            predelay_idx: 0,
            predelay_len: (sr * 0.018) as usize,
        };
        rev.update_params();
        rev
    }

    pub fn set_params(&mut self, wet: f32, room_size: f32, decay_secs: f32, damping: f32, pre_delay_ms: f32, width: f32) {
        self.wet = wet.clamp(0.0, 1.0);
        self.dry = (1.0 - self.wet).max(0.0);
        self.room_size = room_size.clamp(0.0, 1.0);
        self.decay_secs = decay_secs.clamp(0.2, 10.0);
        self.damping = damping.clamp(0.0, 1.0);
        self.pre_delay_ms = pre_delay_ms.clamp(0.0, 120.0);
        self.width = width.clamp(0.0, 2.0);

        self.update_params();
    }

    fn update_params(&mut self) {
        // Map room_size and decay_secs to feedback (0.7 to 0.97)
        let decay_factor = (self.decay_secs / 5.0).clamp(0.1, 1.2);
        let fb = (0.7 + 0.26 * self.room_size * decay_factor).clamp(0.5, 0.97);
        let damp = self.damping * 0.4;

        for comb in &mut self.combs_l {
            comb.feedback = fb;
            comb.damp = damp;
        }
        for comb in &mut self.combs_r {
            comb.feedback = fb;
            comb.damp = damp;
        }

        let max_len = self.predelay_buf_l.len();
        self.predelay_len = ((self.pre_delay_ms / 1000.0) * 48000.0).clamp(0.0, (max_len - 1) as f32) as usize;
    }

    pub fn reset(&mut self) {
        for c in &mut self.combs_l { c.reset(); }
        for c in &mut self.combs_r { c.reset(); }
        for a in &mut self.allpasses_l { a.reset(); }
        for a in &mut self.allpasses_r { a.reset(); }
        self.predelay_buf_l.fill(0.0);
        self.predelay_buf_r.fill(0.0);
        self.predelay_idx = 0;
    }

    #[inline(always)]
    pub fn process_stereo(&mut self, left: f32, right: f32) -> (f32, f32) {
        if !self.enabled || self.wet <= 0.001 {
            return (left, right);
        }

        // Write to pre-delay buffer
        let max_buf = self.predelay_buf_l.len();
        self.predelay_buf_l[self.predelay_idx] = left;
        self.predelay_buf_r[self.predelay_idx] = right;

        // Read from delayed position
        let read_idx = if self.predelay_idx >= self.predelay_len {
            self.predelay_idx - self.predelay_len
        } else {
            max_buf + self.predelay_idx - self.predelay_len
        };
        self.predelay_idx = (self.predelay_idx + 1) % max_buf;

        let del_l = self.predelay_buf_l[read_idx];
        let del_r = self.predelay_buf_r[read_idx];

        let mono_in = (del_l + del_r) * 0.5 * 0.015; // Scaled input to prevent comb clipping

        // Parallel Comb filters accumulation
        let mut out_l = 0.0;
        let mut out_r = 0.0;
        for c in &mut self.combs_l {
            out_l += c.process(mono_in);
        }
        for c in &mut self.combs_r {
            out_r += c.process(mono_in);
        }

        // Series All-Pass diffusion
        for a in &mut self.allpasses_l {
            out_l = a.process(out_l);
        }
        for a in &mut self.allpasses_r {
            out_r = a.process(out_r);
        }

        // Stereo width control
        let wet1 = self.wet * (self.width * 0.5 + 0.5);
        let wet2 = self.wet * ((1.0 - self.width) * 0.5);

        let final_l = left * self.dry + out_l * wet1 + out_r * wet2;
        let final_r = right * self.dry + out_r * wet1 + out_l * wet2;

        (final_l, final_r)
    }
}

// ============================================================================
// 4. Brickwall Peak Safety Limiter
// ============================================================================

#[derive(Debug, Clone)]
pub struct BrickwallLimiter {
    pub enabled: bool,
    pub ceiling_dbtp: f32, // -0.1 dBTP -> linear ~ 0.9885
    ceiling_linear: f32,
}

impl BrickwallLimiter {
    pub fn new() -> Self {
        let ceiling_dbtp = -0.1;
        let ceiling_linear = 10.0_f32.powf(ceiling_dbtp / 20.0);
        Self {
            enabled: true,
            ceiling_dbtp,
            ceiling_linear,
        }
    }

    #[inline(always)]
    pub fn process_stereo(&self, left: f32, right: f32) -> (f32, f32) {
        if !self.enabled {
            return (left, right);
        }

        let l = Self::soft_limit(left, self.ceiling_linear);
        let r = Self::soft_limit(right, self.ceiling_linear);
        (l, r)
    }

    #[inline(always)]
    fn soft_limit(x: f32, ceiling: f32) -> f32 {
        let abs_x = x.abs();
        if abs_x <= ceiling * 0.9 {
            x
        } else {
            // Smooth knee compression into ceiling
            let sign = if x >= 0.0 { 1.0 } else { -1.0 };
            let over = abs_x - ceiling * 0.9;
            let compressed = ceiling * 0.9 + (ceiling * 0.1) * (over / (ceiling * 0.1 + over));
            sign * compressed.min(ceiling)
        }
    }
}
