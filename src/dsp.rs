//! Werbel — syntezator werbla oparty o **feedback synthesis**.
//!
//! Rdzeń dźwięku (za: _„Synthesize SNARES that sound REAL using the power of
//! FEEDBACK”_, youtube.com/watch?v=1Db9rGbth_o):
//! krótki, kilkumilisekundowy **transient** (szum) wstrzyknięty w pętlę
//! sprzężenia zwrotnego (delay + dolnoprzepustowy filtr w pętli). Pętla modeluje
//! **rezonans membrany** — częstotliwość rezonansu wyznacza czas opóźnienia
//! (`f = sr / delay`), a filtr w pętli daje tłumienie zależne od częstotliwości
//! (wyższe mody gaśnię szybciej, jak w prawdziwej membranie).
//!
//! Do tego dodana jest druga, jaśniejsza pętla rezonansowa („sprężyny"/snares),
//! pobudzana tym samym transientem — daje metaliczny poszum drutów werbla.

use std::f32::consts::TAU;

// ---------------------------------------------------------------------------
// Rezonator: delay + filtr w pętli sprzężenia zwrotnego (comb-resonator).
// ---------------------------------------------------------------------------
pub struct Resonator {
    buf: Vec<f32>,
    write: usize,
    delay: f32,       // ułamkowe opóźnienie w próbkach
    feedback: f32,    // 0.0 .. <1.0 (ile wraca do pętli)
    lp: f32,          // stan jednobiegunowego filtru dolnoprzepustowego
    damp: f32,        // współczynnik tego filtru
}

impl Resonator {
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: vec![0.0; capacity],
            write: 0,
            delay: 100.0,
            feedback: 0.9,
            lp: 0.0,
            damp: 0.05,
        }
    }

    /// Ustawia częstotliwość osiową rezonansu (`f = sr / delay`).
    pub fn set_frequency(&mut self, freq: f32, sr: f32) {
        self.delay = (sr / freq).clamp(2.0, (self.buf.len() - 1) as f32);
    }

    /// Ustawia dolnoprzepustowy filtr tłumiący w pętli (Hz).
    pub fn set_damping(&mut self, hz: f32, sr: f32) {
        self.damp = 1.0 - (-TAU * hz / sr).exp();
    }

    /// Jeden krok rezonatora: `input` to wzbudzenie (transient).
    #[inline]
    pub fn tick(&mut self, input: f32) -> f32 {
        let len = self.buf.len() as f32;

        // Odczyt z interpolacją liniową (płynne strojenie).
        let mut rp = self.write as f32 - self.delay;
        while rp < 0.0 {
            rp += len;
        }
        let i0 = rp as usize;
        let frac = rp - i0 as f32;
        let i1 = if i0 + 1 >= self.buf.len() { 0 } else { i0 + 1 };
        let out = self.buf[i0] * (1.0 - frac) + self.buf[i1] * frac;

        // Filtr tłumiący w pętli (lowpass) — energia odpływa z górą pasma.
        self.lp += self.damp * (out - self.lp);

        // Zapis do pętli: wzbudzenie + sprzężenie zwrotne przefiltrowanego sygnału.
        self.buf[self.write] = input + self.feedback * self.lp;
        self.write += 1;
        if self.write >= self.buf.len() {
            self.write = 0;
        }

        out
    }

    pub fn clear(&mut self) {
        self.buf.fill(0.0);
        self.lp = 0.0;
        self.write = 0;
    }
}

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
        // 0..1
        (self.state >> 8) as f32 * (1.0 / 16_777_216.0)
    }
}

// ---------------------------------------------------------------------------
// Głos werbla: transient -> (membrana + sprężyny), naturalne wybrzmienie.
// ---------------------------------------------------------------------------
pub struct SnareVoice {
    head: Resonator,  // membrana (wysokie napięcie -> czyste mody)
    snares: Resonator,// sprężyny (jaśniejsze, wyższy rezonans)
    noise: WhiteNoise,

    // wzbudzenie (transient)
    exc_len: usize,   // długość transjenta w próbkach
    exc_pos: usize,
    snap: f32,        // poziom transjenta (0..1)

    // poziomy
    snare_amt: f32,   // 0..1 — ile sprężyn
    level: f32,       // master

    active: bool,
    sample_rate: f32,
    dead_count: u32,
}

impl SnareVoice {
    pub fn new(sample_rate: f32) -> Self {
        let mut head = Resonator::new(sample_rate as usize);
        let mut snares = Resonator::new(sample_rate as usize);
        head.set_frequency(200.0, sample_rate);
        snares.set_frequency(1600.0, sample_rate);
        head.set_damping(3500.0, sample_rate);
        snares.set_damping(12000.0, sample_rate);
        Self {
            head,
            snares,
            noise: WhiteNoise::new(),
            exc_len: 0,
            exc_pos: 0,
            snap: 0.7,
            snare_amt: 0.6,
            level: 0.8,
            active: false,
            sample_rate,
            dead_count: 0,
        }
    }

    /// Wyzwala werbel. `tone` = częstotliwość membrany, `ring` = sprzężenie
    /// zwrotne (długość wybrzmienia), `damping` = jasność pętli membrany,
    /// `snare` = poziom sprężyn, `snap` = poziom transjenta, `vel` = velocity.
    pub fn trigger(
        &mut self,
        tone: f32,
        ring: f32,
        damping: f32,
        snare: f32,
        snap: f32,
        vel: f32,
    ) {
        let sr = self.sample_rate;
        self.head.set_frequency(tone, sr);
        // Sprężyny rezonują ~8x wyżej niż membrana (metaliczny poszum).
        self.snares.set_frequency((tone * 8.0).clamp(400.0, 8000.0), sr);
        self.head.feedback = ring.clamp(0.0, 0.998);
        self.snares.feedback = (ring.clamp(0.0, 0.998) * 0.97).clamp(0.0, 0.995);
        self.head.set_damping(damping, sr);
        self.snares.set_damping(12000.0, sr);

        // Transient: 3 ms szumu — „katalizator" całego dźwięku.
        self.exc_len = (sr * 0.003) as usize;
        self.exc_pos = 0;
        self.snap = (snap * (0.5 + 0.5 * vel)).clamp(0.0, 1.0);
        self.snare_amt = snare.clamp(0.0, 1.0);
        self.active = true;
        self.dead_count = 0;
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }

    pub fn set_level(&mut self, level: f32) {
        self.level = level;
    }

    /// Renderuje jedną próbkę (mono).
    #[inline]
    pub fn process(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        // --- wzbudzenie (szumowy transient o krótkim czasie życia) ---
        let n = self.noise.next() * 2.0 - 1.0;
        let exc = if self.exc_pos < self.exc_len {
            let k = 1.0 - (self.exc_pos as f32 / self.exc_len as f32);
            self.exc_pos += 1;
            n * k * self.snap * 2.0
        } else {
            0.0
        };

        // --- obie pętle rezonansowe pobudzane tym samym transientem ---
        let head_out = self.head.tick(exc);
        let snares_out = self.snares.tick(exc);

        // Membrana + sprężyny + bezpośredni transient („trzask" pałeczki).
        let mut out = head_out + snares_out * self.snare_amt * 0.9 + exc * 0.25;
        out *= self.level;

        // Naturalne wybrzmienie: dezaktywuj, gdy sygnał wygasł.
        if out.abs() < 1e-4 {
            self.dead_count += 1;
            if self.dead_count > 480 {
                self.active = false;
                return 0.0;
            }
        } else {
            self.dead_count = 0;
        }

        // Miękkie obcięcie — gęstszy, „nagłośniony" charakter.
        (out * 1.6).tanh()
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}