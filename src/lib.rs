//! Werbel — VST3/CLAP syntezator werbla (feedback synthesis).
//!
//! Autor: gwolaniuk
//! Metoda dźwięku wzorowana na filmie „Synthesize SNARES that sound REAL using
//! the power of FEEDBACK" (youtube.com/watch?v=1Db9rGbth_o).

use nih_plug::prelude::*;
use std::sync::Arc;

mod dsp;

struct Werbel {
    params: Arc<WerbelParams>,
    voice: dsp::SnareVoice,
    sample_rate: f32,
}

#[derive(Params)]
struct WerbelParams {
    /// Częstotliwość rezonansu membrany (długość pętli sprzężenia).
    #[id = "tone"]
    tone: FloatParam,

    /// Sprzężenie zwrotne — długość wybrzmienia.
    #[id = "ring"]
    ring: FloatParam,

    /// Jasność membrany (lowpass w pętli).
    #[id = "damping"]
    damping: FloatParam,

    /// Poziom sprężyn (jaśniejsza pętla rezonansowa).
    #[id = "snare"]
    snare: FloatParam,

    /// Poziom transjenta — „trzask"/wzbudzenie.
    #[id = "snap"]
    snap: FloatParam,

    /// Głośność wyjściowa.
    #[id = "level"]
    level: FloatParam,
}

impl Default for Werbel {
    fn default() -> Self {
        Self {
            params: Arc::new(WerbelParams::default()),
            voice: dsp::SnareVoice::new(44100.0),
            sample_rate: 44100.0,
        }
    }
}

impl Default for WerbelParams {
    fn default() -> Self {
        Self {
            tone: FloatParam::new(
                "Tone",
                200.0,
                FloatRange::Linear { min: 80.0, max: 600.0 },
            )
            .with_smoother(SmoothingStyle::Logarithmic(10.0))
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            ring: FloatParam::new("Ring", 0.90, FloatRange::Linear { min: 0.5, max: 0.998 })
                .with_smoother(SmoothingStyle::Linear(10.0))
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),

            damping: FloatParam::new("Damping", 3500.0, FloatRange::Linear { min: 300.0, max: 14000.0 })
                .with_smoother(SmoothingStyle::Logarithmic(10.0))
                .with_unit(" Hz")
                .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),

            snare: FloatParam::new("Snare", 0.6, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(10.0))
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),

            snap: FloatParam::new("Snap", 0.7, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(5.0))
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),

            level: FloatParam::new("Level", 0.8, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(5.0))
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
        }
    }
}

impl Plugin for Werbel {
    const NAME: &'static str = "Werbel";
    const VENDOR: &'static str = "gwolaniuk";
    const URL: &'static str = "https://github.com/gwolaniuk/werbel";
    const EMAIL: &'static str = "gwolaniuk@users.noreply.github.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _bus_config: &BusConfig,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.voice = dsp::SnareVoice::new(self.sample_rate);
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext,
    ) -> ProcessStatus {
        // Wyłapujemy note-on (wyzwolenie werbla).
        let mut trigger = None;
        while let Some(event) = context.next_event() {
            if let NoteEvent::NoteOn { velocity, .. } = event {
                trigger = Some(velocity);
            }
        }

        for mut samples in buffer.iter_samples() {
            let tone = self.params.tone.smoothed.next();
            let ring = self.params.ring.smoothed.next();
            let damping = self.params.damping.smoothed.next();
            let snare = self.params.snare.smoothed.next();
            let snap = self.params.snap.smoothed.next();
            let level = self.params.level.smoothed.next();
            self.voice.set_level(level);

            if let Some(vel) = trigger.take() {
                self.voice.trigger(tone, ring, damping, snare, snap, vel);
            }

            let s = self.voice.process();
            samples[0] = s;
            samples[1] = s;
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Werbel {
    const CLAP_ID: &'static str = "com.gwolaniuk.werbel";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Snare drum synthesizer (feedback synthesis of the drum head)");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] =
        &[ClapFeature::Instrument, ClapFeature::Synthesizer];
}

impl Vst3Plugin for Werbel {
    const VST3_CLASS_ID: [u8; 16] = *b"WrblFeedGwln0001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_plug::nih_export_clap!(Werbel);
nih_plug::nih_export_vst3!(Werbel);