//! Werbel — synteza 3-warstwowa (brzmi jak werbel, nie jak bongo):
//!
//! 1. **Body (membrana)** — trójkąt z szybkim opadnięciem pitchu (klasyczne
//!    „thump"), krótki zanik. To odpowiednik pojedynczego modu pętli, ale tyle
//!    samo trwa, ile trwa uderzenie — nie dzwoni.
//! 2. **Crack (pałeczka)** — szum wysoko-przepustowy, bardzo krótki „pstryk".
//! 3. **Sizzle (sprężyny)** — szum pasmowo-przepustowy przepuszczony przez
//!    krótki comb → metaliczny poszum drutów, dłuższy zanik.
//!
//! Feedback (z wideo) zostaje w warstwie sprężyn (comb) oraz w ugięciu
//! membrany: obwiednia pitchu + sprzężenie głośności pętli.

use std::f32::consts::TAU;

// ---------------------------------------------------------------------------
// Generator szumu (xorshift32) — deterministyczny, bez alokacji.
// ---------------------------------------------------------------------------
pub struct WhiteNoise {
    state: u32,
}

impl WhiteNoise {
    pub fn new() -> Self {
        Self { state: 0x9e37_79b9 }
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        (self.state >> 8) as f32 * (1.0 / 16_777_216.0)
    }
}

// ---------------------------------------------------------------------------
// Głos werbla
// ---------------------------------------------------------------------------
pub struct SnareVoice {
    // --- membrana (body) ---
    body_phase: f32,
    body_f_end: f32,      // docelowa częstotliwość (Tone)
    body_f_span: f32,     // o ile wyżej startuje pitch (Sweep)
    body_env: f32,        // obwiednia (1 -> 0)
    body_pitch_env: f32,  // ugięcie naciągu (1 -> 0)
    body_decay: f32,      // per-sample zanik obwiedni
    pitch_decay: f32,     // per-sample zanik ugięcia

    // --- crack (pałeczka) ---
    crack_env: f32,
    crack_decay: f32,
    crack_hp: f32,        // stan jednobiegunowego high-pass
    crack_k: f32,

    // --- sizzle (sprężyny) ---
    sizzle_env: f32,
    sizzle_decay: f32,
    bp_lp1: f32,          // bandpass: lowpass(górny)
    bp_lp2: f32,          // bandpass: lowpass(dolny)
    bp_k1: f32,
    bp_k2: f32,
    comb: Vec<f32>,       // grzebień sprężyn (rezonans drutów)
    comb_pos: usize,
    comb_delay: usize,    // odczyt = pos + len - delay
    comb_lp: f32,
    comb_damp: f32,

    noise: WhiteNoise,
    level: f32,
    sample_rate: f32,
    active: bool,
    dead_count: u32,
}

impl SnareVoice {
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        Self {
            body_phase: 0.0,
            body_f_end: 200.0,
            body_f_span: 40.0,
            body_env: 0.0,
            body_pitch_env: 0.0,
            body_decay: 1.0,
            pitch_decay: 1.0,
            crack_env: 0.0,
            crack_decay: 1.0,
            crack_hp: 0.0,
            crack_k: 1.0 - (-TAU * 1500.0 / sr).exp(),
            sizzle_env: 0.0,
            sizzle_decay: 1.0,
            bp_lp1: 0.0,
            bp_lp2: 0.0,
            bp_k1: 0.5,
            bp_k2: 0.3,
            comb: vec![0.0; (sr * 0.005) as usize + 1],
            comb_pos: 0,
            comb_delay: 24,
            comb_lp: 0.0,
            comb_damp: 1.0 - (-TAU * 6500.0 / sr).exp(),
            noise: WhiteNoise::new(),
            level: 0.8,
            sample_rate: sr,
            active: false,
            dead_count: 0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.max(1.0);
        self.crack_k = 1.0 - (-TAU * 1500.0 / self.sample_rate).exp();
        self.comb_damp = 1.0 - (-TAU * 6500.0 / self.sample_rate).exp();
    }

    pub fn set_level(&mut self, level: f32) {
        self.level = level;
    }

    pub fn trigger(
        &mut self,
        tone: f32,
        ring: f32,
        damping: f32,
        snare: f32,
        snap: f32,
        sweep: f32,
        vel: f32,
    ) {
        let sr = self.sample_rate;
        let r = ring.clamp(0.2, 1.0);

        // --- membrana ---
        self.body_f_end = tone.clamp(60.0, 900.0);
        self.body_f_span = self.body_f_end * 0.5 * sweep.clamp(0.0, 1.0);
        self.body_phase = 0.0;
        self.body_pitch_env = 1.0;
        self.pitch_decay = (-1.0 / (0.045 * sr)).exp();
        self.body_env = 1.0;
        self.body_decay = (-1.0 / (0.085 * (0.4 + r) * sr)).exp();

        // --- crack ---
        self.crack_env = (snap.clamp(0.0, 1.0) * (0.5 + 0.5 * vel)).clamp(0.0, 1.0) * 1.5;
        self.crack_decay = (-1.0 / (0.020 * sr)).exp();
        self.crack_hp = 0.0;

        // --- sizzle: druty pobudzane RUCHEM membrany ---
        self.sizzle_env = snare.clamp(0.0, 1.0) * 0.9;
        self.sizzle_decay = (-1.0 / (0.16 * (0.5 + 0.8 * r) * sr)).exp();
        let fhi = damping.clamp(2000.0, 10_000.0);
        let flo = (fhi * 0.72).clamp(300.0, 5000.0);   // węższe pasmo
        self.bp_k1 = 1.0 - (-TAU * fhi / sr).exp();
        self.bp_k2 = 1.0 - (-TAU * flo / sr).exp();
        self.bp_lp1 = 0.0;
        self.bp_lp2 = 0.0;
        self.comb.fill(0.0);
        self.comb_pos = 0;
        self.comb_lp = 0.0;
        // rezonans drutów podąża za strojem membrany (~tone x 10)
        self.comb_delay =
            (sr / (tone.clamp(60.0, 900.0) * 10.0)).clamp(8.0, 200.0) as usize;

        self.active = true;
        self.dead_count = 0;
    }

    #[inline]
    pub fn process(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let n = self.noise.next() * 2.0 - 1.0;

        // --- membrana: trójkąt z glide'em ---
        self.body_pitch_env *= self.pitch_decay;
        let f = self.body_f_end + self.body_f_span * self.body_pitch_env;
        self.body_phase += f / self.sample_rate;
        if self.body_phase >= 1.0 {
            self.body_phase -= 1.0;
        }
        let tri = 4.0 * self.body_phase.min(1.0 - self.body_phase) - 1.0;
        self.body_env *= self.body_decay;
        let body = tri * self.body_env * 0.5;

        // --- crack: szum high-pass ---
        self.crack_hp += self.crack_k * (n - self.crack_hp);
        let crack_noise = n - self.crack_hp;
        self.crack_env *= self.crack_decay;
        let crack = crack_noise * self.crack_env * 0.9;

        // --- sizzle: druty „bzzzt" — szum modulowany wychyleniem membrany ---
        self.bp_lp1 += self.bp_k1 * (n - self.bp_lp1);
        self.bp_lp2 += self.bp_k2 * (n - self.bp_lp2);
        let bp = self.bp_lp1 - self.bp_lp2;

        // Ruch membrany bramkuje szum drutów — druty grzęzną w drganiach
        // głowy (to daje charakterystyczny „bzzzt", nie czysty szum).
        let head_sig = tri * self.body_env;
        let wire_exc = bp * (head_sig.abs() * 3.0 + 0.01);

        let clen = self.comb.len();
        let read = (self.comb_pos + clen - self.comb_delay) % clen;
        let comb_out = self.comb[read];
        self.comb_lp += self.comb_damp * (comb_out - self.comb_lp);
        self.comb[self.comb_pos] = wire_exc + 0.6 * self.comb_lp;
        self.comb_pos += 1;
        if self.comb_pos >= clen {
            self.comb_pos = 0;
        }
        self.sizzle_env *= self.sizzle_decay;
        let sizzle = (comb_out * 0.75 + bp * 0.03) * self.sizzle_env;

        let mut out = body + crack + sizzle;
        out *= self.level;

        // dezaktywacja po wygaśnięciu
        if out.abs() < 1e-4 {
            self.dead_count += 1;
            if self.dead_count > 480 {
                self.active = false;
                return 0.0;
            }
        } else {
            self.dead_count = 0;
        }

        (out * 1.5).tanh()
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}