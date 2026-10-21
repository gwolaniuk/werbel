//! Werbel — VST3/CLAP syntezator werbla (feedback synthesis). GUI: egui.
//!
//! Autor: gwolaniuk
//! Metoda dźwięku wzorowana na filmie „Synthesize SNARES that sound REAL using
//! the power of FEEDBACK" (youtube.com/watch?v=1Db9rGbth_o).

use nice_plug::{editor::dpi::LogicalSize, prelude::*};
use nice_plug_egui::{create_egui_editor, EguiEditor, EguiEditorState, EguiNiceSettings, RepaintNotifier};
use std::sync::Arc;

mod dsp;
mod editor;
mod fx;

struct Werbel {
    params: Arc<WerbelParams>,
    voice: dsp::SnareVoice,
    glue: fx::Glue,
    reverb: fx::Reverb,
    sample_rate: f32,

    editor_state: Arc<EguiEditorState>,
    repaint_notifier: RepaintNotifier,
    initial_editor: Option<editor::WerbelEditor>,
}

#[derive(Params)]
pub struct WerbelParams {
    /// Częstotliwość rezonansu membrany (długość pętli sprzężenia).
    #[id = "tone"]
    pub tone: FloatParam,

    /// Sprzężenie zwrotne — długość wybrzmienia korpusu.
    #[id = "ring"]
    pub ring: FloatParam,

    /// Jasność membrany (lowpass w pętli).
    #[id = "damping"]
    pub damping: FloatParam,

    /// Poziom sprężyn (jaśniejsza pętla rezonansowa).
    #[id = "snare"]
    pub snare: FloatParam,

    /// Poziom transjenta — „trzask"/wzbudzenie.
    #[id = "snap"]
    pub snap: FloatParam,

    /// Głębokość zgięcia naciągu po uderzeniu (pitch drop).
    #[id = "sweep"]
    pub sweep: FloatParam,

    /// Kompresja „glue" (0..1 domieszka skompresowanego sygnału).
    #[id = "glue"]
    pub glue: FloatParam,

    /// Reverb pokojowy (0..1 poziom mokrego).
    #[id = "verb"]
    pub verb: FloatParam,

    /// Głośność wyjściowa.
    #[id = "level"]
    pub level: FloatParam,
}

impl Default for Werbel {
    fn default() -> Self {
        let params = Arc::new(WerbelParams::default());
        let initial_editor = editor::WerbelEditor::new(params.clone());
        Self {
            params,
            voice: dsp::SnareVoice::new(48_000.0),
            glue: fx::Glue::new(),
            reverb: fx::Reverb::new(48_000.0),
            sample_rate: 48_000.0,
            editor_state: EguiEditorState::from_size(LogicalSize::new(760.0, 480.0), 1.0),
            repaint_notifier: RepaintNotifier::new(),
            initial_editor: Some(initial_editor),
        }
    }
}

impl Default for WerbelParams {
    fn default() -> Self {
        let pct = |name: &'static str, v: f32| {
            FloatParam::new(name, v, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(7.0))
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage())
        };
        Self {
            tone: FloatParam::new("Tone", 200.0, FloatRange::Linear { min: 80.0, max: 600.0 })
                .with_smoother(SmoothingStyle::Logarithmic(10.0))
                .with_unit(" Hz")
                .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),
            ring: FloatParam::new("Ring", 0.90, FloatRange::Linear { min: 0.5, max: 0.998 })
                .with_smoother(SmoothingStyle::Linear(10.0))
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
            damping: FloatParam::new(
                "Damping",
                3500.0,
                FloatRange::Linear { min: 300.0, max: 14_000.0 },
            )
            .with_smoother(SmoothingStyle::Logarithmic(10.0))
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_hz_then_khz(0)),
            snare: pct("Snare", 0.6),
            snap: pct("Snap", 0.7),
            sweep: pct("Sweep", 0.25),
            glue: pct("Glue", 0.4),
            verb: pct("Verb", 0.25),
            level: pct("Level", 0.8),
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

    type Editor = EguiEditor<editor::WerbelEditor>;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Self::Editor> {
        create_egui_editor(
            self.editor_state.clone(),
            self.repaint_notifier.clone(),
            EguiNiceSettings::new(),
            self.initial_editor.take().unwrap(),
        )
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let sample_rate = context.transport().sample_rate;
        if (sample_rate - self.sample_rate).abs() > 1.0 {
            self.sample_rate = sample_rate;
            self.voice.set_sample_rate(sample_rate);
            self.reverb.set_sample_rate(sample_rate);
        }

        // note-on wyzwala werbel.
        let mut trigger = None;
        while let Some(event) = context.next_event() {
            if let NoteEvent::NoteOn { velocity, .. } = event {
                trigger = Some(velocity);
            }
        }

        for channel_samples in buffer.iter_samples() {
            let tone = self.params.tone.smoothed.next();
            let ring = self.params.ring.smoothed.next();
            let damping = self.params.damping.smoothed.next();
            let snare = self.params.snare.smoothed.next();
            let snap = self.params.snap.smoothed.next();
            let sweep = self.params.sweep.smoothed.next();
            let glue_amt = self.params.glue.smoothed.next();
            let verb_amt = self.params.verb.smoothed.next();
            let level = self.params.level.smoothed.next();
            self.voice.set_level(level);

            if let Some(vel) = trigger.take() {
                self.voice.trigger(tone, ring, damping, snare, snap, sweep, vel);
            }

            let dry = self.voice.process();
            let glued = self.glue.process(dry, glue_amt, sample_rate);
            let (wet_l, wet_r) = self.reverb.process(dry, verb_amt);

            let out_l = glued + wet_l;
            let out_r = glued + wet_r;

            let mut it = channel_samples.into_iter();
            if let Some(l) = it.next() {
                *l = out_l;
            }
            if let Some(r) = it.next() {
                *r = out_r;
            }
        }

        ProcessStatus::Normal
    }

    fn deactivate(&mut self) {}
}

impl ClapPlugin for Werbel {
    const CLAP_ID: &'static str = "com.gwolaniuk.werbel";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Snare drum synthesizer (feedback synthesis of the drum head)");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for Werbel {
    const VST3_CLASS_ID: [u8; 16] = *b"WrblFeedGwln0001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Instrument,
        Vst3SubCategory::Synth,
        Vst3SubCategory::Stereo,
    ];
}

nice_export_clap!(Werbel);
nice_export_vst3!(Werbel);