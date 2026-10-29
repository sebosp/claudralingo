//! The shortcuts exercise: Escape, the editing shortcuts on a line of text (select a word, copy,
//! paste, undo, cut...), the arrows, the numbers, the symbols, Tab and an Alt+Tab round trip to
//! another window and back, timed.
use std::collections::BTreeMap;

use bevy::feathers::constants::fonts;
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor, ThemedText};
use bevy::prelude::*;
use bevy::text::FontSource;
use bevy::window::WindowFocused;

use crate::game::{ActiveLayout, Coach, KeyPressed, Modifiers, NewExercise, TouchResult, Touched};
use crate::klavaro::rand_below;
use crate::layout::{Chord, KeyName, Layout, Mods, Special, Target};
use crate::progress::{ExerciseMode, Progress, Session, now_unix_seconds};
use crate::theme::tokens;
use crate::ui::ShortcutsRoot;

const SENTENCES: [&str; 5] = [
    "the quick brown fox jumps over the lazy dog",
    "pack my box with five dozen liquor jugs",
    "how vexingly quick daft zebras jump",
    "sphinx of black quartz judge my vow",
    "the five boxing wizards jump quickly",
];
const SYMBOLS: &str = "./@#$%^&*(){}?+=\\-[]_!~|:;<>'\"`";
const N_SYMBOLS: usize = 10;
/// The longer pauses are not counted in the time of a chord.
const MAX_CHORD_SECONDS: f64 = 10.0;

/// A line of text with a cursor, a selection, a clipboard and an undo history.
#[derive(Debug, Clone, Default)]
pub struct Editor {
    pub text: Vec<char>,
    pub cursor: usize,
    /// The other end of the selection.
    pub anchor: Option<usize>,
    pub clipboard: Vec<char>,
    undo: Vec<(Vec<char>, usize)>,
}

impl Editor {
    pub fn new(text: &str) -> Self {
        let text: Vec<char> = text.chars().collect();
        Editor { cursor: text.len(), text, ..default() }
    }

    pub fn selection(&self) -> Option<(usize, usize)> {
        self.anchor
            .filter(|a| *a != self.cursor)
            .map(|a| (a.min(self.cursor), a.max(self.cursor)))
    }

    pub fn selected(&self) -> String {
        self.selection().map(|(a, b)| self.text[a..b].iter().collect()).unwrap_or_default()
    }

    /// The end of the word after the cursor, as Ctrl+→ on Linux and macOS.
    fn word_right(&self, from: usize) -> usize {
        let mut i = from;
        while i < self.text.len() && self.text[i] == ' ' {
            i += 1;
        }
        while i < self.text.len() && self.text[i] != ' ' {
            i += 1;
        }
        i
    }

    fn word_left(&self, from: usize) -> usize {
        let mut i = from;
        while i > 0 && self.text[i - 1] == ' ' {
            i -= 1;
        }
        while i > 0 && self.text[i - 1] != ' ' {
            i -= 1;
        }
        i
    }

    fn move_to(&mut self, pos: usize, select: bool) {
        if select {
            self.anchor.get_or_insert(self.cursor);
        } else {
            self.anchor = None;
        }
        self.cursor = pos.min(self.text.len());
    }

    fn save_undo(&mut self) {
        self.undo.push((self.text.clone(), self.cursor));
    }

    fn delete_selection(&mut self) -> bool {
        let Some((a, b)) = self.selection() else {
            return false;
        };
        self.text.drain(a..b);
        self.cursor = a;
        self.anchor = None;
        true
    }

    pub fn apply(&mut self, chord: &Chord) {
        let Mods { ctrl, shift, .. } = chord.mods;
        match chord.key {
            KeyName::Special(Special::Left) => {
                let pos = if ctrl { self.word_left(self.cursor) } else { self.cursor.saturating_sub(1) };
                match self.selection() {
                    Some((a, _)) if !shift => self.move_to(a, false),
                    _ => self.move_to(pos, shift),
                }
            }
            KeyName::Special(Special::Right) => {
                let pos = if ctrl { self.word_right(self.cursor) } else { self.cursor + 1 };
                match self.selection() {
                    Some((_, b)) if !shift => self.move_to(b, false),
                    _ => self.move_to(pos, shift),
                }
            }
            KeyName::Special(Special::Home | Special::Up) => self.move_to(0, shift),
            KeyName::Special(Special::End | Special::Down) => self.move_to(self.text.len(), shift),
            KeyName::Special(Special::Escape) => self.anchor = None,
            KeyName::Special(Special::Backspace) => {
                self.save_undo();
                if !self.delete_selection() {
                    let from = if ctrl { self.word_left(self.cursor) } else { self.cursor.saturating_sub(1) };
                    self.text.drain(from..self.cursor);
                    self.cursor = from;
                }
            }
            KeyName::Special(Special::Delete) => {
                self.save_undo();
                if !self.delete_selection() {
                    let to = if ctrl { self.word_right(self.cursor) } else { (self.cursor + 1).min(self.text.len()) };
                    self.text.drain(self.cursor..to);
                }
            }
            KeyName::Char('a') if ctrl => {
                self.anchor = Some(0);
                self.cursor = self.text.len();
            }
            KeyName::Char('c') if ctrl => self.clipboard = self.selected().chars().collect(),
            KeyName::Char('x') if ctrl => {
                if self.selection().is_some() {
                    self.clipboard = self.selected().chars().collect();
                    self.save_undo();
                    self.delete_selection();
                }
            }
            KeyName::Char('v') if ctrl => {
                self.save_undo();
                self.delete_selection();
                let pasted = self.clipboard.clone();
                let n = pasted.len();
                self.text.splice(self.cursor..self.cursor, pasted);
                self.cursor += n;
            }
            KeyName::Char('z') if ctrl => {
                if let Some((text, cursor)) = self.undo.pop() {
                    self.text = text;
                    self.cursor = cursor;
                    self.anchor = None;
                }
            }
            KeyName::Char(c) if !ctrl && !chord.mods.alt && !chord.mods.super_key => {
                self.save_undo();
                self.delete_selection();
                self.text.insert(self.cursor, c);
                self.cursor += 1;
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    /// The chords edit the line of text.
    Editor,
    Keys,
    /// Leave the window with Alt+Tab and come back.
    AltTab,
}

#[derive(Debug, Clone)]
pub struct Task {
    pub title: &'static str,
    pub chords: Vec<Chord>,
    pub kind: TaskKind,
    /// The layout can not press it, Ctrl+Shift+→ without a shift key: the editing tasks are
    /// done for the user, so the next ones still make sense.
    pub skip: bool,
}

/// The chord, or one doing the same on a line of text when the layout has no such key: ↑ for
/// Home, ↓ for End, Backspace for Delete.
fn adapt(layout: &Layout, chord: Chord) -> Option<Chord> {
    let can = |c: Chord| layout.can_press(Target::Chord(c)).then_some(c);
    can(chord).or_else(|| {
        let KeyName::Special(special) = chord.key else {
            return None;
        };
        let instead = match special {
            Special::Home => Special::Up,
            Special::End => Special::Down,
            Special::Delete => Special::Backspace,
            _ => return None,
        };
        can(Chord::new(chord.mods, KeyName::Special(instead)))
    })
}

fn shuffled<T>(mut items: Vec<T>) -> Vec<T> {
    fastrand::shuffle(&mut items);
    items
}

/// One round: the editing chords follow each other on the same line, a wrong chord changes
/// nothing, the keys come in a random order.
pub fn draw_tasks(layout: &Layout) -> Vec<Task> {
    use Special::*;
    use TaskKind::*;
    let task = |title, kind, chords: Vec<Chord>| {
        let adapted: Vec<Option<Chord>> = chords.iter().map(|c| adapt(layout, *c)).collect();
        match kind {
            // The keys that do not exist are left out.
            Keys => {
                let chords: Vec<Chord> = adapted.into_iter().flatten().collect();
                Task { title, skip: chords.is_empty(), chords, kind }
            }
            _ if adapted.iter().all(Option::is_some) => {
                Task { title, chords: adapted.into_iter().flatten().collect(), kind, skip: false }
            }
            _ => Task { title, chords, kind, skip: true },
        }
    };
    let key = |special| Chord::special(special);
    let ctrl = |c| Chord::new(Mods::CTRL, KeyName::Char(c));
    let with = |mods, special| Chord::new(mods, KeyName::Special(special));
    let digits = shuffled(('0'..='9').map(Chord::char).collect());
    let symbols: Vec<Chord> = shuffled(SYMBOLS.chars().collect()).into_iter().take(N_SYMBOLS).map(Chord::char).collect();
    vec![
        task("Escape cancels and closes the menus: press Esc", Keys, vec![key(Escape)]),
        task("Go to the start of the line", Editor, vec![key(Home)]),
        task("Jump over the first word", Editor, vec![with(Mods::CTRL, Right)]),
        task("Select the next word", Editor, vec![with(Mods::CTRL_SHIFT, Right)]),
        task("Copy the selected word", Editor, vec![ctrl('c')]),
        task("Go to the end of the line", Editor, vec![key(End)]),
        task("Paste the word", Editor, vec![ctrl('v')]),
        task("Undo the paste", Editor, vec![ctrl('z')]),
        task("Select the last word", Editor, vec![with(Mods::CTRL_SHIFT, Left)]),
        task("Cut it", Editor, vec![ctrl('x')]),
        task("Paste it back", Editor, vec![ctrl('v')]),
        task("Delete the word before the cursor", Editor, vec![with(Mods::CTRL, Backspace)]),
        task("Delete the space too", Editor, vec![key(Backspace)]),
        task("Select everything", Editor, vec![ctrl('a')]),
        task("Delete it", Editor, vec![key(Delete)]),
        task("Undo, the text comes back", Editor, vec![ctrl('z')]),
        task("Press the arrows", Keys, shuffled(vec![key(Left), key(Up), key(Right), key(Down)])),
        task("Type the numbers", Keys, digits),
        task("Type the symbols", Keys, symbols),
        task("Tab goes to the next field, Shift+Tab back", Keys, vec![key(Tab), with(Mods::SHIFT, Tab)]),
        task("Switch to another window with Alt+Tab, then come back here", AltTab, vec![with(Mods::ALT, Tab)]),
    ]
}

#[derive(Resource, Debug, Clone)]
pub struct Drill {
    pub tasks: Vec<Task>,
    pub task: usize,
    /// The chord of the task to press next.
    pub step: usize,
    pub editor: Editor,
    pub n_steps: usize,
    pub done_steps: usize,
    /// The steps with at least one wrong chord.
    pub missed_steps: usize,
    step_missed: bool,
    started: Option<f64>,
    step_started: f64,
    pub away_since: Option<f64>,
    pub alt_tab: Option<f64>,
    /// The feedback of the last chord.
    pub note: String,
    pub finished: Option<f64>,
    pub summary: String,
    times: BTreeMap<String, f64>,
}

impl Drill {
    pub fn new(layout: &Layout) -> Self {
        let tasks = draw_tasks(layout);
        let n_steps = tasks.iter().filter(|t| !t.skip).map(|t| t.chords.len()).sum();
        let mut drill = Drill {
            tasks,
            task: 0,
            step: 0,
            editor: Editor::new(SENTENCES[rand_below(SENTENCES.len())]),
            n_steps,
            done_steps: 0,
            missed_steps: 0,
            step_missed: false,
            started: None,
            step_started: 0.0,
            away_since: None,
            alt_tab: None,
            note: String::new(),
            finished: None,
            summary: String::new(),
            times: BTreeMap::new(),
        };
        drill.skip_tasks(0.0);
        drill
    }

    /// Does the skipped tasks, until a task to press.
    fn skip_tasks(&mut self, now: f64) {
        while let Some(task) = self.tasks.get(self.task).filter(|t| t.skip) {
            if task.kind == TaskKind::Editor {
                for chord in task.chords.clone() {
                    self.editor.apply(&chord);
                }
            }
            self.task += 1;
        }
        if self.task >= self.tasks.len() && self.finished.is_none() {
            self.finished = Some(self.elapsed(now));
        }
    }

    /// The tasks the layout can not press, for the summary.
    pub fn skipped(&self) -> Vec<&'static str> {
        self.tasks.iter().filter(|t| t.skip).map(|t| t.title).collect()
    }

    pub fn current(&self) -> Option<&Task> {
        if self.finished.is_some() { None } else { self.tasks.get(self.task) }
    }

    pub fn expected(&self) -> Option<Chord> {
        self.current()?.chords.get(self.step).copied()
    }

    pub fn is_away(&self) -> bool {
        self.away_since.is_some()
    }

    pub fn elapsed(&self, now: f64) -> f64 {
        match (self.finished, self.started) {
            (Some(elapsed), _) => elapsed,
            (None, Some(started)) => now - started,
            (None, None) => 0.0,
        }
    }

    /// The next chord, the next task or the end.
    fn advance(&mut self, now: f64) {
        self.done_steps += 1;
        if self.step_missed {
            self.missed_steps += 1;
        }
        self.step_missed = false;
        self.step_started = now;
        self.step += 1;
        if self.step >= self.tasks[self.task].chords.len() {
            self.step = 0;
            self.task += 1;
            self.skip_tasks(now);
        }
    }

    pub fn accuracy(&self) -> f64 {
        if self.done_steps == 0 {
            0.0
        } else {
            100.0 * (self.done_steps - self.missed_steps) as f64 / self.done_steps as f64
        }
    }
}

fn record_hit(progress: &mut Progress, drill: &mut Drill, chord: &Chord, seconds: f64) {
    let stats = progress.current().shortcuts.entry(chord.name()).or_default();
    stats.hits += 1;
    stats.total_time += seconds.min(MAX_CHORD_SECONDS);
    let time = drill.times.entry(chord.name()).or_default();
    *time = time.max(seconds);
}

#[allow(clippy::too_many_arguments)]
pub fn handle_shortcuts(
    mut keys: MessageReader<KeyPressed>,
    mut focus: MessageReader<WindowFocused>,
    modifiers: Res<Modifiers>,
    time: Res<Time<Real>>,
    mut drill: ResMut<Drill>,
    mut progress: ResMut<Progress>,
    layout: Res<ActiveLayout>,
    mut coach: ResMut<Coach>,
    mut touched: MessageWriter<Touched>,
    mut new_exercise: MessageWriter<NewExercise>,
) {
    if progress.mode != ExerciseMode::Shortcuts {
        keys.clear();
        focus.clear();
        return;
    }
    let now = time.elapsed_secs_f64();
    let key_of = |chord: Chord| layout.0.plan(Target::Chord(chord)).map(|p| p.key);
    for event in focus.read() {
        let Some(expected) = drill.expected() else {
            continue;
        };
        if drill.current().map(|t| t.kind) != Some(TaskKind::AltTab) {
            continue;
        }
        if !event.focused && (modifiers.alt_at_focus_loss || modifiers.mods.alt) {
            drill.away_since = Some(now);
        } else if event.focused
            && let Some(since) = drill.away_since.take()
        {
            let seconds = now - since;
            drill.alt_tab = Some(seconds);
            drill.note = format!("Back in {seconds:.1} s");
            let current = progress.current();
            current.alt_tab_best = Some(current.alt_tab_best.map_or(seconds, |best| best.min(seconds)));
            record_hit(&mut progress, &mut drill, &expected, seconds);
            coach.touched(true, now);
            touched.write(Touched { pressed: None, expected: key_of(expected), result: TouchResult::Ok, shake: false });
            drill.advance(now);
        }
    }
    for key in keys.read() {
        let Some(chord) = key.chord else {
            continue;
        };
        if drill.finished.is_some() {
            if chord.key == KeyName::Special(Special::Enter) {
                new_exercise.write_default();
            }
            continue;
        }
        let Some(expected) = drill.expected() else {
            continue;
        };
        if drill.started.is_none() {
            drill.started = Some(now);
            drill.step_started = now;
        }
        let kind = drill.current().map(|t| t.kind);
        if kind == Some(TaskKind::AltTab) && chord == expected {
            // The window manager did not switch, the window kept the focus.
            drill.note = "Alt+Tab stayed here, switch to another window and back".into();
            continue;
        }
        let ok = chord == expected;
        if ok {
            if kind == Some(TaskKind::Editor) {
                drill.editor.apply(&chord);
            }
            let seconds = now - drill.step_started;
            record_hit(&mut progress, &mut drill, &chord, seconds);
            drill.note = String::new();
            drill.advance(now);
        } else {
            if !drill.step_missed {
                drill.step_missed = true;
                progress.current().shortcuts.entry(expected.name()).or_default().misses += 1;
            }
            drill.note = format!("That was {}, the task needs {}", chord.name(), expected.name());
        }
        coach.touched(ok, now);
        touched.write(Touched {
            pressed: key_of(chord),
            expected: key_of(expected),
            result: if ok { TouchResult::Ok } else { TouchResult::Wrong },
            shake: false,
        });
        if drill.finished.is_some() {
            finish(&mut drill, &mut progress);
        }
    }
    if drill.finished.is_some() && drill.summary.is_empty() {
        // Finished by the Alt+Tab round trip.
        finish(&mut drill, &mut progress);
    }
}

fn finish(drill: &mut Drill, progress: &mut Progress) {
    let elapsed = drill.finished.unwrap_or_default();
    let mut summary = format!(
        "{} keys and shortcuts in {elapsed:.0} s, {} missed, accuracy {:.0}%",
        drill.done_steps,
        drill.missed_steps,
        drill.accuracy()
    );
    let current = progress.current();
    if let Some(seconds) = drill.alt_tab {
        let best = current.alt_tab_best.unwrap_or(seconds);
        summary.push_str(&format!("\nAlt+Tab round trip {seconds:.1} s, best {best:.1} s"));
    }
    if let Some((name, seconds)) = drill.times.iter().filter(|(n, _)| *n != "Alt+Tab").max_by(|a, b| a.1.total_cmp(b.1)) {
        summary.push_str(&format!(" · slowest: {name} ({seconds:.1} s)"));
    }
    let skipped = drill.skipped();
    if !skipped.is_empty() {
        summary.push_str(&format!("\nNot on this keymap, done for you: {}", skipped.join(", ")));
    }
    summary.push_str("\nEnter: another round");
    drill.summary = summary;
    current.sessions.push(Session {
        mode: ExerciseMode::Shortcuts,
        lesson: None,
        accuracy: drill.accuracy(),
        velocity: 0.0,
        fluidness: 0.0,
        finished_at: now_unix_seconds(),
        elapsed,
    });
    progress.save();
}

pub struct ShortcutsPlugin;

impl Plugin for ShortcutsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_view.after(crate::game::GameSystems));
    }
}

fn text(commands: &mut Commands, parent: Entity, value: String, font: &TextFont, token: bevy::feathers::theme::ThemeToken) {
    commands.spawn((Text::new(value), font.clone(), ThemedText, ThemeTextColor(token), ChildOf(parent)));
}

/// Rebuilds the task, the chords and the line of text on every change, they are small.
fn update_view(
    drill: Res<Drill>,
    progress: Res<Progress>,
    roots: Query<Entity, With<ShortcutsRoot>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    if !drill.is_changed() && !progress.is_changed() {
        return;
    }
    let Ok(root) = roots.single() else {
        return;
    };
    commands.entity(root).despawn_children();
    if progress.mode != ExerciseMode::Shortcuts {
        return;
    }
    let mono = FontSource::Handle(asset_server.load(fonts::MONO));
    let font = |size: f32| TextFont { font: mono.clone(), font_size: FontSize::Px(size), ..default() };
    let row = |commands: &mut Commands, gap: f32| {
        commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(gap),
                    min_height: px(34),
                    ..default()
                },
                ChildOf(root),
            ))
            .id()
    };
    let Some(task) = drill.current() else {
        let line = row(&mut commands, 0.0);
        text(&mut commands, line, "Round done!".into(), &font(22.0), tokens::CHAR_CORRECT);
        return;
    };
    let line = row(&mut commands, 10.0);
    text(&mut commands, line, format!("{}/{}", drill.task + 1, drill.tasks.len()), &font(18.0), tokens::CHAR_PENDING);
    text(&mut commands, line, task.title.into(), &font(20.0), tokens::CHAR_CURRENT);

    // The chords of the task, done, current and to come.
    let chips = row(&mut commands, 8.0);
    for (i, chord) in task.chords.iter().enumerate() {
        let (text_token, border) = match i.cmp(&drill.step) {
            std::cmp::Ordering::Less => (tokens::CHAR_CORRECT, tokens::KEY_BORDER),
            std::cmp::Ordering::Equal => (tokens::CHAR_CURRENT, tokens::CURSOR),
            std::cmp::Ordering::Greater => (tokens::CHAR_PENDING, tokens::KEY_BORDER),
        };
        let chip = commands
            .spawn((
                Node {
                    padding: UiRect::axes(px(10), px(4)),
                    border: UiRect::all(px(if i == drill.step { 2 } else { 1 })),
                    border_radius: BorderRadius::all(px(6)),
                    ..default()
                },
                ThemeBackgroundColor(tokens::KEY_BG),
                ThemeBorderColor(border),
                ChildOf(chips),
            ))
            .id();
        text(&mut commands, chip, chord.name(), &font(20.0), text_token);
    }

    // The line of text with its selection and cursor.
    let editor = &drill.editor;
    let line = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                height: px(40),
                margin: UiRect::top(px(6)),
                padding: UiRect::horizontal(px(10)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            ThemeBackgroundColor(tokens::KEYBOARD_CASE),
            ChildOf(root),
        ))
        .id();
    let selection = editor.selection();
    let editing = task.kind == TaskKind::Editor;
    for i in 0..=editor.text.len() {
        let selected = selection.is_some_and(|(a, b)| (a..b).contains(&i));
        let node = commands
            .spawn((
                Node {
                    width: px(if i == editor.text.len() { 4.0 } else { 15.0 }),
                    height: px(32),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::left(px(2)),
                    ..default()
                },
                ThemeBackgroundColor(if selected { tokens::SELECTION } else { tokens::TRANSPARENT }),
                ThemeBorderColor(match (i == editor.cursor && editing, selected) {
                    (true, _) => tokens::CURSOR,
                    (false, true) => tokens::SELECTION,
                    (false, false) => tokens::TRANSPARENT,
                }),
                ChildOf(line),
            ))
            .id();
        if let Some(c) = editor.text.get(i) {
            let token = if editing { tokens::CHAR_CURRENT } else { tokens::CHAR_PENDING };
            text(&mut commands, node, c.to_string(), &font(22.0), token);
        }
    }

    let line = row(&mut commands, 18.0);
    if !editor.clipboard.is_empty() {
        let clip: String = editor.clipboard.iter().collect();
        text(&mut commands, line, format!("Clipboard: “{clip}”"), &font(15.0), tokens::CHAR_PENDING);
    }
    if !drill.note.is_empty() {
        let token = if drill.note.starts_with("That was") { tokens::CHAR_WRONG } else { tokens::CHAR_CORRECT };
        text(&mut commands, line, drill.note.clone(), &font(15.0), token);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The round makes sense when every chord is pressed as asked.
    #[test]
    fn editing_round() {
        let mut editor = Editor::new("the quick brown fox jumps over the lazy dog");
        let tasks = draw_tasks(&crate::layout::LayoutId::QwertyUs.build());
        let mut after = Vec::new();
        for task in tasks.iter().filter(|t| t.kind == TaskKind::Editor) {
            for chord in &task.chords {
                editor.apply(chord);
                after.push((editor.text.iter().collect::<String>(), editor.selected(), editor.clipboard.iter().collect::<String>()));
            }
        }
        // Ctrl+Shift+→ selects " quick", pasted at the end, then undone.
        assert_eq!(after[2].1, " quick");
        assert_eq!(after[5].0, "the quick brown fox jumps over the lazy dog quick");
        assert_eq!(after[6].0, "the quick brown fox jumps over the lazy dog");
        assert_eq!(after[7].1, "dog");
        assert_eq!(after[8].0, "the quick brown fox jumps over the lazy ");
        assert_eq!(after[10].0, "the quick brown fox jumps over the lazy ");
        assert_eq!(after[11].0, "the quick brown fox jumps over the lazy");
        assert_eq!(after[13].0, "");
        assert_eq!(after.last().unwrap().0, "the quick brown fox jumps over the lazy");
    }

    #[test]
    fn every_chord_has_a_plan() {
        for id in crate::layout::LayoutId::ALL {
            let layout = id.build();
            for task in draw_tasks(&layout).into_iter().filter(|t| !t.skip) {
                for chord in task.chords {
                    assert!(layout.can_press(Target::Chord(chord)), "{} on {id:?}", chord.name());
                }
            }
            // The editing round stays the same: every skipped task is done for the user.
            let mut drill = Drill::new(&layout);
            while let Some(chord) = drill.expected() {
                if drill.current().map(|t| t.kind) == Some(TaskKind::Editor) {
                    drill.editor.apply(&chord);
                }
                drill.advance(0.0);
            }
            assert!(drill.finished.is_some());
            assert!(drill.editor.text.iter().collect::<String>().split(' ').count() > 2, "{id:?}");
            for c in SYMBOLS.chars() {
                assert!(layout.plan(Target::Char(c)).is_some(), "{c} on {id:?}");
            }
        }
    }
}
