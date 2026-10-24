//! 10-Band & 20-Band Graphic/Parametric Equalizer with Cascaded Biquad Filters

use super::biquad::{Biquad, FilterType};

pub const ISO_10_FREQS: [f32; 10] = [
    31.0, 63.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

pub const ISO_20_FREQS: [f32; 20] = [
    20.0, 31.5, 45.0, 63.0, 90.0, 125.0, 180.0, 250.0, 355.0, 500.0,
    710.0, 1000.0, 1400.0, 2000.0, 2800.0, 4000.0, 5600.0, 8000.0, 11200.0, 16000.0,
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EqMode {
    Bands10,
    Bands20,
}

#[derive(Debug, Clone)]
pub struct Equalizer {
    mode: EqMode,
    sample_rate: f32,
    bypassed: bool,
    solo_band: Option<usize>,

    // 10-Band filters
    filters_10: Vec<Biquad>,
    gains_10: [f32; 10],

    // 20-Band filters
    filters_20: Vec<Biquad>,
    gains_20: [f32; 20],

    // Low-Cut (High-Pass) slope filters
    low_cut_active: bool,
    low_cut_freq: f32,
    low_cut_24db: bool,
    low_cut_f1: Biquad,
    low_cut_f2: Biquad,

    // High-Cut (Low-Pass) slope filters
    high_cut_active: bool,
    high_cut_freq: f32,
    high_cut_24db: bool,
    high_cut_f1: Biquad,
    high_cut_f2: Biquad,
}

impl Equalizer {
    pub fn new(sample_rate: f32) -> Self {
        let sr = if sample_rate > 0.0 { sample_rate } else { 48000.0 };
        let q10 = 1.414;
        let q20 = 2.87;

        let mut filters_10 = Vec::with_capacity(10);
        for &f in &ISO_10_FREQS {
            filters_10.push(Biquad::new(FilterType::Peaking { gain_db: 0.0, q: q10 }, f, sr));
        }

        let mut filters_20 = Vec::with_capacity(20);
        for &f in &ISO_20_FREQS {
            filters_20.push(Biquad::new(FilterType::Peaking { gain_db: 0.0, q: q20 }, f, sr));
        }

        let low_cut_f1 = Biquad::new(FilterType::HighPass { q: 0.707 }, 24.0, sr);
        let low_cut_f2 = Biquad::new(FilterType::HighPass { q: 0.707 }, 24.0, sr);

        let high_cut_f1 = Biquad::new(FilterType::LowPass { q: 0.707 }, 18500.0, sr);
        let high_cut_f2 = Biquad::new(FilterType::LowPass { q: 0.707 }, 18500.0, sr);

        Self {
            mode: EqMode::Bands10,
            sample_rate: sr,
            bypassed: false,
            solo_band: None,
            filters_10,
            gains_10: [0.0; 10],
            filters_20,
            gains_20: [0.0; 20],
            low_cut_active: true,
            low_cut_freq: 24.0,
            low_cut_24db: true,
            low_cut_f1,
            low_cut_f2,
            high_cut_active: true,
            high_cut_freq: 18500.0,
            high_cut_24db: false,
            high_cut_f1,
            high_cut_f2,
        }
    }

    pub fn set_mode(&mut self, mode: EqMode) {
        self.mode = mode;
        self.reset_state();
    }

    pub fn mode(&self) -> EqMode {
        self.mode
    }

    pub fn set_bypassed(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    pub fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    pub fn set_solo_band(&mut self, solo: Option<usize>) {
        self.solo_band = solo;
    }

    pub fn reset_state(&mut self) {
        for f in &mut self.filters_10 {
            f.reset_state();
        }
        for f in &mut self.filters_20 {
            f.reset_state();
        }
        self.low_cut_f1.reset_state();
        self.low_cut_f2.reset_state();
        self.high_cut_f1.reset_state();
        self.high_cut_f2.reset_state();
    }

    pub fn set_band_gain(&mut self, index: usize, gain_db: f32) {
        let gain = gain_db.clamp(-12.0, 12.0);
        match self.mode {
            EqMode::Bands10 => {
                if index < self.filters_10.len() {
                    self.gains_10[index] = gain;
                    self.filters_10[index].set_gain(gain);
                }
            }
            EqMode::Bands20 => {
                if index < self.filters_20.len() {
                    self.gains_20[index] = gain;
                    self.filters_20[index].set_gain(gain);
                }
            }
        }
    }

    pub fn get_gains(&self) -> Vec<f32> {
        match self.mode {
            EqMode::Bands10 => self.gains_10.to_vec(),
            EqMode::Bands20 => self.gains_20.to_vec(),
        }
    }

    pub fn set_all_gains(&mut self, gains: &[f32]) {
        match self.mode {
            EqMode::Bands10 => {
                for (i, &g) in gains.iter().enumerate().take(10) {
                    self.set_band_gain(i, g);
                }
            }
            EqMode::Bands20 => {
                for (i, &g) in gains.iter().enumerate().take(20) {
                    self.set_band_gain(i, g);
                }
            }
        }
    }

    pub fn apply_preset(&mut self, preset_name: &str) {
        match preset_name {
            "flat" => {
                let zeros = [0.0; 20];
                self.set_all_gains(&zeros);
            }
            "bass" => {
                // Bass boost curve
                let gains10 = [6.5, 5.2, 3.1, 0.8, -0.5, -0.8, 0.0, 0.5, 0.2, 0.0];
                let gains20 = [7.0, 6.5, 5.8, 5.2, 4.2, 3.1, 1.8, 0.8, 0.0, -0.5, -0.7, -0.8, -0.4, 0.0, 0.2, 0.5, 0.4, 0.2, 0.1, 0.0];
                match self.mode {
                    EqMode::Bands10 => self.set_all_gains(&gains10),
                    EqMode::Bands20 => self.set_all_gains(&gains20),
                }
            }
            "vocal" => {
                // Vocal presence lift
                let gains10 = [-2.0, -1.0, 0.0, 0.5, 1.8, 3.4, 3.8, 2.5, 1.5, 0.8];
                let gains20 = [-2.5, -2.0, -1.5, -1.0, -0.5, 0.0, 0.2, 0.5, 1.0, 1.8, 2.5, 3.4, 3.8, 3.2, 2.5, 2.0, 1.5, 1.0, 0.8, 0.5];
                match self.mode {
                    EqMode::Bands10 => self.set_all_gains(&gains10),
                    EqMode::Bands20 => self.set_all_gains(&gains20),
                }
            }
            "loudness" => {
                // Fletcher-Munson contour
                let gains10 = [4.8, 3.5, 1.0, -1.2, -1.8, -1.5, 0.0, 1.6, 3.2, 4.5];
                let gains20 = [5.2, 4.8, 4.0, 3.5, 2.2, 1.0, -0.5, -1.2, -1.6, -1.8, -1.7, -1.5, -0.8, 0.0, 0.8, 1.6, 2.4, 3.2, 4.0, 4.5];
                match self.mode {
                    EqMode::Bands10 => self.set_all_gains(&gains10),
                    EqMode::Bands20 => self.set_all_gains(&gains20),
                }
            }
            "custom" => {
                let gains10 = [2.4, 3.8, 1.2, -1.5, -0.8, 0.0, 1.8, 3.2, 2.1, 4.0];
                match self.mode {
                    EqMode::Bands10 => self.set_all_gains(&gains10),
                    EqMode::Bands20 => {
                        let mut g20 = [0.0; 20];
                        for (i, &g) in gains10.iter().enumerate() {
                            if i * 2 < 20 {
                                g20[i * 2] = g;
                            }
                            if i * 2 + 1 < 20 {
                                g20[i * 2 + 1] = g * 0.9;
                            }
                        }
                        self.set_all_gains(&g20);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn set_low_cut(&mut self, active: bool, freq: f32, is_24db: bool) {
        self.low_cut_active = active;
        self.low_cut_freq = freq;
        self.low_cut_24db = is_24db;
        self.low_cut_f1.set_params(FilterType::HighPass { q: 0.707 }, freq, self.sample_rate);
        self.low_cut_f2.set_params(FilterType::HighPass { q: 0.707 }, freq, self.sample_rate);
    }

    pub fn set_high_cut(&mut self, active: bool, freq: f32, is_24db: bool) {
        self.high_cut_active = active;
        self.high_cut_freq = freq;
        self.high_cut_24db = is_24db;
        self.high_cut_f1.set_params(FilterType::LowPass { q: 0.707 }, freq, self.sample_rate);
        self.high_cut_f2.set_params(FilterType::LowPass { q: 0.707 }, freq, self.sample_rate);
    }

    /// Process a single stereo sample: ch=0 for Left, ch=1 for Right
    #[inline(always)]
    pub fn process_sample(&mut self, ch: usize, mut sample: f32) -> f32 {
        if self.bypassed {
            return sample;
        }

        // Low-Cut filter
        if self.low_cut_active {
            sample = self.low_cut_f1.process_sample(ch, sample);
            if self.low_cut_24db {
                sample = self.low_cut_f2.process_sample(ch, sample);
            }
        }

        // High-Cut filter
        if self.high_cut_active {
            sample = self.high_cut_f1.process_sample(ch, sample);
            if self.high_cut_24db {
                sample = self.high_cut_f2.process_sample(ch, sample);
            }
        }

        // EQ Cascaded Bands
        match self.mode {
            EqMode::Bands10 => {
                if let Some(solo_idx) = self.solo_band {
                    if solo_idx < self.filters_10.len() {
                        sample = self.filters_10[solo_idx].process_sample(ch, sample);
                    }
                } else {
                    for filter in &mut self.filters_10 {
                        sample = filter.process_sample(ch, sample);
                    }
                }
            }
            EqMode::Bands20 => {
                if let Some(solo_idx) = self.solo_band {
                    if solo_idx < self.filters_20.len() {
                        sample = self.filters_20[solo_idx].process_sample(ch, sample);
                    }
                } else {
                    for filter in &mut self.filters_20 {
                        sample = filter.process_sample(ch, sample);
                    }
                }
            }
        }

        sample
    }
}
