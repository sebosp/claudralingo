//! The sounds of the typing (the setting "Sounds"): a bell on a miss, a short chime on a line
//! typed without a slip, a rising chord on a passed exercise and a falling "uh-oh" for a key
//! held too long by the auto shift. And the key clicks (their own setting, off at first), in the
//! sound of a blue, brown or red mechanical switch. They are synthesized at the start, no files.
//!
//! The advice of the auto shift is also shown over the text, where the eyes are while typing,
//! not only above the keyboard.
use std::sync::Arc;

use bevy::feathers::theme::{ThemeBackgroundColor, ThemeTextColor};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::game::{Coach, Exercise, GameSystems, TouchResult, Touched};
use crate::klavaro::tutor::{CharState, Query as TutorQuery};
use crate::progress::Progress;
use crate::theme::tokens;
use crate::ui::ExerciseArea;

const SAMPLE_RATE: u32 = 22_050;
/// The sounds stay under the speech of the hanzi and the music of the user.
const VOLUME: f32 = 0.28;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    Miss,
    HeldTooLong,
    CleanLine,
    Passed,
    Perfect,
}

/// The mechanical switch the key clicks sound like.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Switch {
    /// Clicky: a sharp click as the key goes down, a second one as it comes back.
    Blue,
    /// Tactile: a soft tick at the bump, then the key bottoms out.
    #[default]
    Brown,
    /// Linear: no click, a low thock at the bottom.
    Red,
}

impl Switch {
    pub const ALL: [Switch; 3] = [Switch::Blue, Switch::Brown, Switch::Red];

    pub fn name(self) -> &'static str {
        match self {
            Switch::Blue => "Blue, clicky",
            Switch::Brown => "Brown, tactile",
            Switch::Red => "Red, linear",
        }
    }
}

/// The key clicks are softer than the cues, there is one per key.
const CLICK_VOLUME: f32 = 0.2;

/// A burst of noise through a one-pole filter: its start and decay in seconds, the coefficient
/// of the low-pass (1 lets everything through), high-passed instead for the bright clicks.
struct Burst {
    start: f32,
    decay: f32,
    lowpass: f32,
    highpass: bool,
    amplitude: f32,
}

/// A ringing of the plastic: start, frequency, decay and amplitude.
struct Ring(f32, f32, f32, f32);

/// The click of a switch, from noise bursts and short rings, about 90 ms.
fn click_samples(switch: Switch) -> Vec<f32> {
    let burst = |start, decay, lowpass, highpass, amplitude| Burst { start, decay, lowpass, highpass, amplitude };
    let (bursts, rings) = match switch {
        Switch::Blue => (
            vec![burst(0.0, 0.0012, 0.9, true, 1.0), burst(0.011, 0.0015, 0.8, true, 0.7), burst(0.0, 0.010, 0.15, false, 0.35)],
            vec![Ring(0.0, 3400.0, 0.003, 0.35), Ring(0.011, 2900.0, 0.003, 0.25), Ring(0.002, 260.0, 0.012, 0.4)],
        ),
        Switch::Brown => (
            vec![burst(0.0, 0.0018, 0.5, true, 0.45), burst(0.003, 0.012, 0.12, false, 0.6)],
            vec![Ring(0.003, 210.0, 0.016, 0.7), Ring(0.0, 1800.0, 0.002, 0.15)],
        ),
        Switch::Red => (vec![burst(0.0, 0.010, 0.08, false, 0.6)], vec![Ring(0.0, 170.0, 0.02, 0.9), Ring(0.0, 1200.0, 0.0015, 0.08)]),
    };
    let n = (0.09 * SAMPLE_RATE as f32) as usize;
    let mut samples = vec![0.0f32; n];
    // The same noise every start: a xorshift.
    let mut seed = 0x9e37_79b9u32 ^ (switch as u32 + 1).wrapping_mul(0x85eb_ca6b);
    let mut noise = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed as f32 / u32::MAX as f32 * 2.0 - 1.0
    };
    for b in &bursts {
        let mut low = 0.0f32;
        for (i, sample) in samples.iter_mut().enumerate() {
            let t = i as f32 / SAMPLE_RATE as f32 - b.start;
            let x = noise();
            low += b.lowpass * (x - low);
            if t < 0.0 {
                continue;
            }
            let envelope = (t / 0.0005).min(1.0) * (-t / b.decay).exp();
            *sample += b.amplitude * envelope * if b.highpass { x - low } else { low };
        }
    }
    for Ring(start, frequency, decay, amplitude) in &rings {
        for (i, sample) in samples.iter_mut().enumerate() {
            let t = i as f32 / SAMPLE_RATE as f32 - start;
            if t >= 0.0 {
                *sample += amplitude * (t / 0.0005).min(1.0) * (-t / decay).exp() * (std::f32::consts::TAU * frequency * t).sin();
            }
        }
    }
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs())).max(1e-6);
    let tail = (0.01 * SAMPLE_RATE as f32) as usize;
    for (i, sample) in samples.iter_mut().enumerate() {
        let fade = ((n - i) as f32 / tail as f32).min(1.0);
        *sample *= CLICK_VOLUME * fade / peak;
    }
    samples
}

#[derive(Resource)]
struct Sounds {
    miss: Handle<AudioSource>,
    held: Handle<AudioSource>,
    line: Handle<AudioSource>,
    passed: Handle<AudioSource>,
    perfect: Handle<AudioSource>,
    /// The clicks, in the order of [`Switch::ALL`].
    clicks: Vec<Handle<AudioSource>>,
}

impl Sounds {
    fn get(&self, cue: Cue) -> Handle<AudioSource> {
        match cue {
            Cue::Miss => self.miss.clone(),
            Cue::HeldTooLong => self.held.clone(),
            Cue::CleanLine => self.line.clone(),
            Cue::Passed => self.passed.clone(),
            Cue::Perfect => self.perfect.clone(),
        }
    }
}

/// The advice over the text, with the time it fades out.
#[derive(Component)]
struct AdviceBadge {
    text: Entity,
    shown_until: f64,
}

pub struct SoundsPlugin;

impl Plugin for SoundsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, make_sounds)
            .add_systems(Update, (play_cues, show_advice).after(GameSystems));
    }
}

/// A partial of a sound: its frequency, its amplitude and the time it decays by e.
struct Partial(f32, f32, f32);

/// A note at `start` seconds made of partials, with a 4 ms attack against the clicks.
struct Note {
    start: f32,
    partials: Vec<Partial>,
}

/// The samples of the notes, mixed and normalized to the volume.
fn synthesize(notes: &[Note], length: f32) -> Vec<f32> {
    let n = (length * SAMPLE_RATE as f32) as usize;
    let mut samples = vec![0.0f32; n];
    for note in notes {
        let first = (note.start * SAMPLE_RATE as f32) as usize;
        for (i, sample) in samples.iter_mut().enumerate().skip(first) {
            let t = (i - first) as f32 / SAMPLE_RATE as f32;
            let attack = (t / 0.004).min(1.0);
            for Partial(frequency, amplitude, decay) in &note.partials {
                *sample += attack * amplitude * (-t / decay).exp() * (std::f32::consts::TAU * frequency * t).sin();
            }
        }
    }
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs())).max(1e-6);
    // The last 10 ms fade out.
    let tail = (0.01 * SAMPLE_RATE as f32) as usize;
    for (i, sample) in samples.iter_mut().enumerate() {
        let fade = ((n - i) as f32 / tail as f32).min(1.0);
        *sample *= VOLUME * fade / peak;
    }
    samples
}

/// A mono 16 bit WAV file of the samples.
fn wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        bytes.extend_from_slice(&((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).to_le_bytes());
    }
    bytes
}

/// A small bell: the inharmonic partials of a bell, a short ring.
fn bell(start: f32, frequency: f32) -> Note {
    Note {
        start,
        partials: vec![
            Partial(frequency, 1.0, 0.35),
            Partial(frequency * 2.756, 0.45, 0.18),
            Partial(frequency * 5.404, 0.2, 0.08),
        ],
    }
}

/// A soft tone, a sine with its octave.
fn tone(start: f32, frequency: f32, decay: f32) -> Note {
    Note { start, partials: vec![Partial(frequency, 1.0, decay), Partial(frequency * 2.0, 0.15, decay * 0.5)] }
}

fn cue_samples(cue: Cue) -> Vec<f32> {
    match cue {
        Cue::Miss => synthesize(&[bell(0.0, 740.0)], 0.6),
        Cue::HeldTooLong => synthesize(&[tone(0.0, 523.25, 0.09), tone(0.13, 392.0, 0.16)], 0.5),
        Cue::CleanLine => synthesize(&[tone(0.0, 1318.5, 0.07)], 0.25),
        // C E G, then the octave for a perfect exercise.
        Cue::Passed => synthesize(&[tone(0.0, 523.25, 0.2), tone(0.09, 659.25, 0.2), tone(0.18, 783.99, 0.35)], 0.9),
        Cue::Perfect => synthesize(
            &[tone(0.0, 523.25, 0.2), tone(0.09, 659.25, 0.2), tone(0.18, 783.99, 0.25), tone(0.27, 1046.5, 0.45)],
            1.1,
        ),
    }
}

fn make_sounds(mut sources: ResMut<Assets<AudioSource>>, mut commands: Commands) {
    let clicks = Switch::ALL.iter().map(|s| sources.add(AudioSource { bytes: Arc::from(wav(&click_samples(*s))) })).collect();
    let mut add = |cue| sources.add(AudioSource { bytes: Arc::from(wav(&cue_samples(cue))) });
    commands.insert_resource(Sounds {
        clicks,
        miss: add(Cue::Miss),
        held: add(Cue::HeldTooLong),
        line: add(Cue::CleanLine),
        passed: add(Cue::Passed),
        perfect: add(Cue::Perfect),
    });
}

/// The end of a line just typed: the index of its `\n` between the cursor before and now.
fn line_end_crossed(text: &[char], before: usize, now: usize) -> Option<usize> {
    (before..now.min(text.len())).rev().find(|i| text[*i] == '\n')
}

/// The characters of the line ending at `end` typed right the first time.
fn line_is_clean(text: &[char], states: &[CharState], end: usize) -> bool {
    let start = text[..end].iter().rposition(|c| *c == '\n').map_or(0, |i| i + 1);
    (start..end).all(|i| matches!(states[i], CharState::Correct) || text[i] == '\n')
}

#[allow(clippy::too_many_arguments)]
fn play_cues(
    mut touches: MessageReader<Touched>,
    exercise: Res<Exercise>,
    coach: Res<Coach>,
    progress: Res<Progress>,
    sounds: Option<Res<Sounds>>,
    mut last: Local<(u64, usize, f64, bool)>,
    mut clicks: Local<(u32, Option<(bool, Switch)>)>,
    mut commands: Commands,
) {
    let (last_id, last_cursor, last_flash, was_end) = &mut *last;
    let tutor = &exercise.tutor;
    if *last_id != exercise.id {
        *last_id = exercise.id;
        *last_cursor = tutor.cursor;
        *was_end = tutor.query == TutorQuery::End;
    }
    let flash = coach.flash.as_ref().map_or(*last_flash, |(_, until)| *until);
    let held = flash != *last_flash;
    *last_flash = flash;

    let mut cue = None;
    // A click per key, a little higher or lower each time; one when the switch is picked.
    let (count, setting) = &mut *clicks;
    let picked = setting.is_some_and(|s| s != (progress.key_clicks, progress.switch));
    *setting = Some((progress.key_clicks, progress.switch));
    let n_clicks = touches.len() + usize::from(picked);
    if progress.key_clicks
        && let Some(sounds) = sounds.as_ref()
        && let Some(click) = Switch::ALL.iter().position(|s| *s == progress.switch).and_then(|i| sounds.clicks.get(i))
    {
        for _ in 0..n_clicks {
            *count = count.wrapping_add(1);
            let speed = 0.94 + 0.12 * ((*count as f32 * 0.618_034).fract());
            commands.spawn((AudioPlayer(click.clone()), PlaybackSettings::DESPAWN.with_speed(speed)));
        }
    }
    for touch in touches.read() {
        if touch.result == TouchResult::Wrong {
            // The first key of a slip rings, not the keys typed on after it.
            let first = !progress.mode.is_typing() || tutor.retro_pos <= 1;
            if first {
                cue = Some(if held { Cue::HeldTooLong } else { Cue::Miss });
            }
        }
    }
    if progress.mode.is_typing() {
        if let Some(end) = line_end_crossed(&tutor.text, *last_cursor, tutor.cursor)
            && tutor.retro_pos == 0
            && line_is_clean(&tutor.text, &tutor.states, end)
        {
            cue = cue.or(Some(Cue::CleanLine));
        }
        let is_end = tutor.query == TutorQuery::End;
        if is_end && !*was_end
            && let Some(results) = tutor.results
        {
            if results.n_errors == 0 {
                cue = Some(Cue::Perfect);
            } else if results.accuracy >= crate::game::goal(exercise.mode) {
                cue = Some(Cue::Passed);
            }
        }
        *was_end = is_end;
        *last_cursor = tutor.cursor;
    }
    if let (Some(cue), Some(sounds)) = (cue, sounds)
        && progress.typing_sounds
    {
        commands.spawn((AudioPlayer(sounds.get(cue)), PlaybackSettings::DESPAWN));
    }
}

/// The advice of the coach (the auto shift) as a badge over the text, for 2.5 seconds.
fn show_advice(
    coach: Res<Coach>,
    time: Res<Time<Real>>,
    area: Query<Entity, With<ExerciseArea>>,
    mut badges: Query<(&mut AdviceBadge, &mut Node)>,
    mut texts: Query<&mut Text>,
    mut commands: Commands,
) {
    let now = time.elapsed_secs_f64();
    let advice = coach.flash.as_ref().filter(|(_, until)| now < *until);
    let Ok((mut badge, mut node)) = badges.single_mut() else {
        let Ok(area) = area.single() else {
            return;
        };
        let badge = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: px(-30),
                    padding: UiRect::axes(px(14), px(6)),
                    border_radius: BorderRadius::all(px(8)),
                    display: Display::None,
                    ..default()
                },
                ThemeBackgroundColor(tokens::KEY_SHIFT),
                GlobalZIndex(40),
                Pickable::IGNORE,
                ChildOf(area),
            ))
            .id();
        let text = commands
            .spawn((
                Text::new(""),
                TextFont { font_size: FontSize::Px(20.0), ..default() },
                ThemeTextColor(tokens::KEY_TEXT_ON_HINT),
                ChildOf(badge),
            ))
            .id();
        commands.entity(badge).insert(AdviceBadge { text, shown_until: 0.0 });
        return;
    };
    match advice {
        Some((message, until)) => {
            if badge.shown_until != *until {
                badge.shown_until = *until;
                if let Ok(mut text) = texts.get_mut(badge.text) {
                    text.0 = format!("⇧  {message}");
                }
            }
            if node.display != Display::Flex {
                node.display = Display::Flex;
            }
        }
        None => {
            if node.display != Display::None {
                node.display = Display::None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_clicks_are_short_and_soft() {
        for switch in Switch::ALL {
            let samples = click_samples(switch);
            assert!(samples.len() < SAMPLE_RATE as usize / 10);
            assert!(samples.iter().all(|s| s.abs() <= CLICK_VOLUME + 1e-4));
            assert!(samples.iter().any(|s| s.abs() > CLICK_VOLUME * 0.9));
        }
        assert_ne!(click_samples(Switch::Blue), click_samples(Switch::Red));
    }

    #[test]
    fn the_sounds_are_short_wav_files() {
        for cue in [Cue::Miss, Cue::HeldTooLong, Cue::CleanLine, Cue::Passed, Cue::Perfect] {
            let samples = cue_samples(cue);
            assert!(samples.len() < SAMPLE_RATE as usize * 2);
            assert!(samples.iter().all(|s| s.abs() <= VOLUME + 1e-4));
            let bytes = wav(&samples);
            assert_eq!(&bytes[..4], b"RIFF");
            assert_eq!(bytes.len(), 44 + 2 * samples.len());
        }
    }

    #[test]
    fn a_clean_line_has_no_slip() {
        let text: Vec<char> = "ab\ncd\n".chars().collect();
        use CharState::*;
        let states = [Correct, Correct, Untouched, Retouched, Correct, Untouched];
        assert_eq!(line_end_crossed(&text, 1, 3), Some(2));
        assert!(line_is_clean(&text, &states, 2));
        assert!(!line_is_clean(&text, &states, 5));
        assert_eq!(line_end_crossed(&text, 3, 4), None);
    }
}
