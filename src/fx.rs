//! Efekty końcowe: kompresor „glue" + reverb Freeverb (8 grzebieni + 4 allpassy).
use std::f32::consts::TAU;

/// Miękki kompresor (soft-knee) z envelope-followerem. `mix` (0..1) domiesza
/// skompresowany sygnał do suchego.
pub struct Glue {
    follower: f32,
}

impl Glue {
    pub fn new() -> Self {
        Self { follower: 0.0 }
    }

    #[inline]
    pub fn process(&mut self, x: f32, mix: f32, sample_rate: f32) -> f32 {
        let abs = x.abs();
        let coeff = if abs > self.follower {
            1.0 - (-1.0 / (0.002 * sample_rate)).exp()
        } else {
            1.0 - (-1.0 / (0.150 * sample_rate)).exp()
        };
        self.follower += coeff * (abs - self.follower);

        let db = 20.0 * self.follower.max(1e-6).log10();
        let thr = -20.0;
        let knee = 12.0;
        let ratio = 4.0;
        let gr = if db < thr - knee * 0.5 {
            0.0
        } else if db > thr + knee * 0.5 {
            (db - thr) * (1.0 - 1.0 / ratio)
        } else {
            let t = db - thr + knee * 0.5;
            (1.0 - 1.0 / ratio) * t * t / (2.0 * knee)
        };

        let gain = 10f32.powf(-gr / 20.0) * 1.78;
        x * (1.0 - mix) + (x * gain) * mix
    }
}

// ---------------------------------------------------------------------------
// Freeverb (Schroeder/Moorer): 8 równoległych grzebieni + 4 allpassy szeregowo.
// Gładki, naturalny ogon — bez metalicznego „piiuu".
// ---------------------------------------------------------------------------
// Opóźnienia grzebieni (s) i allpassów (s) — wzorowane na klasycznym Freeverb.
const COMB_DELAYS: [f32; 8] = [0.0253, 0.0269, 0.0290, 0.0307, 0.0322, 0.0338, 0.0353, 0.0367];
const AP_DELAYS: [f32; 4] = [0.0126, 0.0100, 0.0077, 0.0051];
const STEREO_SPREAD: f32 = 0.00052;

struct Comb {
    buf: Vec<f32>,
    pos: usize,
    fb: f32,
    damp: f32,
    lp: f32,
}

impl Comb {
    fn new(length: usize) -> Self {
        Self { buf: vec![0.0; length], pos: 0, fb: 0.0, damp: 0.0, lp: 0.0 }
    }

    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let len = self.buf.len();
        let out = self.buf[self.pos];
        self.lp += self.damp * (out - self.lp);
        self.buf[self.pos] = x + self.fb * self.lp;
        self.pos += 1;
        if self.pos >= len {
            self.pos = 0;
        }
        out
    }
}

struct Allpass {
    buf: Vec<f32>,
    pos: usize,
    g: f32,
}

impl Allpass {
    fn new(length: usize) -> Self {
        Self { buf: vec![0.0; length], pos: 0, g: 0.5 }
    }

    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let len = self.buf.len();
        let bufout = self.buf[self.pos];
        let y = bufout - x * self.g;
        self.buf[self.pos] = x + bufout * self.g;
        self.pos += 1;
        if self.pos >= len {
            self.pos = 0;
        }
        y
    }
}

pub struct Reverb {
    comb_l: [Comb; 8],
    comb_r: [Comb; 8],
    ap_l: [Allpass; 4],
    ap_r: [Allpass; 4],
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        let mut r = Self {
            comb_l: std::array::from_fn(|_| Comb::new(1)),
            comb_r: std::array::from_fn(|_| Comb::new(1)),
            ap_l: std::array::from_fn(|_| Allpass::new(1)),
            ap_r: std::array::from_fn(|_| Allpass::new(1)),
        };
        r.set_sample_rate(sample_rate);
        r
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        let sr = sr.max(1.0);
        let damp = 1.0 - (-TAU * 3600.0 / sr).exp();
        for i in 0..8 {
            let l = (sr * COMB_DELAYS[i]) as usize + 1;
            let rr = (sr * (COMB_DELAYS[i] + STEREO_SPREAD)) as usize + 1;
            self.comb_l[i] = Comb { buf: vec![0.0; l], pos: 0, fb: 0.82, damp, lp: 0.0 };
            self.comb_r[i] = Comb { buf: vec![0.0; rr], pos: 0, fb: 0.82, damp, lp: 0.0 };
        }
        for i in 0..4 {
            let l = (sr * AP_DELAYS[i]) as usize + 1;
            let rr = (sr * (AP_DELAYS[i] + STEREO_SPREAD)) as usize + 1;
            self.ap_l[i] = Allpass { buf: vec![0.0; l], pos: 0, g: 0.5 };
            self.ap_r[i] = Allpass { buf: vec![0.0; rr], pos: 0, g: 0.5 };
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32, wet: f32) -> (f32, f32) {
        let mut l = 0.0;
        for c in self.comb_l.iter_mut() {
            l += c.tick(x);
        }
        l *= 0.125;
        for a in self.ap_l.iter_mut() {
            l = a.tick(l);
        }

        let mut r = 0.0;
        for c in self.comb_r.iter_mut() {
            r += c.tick(x);
        }
        r *= 0.125;
        for a in self.ap_r.iter_mut() {
            r = a.tick(r);
        }

        (l * wet, r * wet)
    }
}