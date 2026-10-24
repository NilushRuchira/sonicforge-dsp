//! Biquad Filter Implementation (Direct Form II Transposed)
//! 
//! Numerically stable 2nd-order IIR filter with denormal protection,
//! NaN/Inf sanitization, and coefficient stability verification.

use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterType {
    Peaking { gain_db: f32, q: f32 },
    LowShelf { gain_db: f32, q: f32 },
    HighShelf { gain_db: f32, q: f32 },
    HighPass { q: f32 },
    LowPass { q: f32 },
    AllPass { q: f32 },
}

#[derive(Debug, Clone)]
pub struct Biquad {
    // Coefficients
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,

    // State variables (Direct Form II Transposed: 2 delay elements per channel)
    // Supports stereo (left = [0], right = [1])
    s1: [f32; 2],
    s2: [f32; 2],

    // Target configuration
    filter_type: FilterType,
    freq: f32,
    sample_rate: f32,
}

impl Biquad {
    pub fn new(filter_type: FilterType, freq: f32, sample_rate: f32) -> Self {
        let mut biquad = Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            s1: [0.0; 2],
            s2: [0.0; 2],
            filter_type,
            freq,
            sample_rate: if sample_rate > 0.0 { sample_rate } else { 48000.0 },
        };
        biquad.recalculate();
        biquad
    }

    pub fn set_params(&mut self, filter_type: FilterType, freq: f32, sample_rate: f32) {
        self.filter_type = filter_type;
        self.freq = freq;
        if sample_rate > 0.0 {
            self.sample_rate = sample_rate;
        }
        self.recalculate();
    }

    pub fn set_gain(&mut self, gain_db: f32) {
        match self.filter_type {
            FilterType::Peaking { gain_db: ref mut g, .. } => *g = gain_db,
            FilterType::LowShelf { gain_db: ref mut g, .. } => *g = gain_db,
            FilterType::HighShelf { gain_db: ref mut g, .. } => *g = gain_db,
            _ => return,
        }
        self.recalculate();
    }

    pub fn reset_state(&mut self) {
        self.s1 = [0.0; 2];
        self.s2 = [0.0; 2];
    }

    pub fn recalculate(&mut self) {
        let sr = self.sample_rate;
        let nyquist = sr * 0.5;
        // Clamp frequency between 10 Hz and 0.49 * sample_rate to prevent alias explosion
        let f0 = self.freq.clamp(10.0, nyquist * 0.98);
        let omega = 2.0 * PI * f0 / sr;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();

        let (b0, b1, b2, a0, a1, a2) = match self.filter_type {
            FilterType::Peaking { gain_db, q } => {
                let q_safe = q.max(0.01);
                let a = 10.0_f32.powf(gain_db / 40.0);
                let alpha = sin_omega / (2.0 * q_safe);

                let b0 = 1.0 + alpha * a;
                let b1 = -2.0 * cos_omega;
                let b2 = 1.0 - alpha * a;
                let a0 = 1.0 + alpha / a;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha / a;

                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowShelf { gain_db, q } => {
                let a = 10.0_f32.powf(gain_db / 40.0);
                let q_safe = q.max(0.01);
                let alpha = sin_omega / (2.0 * q_safe);
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;

                let b0 = a * ((a + 1.0) - (a - 1.0) * cos_omega + two_sqrt_a_alpha);
                let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_omega);
                let b2 = a * ((a + 1.0) - (a - 1.0) * cos_omega - two_sqrt_a_alpha);
                let a0 = (a + 1.0) + (a - 1.0) * cos_omega + two_sqrt_a_alpha;
                let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_omega);
                let a2 = (a + 1.0) + (a - 1.0) * cos_omega - two_sqrt_a_alpha;

                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighShelf { gain_db, q } => {
                let a = 10.0_f32.powf(gain_db / 40.0);
                let q_safe = q.max(0.01);
                let alpha = sin_omega / (2.0 * q_safe);
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;

                let b0 = a * ((a + 1.0) + (a - 1.0) * cos_omega + two_sqrt_a_alpha);
                let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_omega);
                let b2 = a * ((a + 1.0) + (a - 1.0) * cos_omega - two_sqrt_a_alpha);
                let a0 = (a + 1.0) - (a - 1.0) * cos_omega + two_sqrt_a_alpha;
                let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_omega);
                let a2 = (a + 1.0) - (a - 1.0) * cos_omega - two_sqrt_a_alpha;

                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighPass { q } => {
                let q_safe = q.max(0.01);
                let alpha = sin_omega / (2.0 * q_safe);

                let b0 = (1.0 + cos_omega) / 2.0;
                let b1 = -(1.0 + cos_omega);
                let b2 = (1.0 + cos_omega) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;

                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowPass { q } => {
                let q_safe = q.max(0.01);
                let alpha = sin_omega / (2.0 * q_safe);

                let b0 = (1.0 - cos_omega) / 2.0;
                let b1 = 1.0 - cos_omega;
                let b2 = (1.0 - cos_omega) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;

                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::AllPass { q } => {
                let q_safe = q.max(0.01);
                let alpha = sin_omega / (2.0 * q_safe);

                let b0 = 1.0 - alpha;
                let b1 = -2.0 * cos_omega;
                let b2 = 1.0 + alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;

                (b0, b1, b2, a0, a1, a2)
            }
        };

        let inv_a0 = if a0.abs() > 1e-12 { 1.0 / a0 } else { 1.0 };

        let norm_b0 = b0 * inv_a0;
        let norm_b1 = b1 * inv_a0;
        let norm_b2 = b2 * inv_a0;
        let norm_a1 = a1 * inv_a0;
        let norm_a2 = a2 * inv_a0;

        // Stability check: For Direct Form II, poles must lie inside unit circle:
        // 1) |a2| < 1
        // 2) |a1| < 1 + a2
        if norm_a2.abs() < 1.0 && norm_a1.abs() < (1.0 + norm_a2) && norm_b0.is_finite() {
            self.b0 = norm_b0;
            self.b1 = norm_b1;
            self.b2 = norm_b2;
            self.a1 = norm_a1;
            self.a2 = norm_a2;
        } else {
            // Passthrough if unstable
            self.b0 = 1.0;
            self.b1 = 0.0;
            self.b2 = 0.0;
            self.a1 = 0.0;
            self.a2 = 0.0;
        }
    }

    /// Process a single sample on given channel (0 = Left, 1 = Right)
    /// Direct Form II Transposed structure:
    /// y[n] = b0 * x[n] + s1[n-1]
    /// s1[n] = b1 * x[n] - a1 * y[n] + s2[n-1]
    /// s2[n] = b2 * x[n] - a2 * y[n]
    #[inline(always)]
    pub fn process_sample(&mut self, ch: usize, input: f32) -> f32 {
        let ch = ch & 1;
        // Sanitize input against NaNs, infinities, or catastrophic over-range spikes
        let x = if input.is_finite() { input.clamp(-8.0, 8.0) } else { 0.0 };

        let y = self.b0 * x + self.s1[ch];
        
        let new_s1 = self.b1 * x - self.a1 * y + self.s2[ch];
        let new_s2 = self.b2 * x - self.a2 * y;

        // Flush denormals / subnormals to zero and clamp internal state to prevent windup
        self.s1[ch] = if new_s1.abs() > 1e-15 && new_s1.is_finite() { new_s1.clamp(-32.0, 32.0) } else { 0.0 };
        self.s2[ch] = if new_s2.abs() > 1e-15 && new_s2.is_finite() { new_s2.clamp(-32.0, 32.0) } else { 0.0 };

        if y.is_finite() { y } else { 0.0 }
    }

    /// Check mathematical stability: returns true if poles lie inside the unit circle.
    pub fn is_stable(&self) -> bool {
        self.a2.abs() < 1.0 && self.a1.abs() < (1.0 + self.a2)
    }
}
