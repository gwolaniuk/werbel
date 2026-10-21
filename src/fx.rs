//! Efekty końcowe: kompresor „glue" + prosty stereo-reverb (2 comby).
use std::f32::consts::TAU;

/// Miękki kompresor (soft-knee) z envelope-followerem. Parametr `mix` (0..1)
/// decyduje ile skompresowanego sygnału domieszać do suchego.
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
        // detektor szczytu: atak 2 ms, release 150 ms (dwubiegunowa jedna stała).
        let coeff = if abs > self.follower {
            1.0 - (-1.0 / (0.002 * sample_rate)).exp()
        } else {
            1.0 - (-1.0 / (0.150 * sample_rate)).exp()
        };
        self.follower += coeff * (abs - self.follower);

        // Soft-knee wokół -20 dB, ratio 4:1.
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

        // makeup ~+5 dB, żeby „glue" zagęszczał, nie ściszał.
        let gain = 10f32.powf(-gr / 20.0) * 1.78;
        x * (1.0 - mix) + (x * gain) * mix
    }
}

/// Bardzo prosty, ale przyjemny reverb: dwa równoległe rezonatory grzebieniowe
/// (comb) z tłumieniem w pętli — jeden dla L, drugi dla R (różne długości dają
/// szerokość stereo).
pub struct Reverb {
    l_buf: Vec<f32>,
    l_pos: usize,
    l_lp: f32,
    l_damp: f32,
    l_fb: f32,

    r_buf: Vec<f32>,
    r_pos: usize,
    r_lp: f32,
    r_damp: f32,
    r_fb: f32,
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        let mut r = Self {
            l_buf: Vec::new(),
            l_pos: 0,
            l_lp: 0.0,
            l_damp: 0.1,
            l_fb: 0.72,
            r_buf: Vec::new(),
            r_pos: 0,
            r_lp: 0.0,
            r_damp: 0.1,
            r_fb: 0.68,
        };
        r.set_sample_rate(sample_rate);
        r
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        // Długości grzebieni ~ czas wybrzmienia małego/średniego pokoju.
        self.l_buf = vec![0.0; (sample_rate * 0.0338) as usize];
        self.r_buf = vec![0.0; (sample_rate * 0.0471) as usize];
        self.l_damp = 1.0 - (-TAU * 4200.0 / sample_rate).exp();
        self.r_damp = 1.0 - (-TAU * 3700.0 / sample_rate).exp();
        self.l_pos = 0;
        self.r_pos = 0;
        self.l_lp = 0.0;
        self.r_lp = 0.0;
    }

    #[inline]
    pub fn process(&mut self, x: f32, wet: f32) -> (f32, f32) {
        let l_len = self.l_buf.len().max(1);
        let r_len = self.r_buf.len().max(1);

        // Lewy grzebień.
        let lout = self.l_buf[self.l_pos % l_len];
        self.l_lp += self.l_damp * (lout - self.l_lp);
        self.l_buf[self.l_pos % l_len] = x * 0.5 + self.l_fb * self.l_lp;
        self.l_pos += 1;

        // Prawy grzebień.
        let rout = self.r_buf[self.r_pos % r_len];
        self.r_lp += self.r_damp * (rout - self.r_lp);
        self.r_buf[self.r_pos % r_len] = x * 0.5 + self.r_fb * self.r_lp;
        self.r_pos += 1;

        (lout * wet, rout * wet)
    }
}