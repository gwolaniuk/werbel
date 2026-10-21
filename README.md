# Werbel

**VST3 / CLAP syntezator werbla oparty o _feedback synthesis_.**

> Autor: **gwolaniuk** · Rust + [nih-plug](https://github.com/robbert-vdh/nih-plug)

## Idea

Zamiast samplować, **modelujemy rezonans membrany** techniką opisaną w filmie
[„Synthesize SNARES that sound REAL using the power of FEEDBACK"](https://www.youtube.com/watch?v=1Db9rGbth_o):

1. Krótki, kilkumilisekundowy **transient** (szum) jest „katalizatorem" całego dźwięku.
2. Trafia w **pętlę sprzężenia zwrotnego**: delay + dolnoprzepustowy filtr w pętli.
3. Czas opóźnienia wyznacza **częstotliwość rezonansu** membrany (`f = sr / delay`),
   a filtr w pętli daje **tłumienie zależne od częstotliwości** — wyższe mody
   gasną szybciej, dokładnie jak w prawdziwym naciągu.
4. Druga, jaśniejsza pętla (rezonans ~8× wyżej) modeluje **druty/sprężyny** werbla,
   pobudzane tym samym transientem.

Całe brzmienie rośnie z jednego kilkumilisekundowego zderzenia — dlatego werbel
brzmi spójnie i „realnie", a nie jak doklejony szum do oscylatora.

## Parametry

| Parametr | Zakres | Domyślnie | Co robi |
|---|---|---|---|
| **Tone** | 80–600 Hz | 200 Hz | częstotliwość membrany (długość pętli) |
| **Ring** | 50–99.8% | 90% | sprzężenie zwrotne → długość wybrzmienia |
| **Damping** | 300–14000 Hz | 3500 Hz | jasność membrany (lowpass w pętli) |
| **Snare** | 0–100% | 60% | poziom drutów/sprężyn |
| **Snap** | 0–100% | 70% | poziom transjenta (trzask) |
| **Level** | 0–100% | 80% | głośność |

MIDI: każdy note-on wyzwala werbel (velocity skaluje transient).

## Architektura

```
src/
├── lib.rs   — definicja pluginu (nih-plug): parametry, MIDI, proces, VST3/CLAP export
└── dsp.rs   — czysty DSP bez zewnętrznych zależności:
               Resonator (delay + filtr w pętli), WhiteNoise (xorshift32),
               SnareVoice (membrana + sprężyny + transient)
```

## Budowanie

Wymagane: **Rust ≥ 1.88** (stable). Do linkowania potrzeba kompilatora C.

### Windows (MinGW, bez MSVC/Visual Studio)

```bash
rustup toolchain install stable-x86_64-pc-windows-gnu

# przenośny MinGW-w64 — winlibs (extract do ~/winlibs/)
# https://github.com/brechtsanders/winlibs_mingw/releases

# ustaw linker (raz):
export RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-gnu
export PATH="$HOME/winlibs/mingw64/bin:$PATH"
antconfig:
mkdir -p .cargo && cat > .cargo/config.toml <<'EOF'
[target.x86_64-pc-windows-gnu]
linker = "<sciezka>/winlibs/mingw64/bin/x86_64-w64-mingw32-gcc.exe"
ar = "<sciezka>/winlibs/mingw64/bin/ar.exe"
EOF

cargo build --release
```

### Windows (MSVC — rekomendowane)

Zainstaluj „Visual Studio 2022 Build Tools" (komponent „Desktop development with C++"), potem:

```bash
cargo build --release
```

### Pakowanie do .vst3

Docelowo przez `cargo nice-plug bundle` (wymaga `cargo install cargo-nice-plug`).
Ręcznie (Windows): skopiuj `target/release/werbel.dll` do
`Werbel.vst3/Contents/x86_64-win/Werbel.vst3` i wrzuć do katalogu VST3 hosta
(`C:\Program Files\Common Files\VST3\`).

## Roadmap

- [ ] `xtask` — pakowanie do `.vst3` / `.clap`
- [ ] pitch sweep membrany (delay modulowany amplitudą pętli) → większy realizm przy głośnym wyzwoleniu
- [ ] lekki reverb/kompresor końcowy (opcjonalny „glue")
- [ ] GUI (nih-plug egui) — suwaki + wizualizacja pętli
- [ ] więcej modów membrany (kilka delayów z różnymi tłumieniami)

## Licencja

MIT — zobacz [LICENSE](LICENSE). Technika syntezy wg filmu autora
(link powyżej); dźwięki wyjściowe są Twoje.