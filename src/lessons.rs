//! The lessons panel as data: the sections of a subject (the courses of the touch typing, the
//! HSK levels of the hanzi, the small letters of the penmanship) with their lessons, each
//! graded, open, current or ahead; and the grades. The panel draws them, see [`crate::ui`].
//!
//! The grades follow the three tiers of the games, from the goal up: Good from the goal of
//! the exercise, Great well above it, Legendary without a single error. Gold marks the top in
//! the games (the loot of World of Warcraft, Diablo, Genshin, the completed path of Duolingo),
//! purple the tier under it, green the first: the same here.
use bevy::feathers::theme::ThemeToken;

use crate::drills::FINGERS;
use crate::hanzi::data::HanziData;
use crate::klavaro::{KbdGrid, N_BASIC_LESSONS};
use crate::layout::{Layout, finger_name};
use crate::progress::{ExerciseMode, Progress, Session, Subject};
use crate::theme::tokens;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Grade {
    Good,
    Great,
    Legendary,
}

impl Grade {
    /// Of an accuracy against the goal of the exercise: none below the goal, Great from 60% of
    /// the way from the goal to 100 (98% for a goal of 95%), Legendary at 100%.
    pub fn of(accuracy: f64, goal: f64) -> Option<Grade> {
        if accuracy >= 99.95 {
            Some(Grade::Legendary)
        } else if accuracy >= goal + 0.6 * (100.0 - goal) {
            Some(Grade::Great)
        } else if accuracy >= goal {
            Some(Grade::Good)
        } else {
            None
        }
    }

    /// Of a hanzi level done: the misses over its ten words.
    pub fn of_misses(misses: u32) -> Grade {
        match misses {
            0 => Grade::Legendary,
            1..=4 => Grade::Great,
            _ => Grade::Good,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Grade::Good => "Good",
            Grade::Great => "Great",
            Grade::Legendary => "Legendary",
        }
    }

    pub fn token(self) -> ThemeToken {
        match self {
            Grade::Good => tokens::GRADE_GOOD,
            Grade::Great => tokens::GRADE_GREAT,
            Grade::Legendary => tokens::GRADE_LEGENDARY,
        }
    }
}

/// How far an exercise of the menu is, the star beside its name: not started, started (light
/// blue, under 20% of the way to Legendary), then the colors of the grades.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mastery {
    NotStarted,
    Started,
    Reached(Grade),
}

impl Mastery {
    pub fn token(self) -> ThemeToken {
        match self {
            Mastery::NotStarted => bevy::feathers::tokens::TEXT_DIM,
            Mastery::Started => tokens::GRADE_STARTED,
            Mastery::Reached(grade) => grade.token(),
        }
    }
}

/// The mastery of an exercise on the keyboard of the moment. A course counts its lessons, each
/// Good 1, Great 2, Legendary 3, out of 3 per lesson: all Good is Good, all Legendary Legendary.
/// The exercises without lessons take the best grade of their sessions.
pub fn mastery(progress: &Progress, mode: ExerciseMode, layout: &Layout) -> Mastery {
    let sessions = progress.layouts.get(progress.layout.key()).map(|l| l.sessions.as_slice()).unwrap_or_default();
    if !sessions.iter().any(|s| s.mode == mode) {
        return Mastery::NotStarted;
    }
    let lessons: Option<Vec<usize>> = match mode {
        ExerciseMode::Basic => Some((1..=N_BASIC_LESSONS).collect()),
        ExerciseMode::Layers => Some((1..=crate::course::layer_lessons(layout).len()).collect()),
        ExerciseMode::Fingers => Some(FINGERS.iter().map(|f| *f as usize).collect()),
        _ => None,
    };
    let goal = crate::game::goal(mode);
    let points = |accuracy: Option<f64>| -> usize {
        match accuracy.and_then(|a| Grade::of(a, goal)) {
            Some(Grade::Good) => 1,
            Some(Grade::Great) => 2,
            Some(Grade::Legendary) => 3,
            None => 0,
        }
    };
    let (sum, n) = match lessons {
        Some(lessons) => (lessons.iter().map(|l| points(best(sessions, mode, *l))).sum::<usize>(), lessons.len()),
        None => (points(sessions.iter().filter(|s| s.mode == mode).map(|s| s.accuracy).reduce(f64::max)), 1),
    };
    if n == 0 || sum * 5 < n * 3 {
        Mastery::Started
    } else if sum < 2 * n {
        Mastery::Reached(Grade::Good)
    } else if sum < 3 * n {
        Mastery::Reached(Grade::Great)
    } else {
        Mastery::Reached(Grade::Legendary)
    }
}

/// Where a lesson stands.
#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    /// Not reached yet: after the lesson `next`, the first one not passed. Locked, it waits for
    /// that one; unlocked, a click jumps ahead.
    Ahead { locked: bool, next: String },
    NotYet,
    /// Tried, the best accuracy below the goal.
    Below(f64),
    Passed(Grade),
}

/// Where a click on a lesson goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Go {
    Basic(usize),
    Layers(usize),
    Finger(u8),
    Hanzi(usize),
    Penmanship(usize),
    /// An exercise without lessons of the touch typing.
    Mode(ExerciseMode),
    /// The review of the weak hanzi words.
    HanziReview,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Lesson {
    /// "4  a;", "left pinky", "3/15".
    pub title: String,
    /// The words of a hanzi level, in the CJK font.
    pub cjk: Option<String>,
    pub status: Status,
    pub current: bool,
    pub go: Go,
}

/// A course, an HSK level: a node of the path with its lessons.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub name: String,
    pub lessons: Vec<Lesson>,
    /// Its lessons earn points, see [`points`]: not the free practice (the warm-up, your text,
    /// the weak words).
    pub scored: bool,
}

impl Section {
    pub fn passed(&self) -> usize {
        self.lessons.iter().filter(|l| matches!(l.status, Status::Passed(_))).count()
    }

    /// The grade of the section once every lesson passed: the lowest of them.
    pub fn grade(&self) -> Option<Grade> {
        self.lessons.iter().map(|l| if let Status::Passed(g) = l.status { Some(g) } else { None }).min().flatten()
    }

    pub fn current(&self) -> bool {
        self.lessons.iter().any(|l| l.current)
    }

    pub fn reached(&self) -> bool {
        self.lessons.iter().any(|l| !matches!(l.status, Status::Ahead { .. }))
    }
}

/// The best accuracy of the sessions of a lesson.
fn best(sessions: &[Session], mode: ExerciseMode, lesson: usize) -> Option<f64> {
    sessions.iter().filter(|s| s.mode == mode && s.lesson == Some(lesson)).map(|s| s.accuracy).fold(None, |best, a| Some(best.map_or(a, |b: f64| b.max(a))))
}

fn status(sessions: &[Session], mode: ExerciseMode, lesson: usize) -> Status {
    match best(sessions, mode, lesson) {
        Some(b) => Grade::of(b, crate::game::goal(mode)).map_or(Status::Below(b), Status::Passed),
        None => Status::NotYet,
    }
}

/// The lessons of a course with a lock: the ones after `unlocked` are ahead.
fn course(progress: &Progress, mode: ExerciseMode, titles: Vec<String>, unlocked: usize, current: usize, go: fn(usize) -> Go) -> Vec<Lesson> {
    let sessions = progress.layouts.get(progress.layout.key()).map(|l| l.sessions.as_slice()).unwrap_or_default();
    titles
        .into_iter()
        .enumerate()
        .map(|(i, title)| {
            let lesson = i + 1;
            let status = if lesson > unlocked {
                Status::Ahead { locked: progress.lock_lessons, next: format!("lesson {unlocked}") }
            } else {
                status(sessions, mode, lesson)
            };
            Lesson { title, cjk: None, status, current: lesson == current, go: go(lesson) }
        })
        .collect()
}

/// The characters of a basic lesson on a tile, the long sets shortened.
const MAX_TILE_CHARS: usize = 7;

/// The points of a subject: each lesson Good 1, Great 2, Legendary 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Points {
    pub earned: usize,
    pub total: usize,
    /// The lessons at Good or better, at Great or better, at Legendary: how full each part of
    /// the bar is.
    pub tiers: [usize; 3],
    pub lessons: usize,
}

/// The points of the scored sections.
pub fn points(sections: &[Section]) -> Points {
    let mut points = Points::default();
    for lesson in sections.iter().filter(|s| s.scored).flat_map(|s| &s.lessons) {
        points.lessons += 1;
        points.total += 3;
        if let Status::Passed(grade) = lesson.status {
            let n = grade as usize + 1;
            points.earned += n;
            for tier in &mut points.tiers[..n] {
                *tier += 1;
            }
        }
    }
    points
}

/// The sections of the subject of the moment.
pub fn sections(progress: &Progress, grid: &KbdGrid, layout: &Layout, hanzi: &HanziData) -> Vec<Section> {
    subject_sections(progress, progress.mode.subject(), grid, layout, hanzi)
}

/// The sections of a subject.
pub fn subject_sections(progress: &Progress, subject: Subject, grid: &KbdGrid, layout: &Layout, hanzi: &HanziData) -> Vec<Section> {
    match subject {
        Subject::Typing => typing_sections(progress, grid, layout),
        Subject::Hanzi => hanzi_sections(progress, hanzi),
        Subject::Penmanship => {
            let pen = &progress.penmanship;
            let titles = (0..crate::penmanship::n_lessons()).map(|l| crate::penmanship::lesson_name(l).to_string()).collect();
            vec![Section {
                name: "Small letters".into(),
                lessons: course(progress, ExerciseMode::Penmanship, titles, pen.unlocked + 1, pen.lesson + 1, |l| Go::Penmanship(l - 1)),
                scored: true,
            }]
        }
    }
}

fn typing_sections(progress: &Progress, grid: &KbdGrid, layout: &Layout) -> Vec<Section> {
    let current = progress.layouts.get(progress.layout.key());
    let (basic_lesson, basic_unlocked) = current.map_or((1, 1), |l| (l.basic_lesson, l.basic_unlocked));
    let titles = (1..=N_BASIC_LESSONS)
        .map(|lesson| {
            let chars = crate::klavaro::basic::lesson_char_set(lesson, grid);
            let mut shown: String = chars.iter().take(MAX_TILE_CHARS).collect();
            if chars.len() > MAX_TILE_CHARS {
                shown.push('…');
            }
            format!("{lesson}  {shown}")
        })
        .collect();
    let mut sections =
        vec![Section { name: "Basic course".into(), lessons: course(progress, ExerciseMode::Basic, titles, basic_unlocked, basic_lesson, Go::Basic), scored: true }];
    let layers = crate::course::layer_lessons(layout);
    if !layers.is_empty() {
        let (layer_lesson, layer_unlocked) = current.map_or((1, 1), |l| (l.layer_lesson, l.layer_unlocked));
        let titles = layers.iter().enumerate().map(|(i, l)| format!("{}  {}", i + 1, l.name)).collect();
        sections.push(Section {
            name: "Layer course".into(),
            lessons: course(progress, ExerciseMode::Layers, titles, layer_unlocked, layer_lesson, Go::Layers),
            scored: true,
        });
    }
    let sessions = current.map(|l| l.sessions.as_slice()).unwrap_or_default();
    let finger = current.map_or(FINGERS[0], |l| l.finger);
    let fingers = FINGERS
        .iter()
        .map(|f| Lesson {
            title: crate::ui::capitalize(finger_name(*f)),
            cjk: None,
            status: status(sessions, ExerciseMode::Fingers, *f as usize),
            current: *f == finger,
            go: Go::Finger(*f),
        })
        .collect();
    sections.push(Section { name: "Finger drills".into(), lessons: fingers, scored: true });
    // The exercises without lessons, graded by their best session.
    let mode_lesson = |mode: ExerciseMode| {
        let best = sessions.iter().filter(|s| s.mode == mode).map(|s| s.accuracy).reduce(f64::max);
        let status = match best {
            Some(b) => Grade::of(b, crate::game::goal(mode)).map_or(Status::Below(b), Status::Passed),
            None => Status::NotYet,
        };
        Lesson { title: mode.name().to_string(), cjk: None, status, current: progress.mode == mode, go: Go::Mode(mode) }
    };
    sections.push(Section { name: "Exercises".into(), lessons: EXERCISES.iter().map(|m| mode_lesson(*m)).collect(), scored: true });
    sections.push(Section {
        name: "Free practice".into(),
        lessons: [ExerciseMode::Review, ExerciseMode::Custom].iter().map(|m| mode_lesson(*m)).collect(),
        scored: false,
    });
    sections
}

/// The exercises of the touch typing without lessons, in the section "Exercises".
pub const EXERCISES: [ExerciseMode; 8] = [
    ExerciseMode::Adaptability,
    ExerciseMode::Velocity,
    ExerciseMode::Fluidness,
    ExerciseMode::Quotes,
    ExerciseMode::Affirmations,
    ExerciseMode::BounceLeft,
    ExerciseMode::BounceRight,
    ExerciseMode::Shortcuts,
];

fn hanzi_sections(progress: &Progress, data: &HanziData) -> Vec<Section> {
    let unlocked = crate::ui::hanzi_unlocked(progress);
    let mut sections: Vec<Section> = Vec::new();
    for (i, level) in data.levels.iter().enumerate() {
        if sections.last().is_none_or(|s| s.name != format!("HSK {}", level.hsk)) {
            sections.push(Section { name: format!("HSK {}", level.hsk), lessons: Vec::new(), scored: true });
        }
        let words: String = data.words[level.words.clone()].iter().take(4).map(|w| w.word.as_str()).collect::<Vec<_>>().join(" ");
        let status = match progress.hanzi.levels_done.get(&i) {
            Some(misses) => Status::Passed(Grade::of_misses(*misses)),
            None if i > unlocked => Status::Ahead { locked: progress.lock_lessons, next: format!("level {}", unlocked + 1) },
            None => Status::NotYet,
        };
        let lesson = Lesson { title: format!("{}/{}", level.part, level.parts), cjk: Some(words), status, current: i == progress.hanzi.level, go: Go::Hanzi(i) };
        if let Some(section) = sections.last_mut() {
            section.lessons.push(lesson);
        }
    }
    let weak = crate::hanzi::weak_words(progress, data).len();
    sections.push(Section {
        name: "Review".into(),
        lessons: vec![Lesson {
            title: if weak == 0 { "Weak words".into() } else { format!("{weak} weak words") },
            cjk: None,
            status: Status::NotYet,
            current: progress.mode == ExerciseMode::Hanzi && progress.hanzi.review,
            go: Go::HanziReview,
        }],
        scored: false,
    });
    sections
}

/// A click on a lesson: the mode and the lesson of the progress, the caller saves and starts
/// the exercise.
pub fn go(progress: &mut Progress, go: Go) {
    progress.words_of = None;
    match go {
        Go::Mode(mode) => progress.mode = mode,
        // Started by the caller, it needs the words, see [`crate::hanzi::start_review`].
        Go::HanziReview => progress.mode = ExerciseMode::Hanzi,
        Go::Basic(lesson) => {
            progress.mode = ExerciseMode::Basic;
            progress.current().basic_lesson = lesson;
        }
        Go::Layers(lesson) => {
            progress.mode = ExerciseMode::Layers;
            progress.current().layer_lesson = lesson;
        }
        Go::Finger(finger) => {
            progress.mode = ExerciseMode::Fingers;
            progress.current().finger = finger;
        }
        Go::Hanzi(level) => {
            progress.mode = ExerciseMode::Hanzi;
            progress.hanzi.level = level;
            progress.hanzi.word = 0;
            progress.hanzi.review = false;
        }
        Go::Penmanship(lesson) => {
            progress.mode = ExerciseMode::Penmanship;
            progress.penmanship.lesson = lesson;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::LayoutId;

    fn session(mode: ExerciseMode, lesson: usize, accuracy: f64) -> Session {
        Session { elapsed: 60.0, mode, lesson: Some(lesson), accuracy, velocity: 20.0, fluidness: 80.0, finished_at: 1_793_000_000, quality: None }
    }

    #[test]
    fn the_grades_start_at_the_goal_and_gold_needs_no_error() {
        assert_eq!(Grade::of(94.9, 95.0), None);
        assert_eq!(Grade::of(95.0, 95.0), Some(Grade::Good));
        assert_eq!(Grade::of(97.9, 95.0), Some(Grade::Good));
        assert_eq!(Grade::of(98.0, 95.0), Some(Grade::Great));
        assert_eq!(Grade::of(99.9, 95.0), Some(Grade::Great));
        assert_eq!(Grade::of(100.0, 95.0), Some(Grade::Legendary));
        // The penmanship: 75% to pass, 90% for Great.
        assert_eq!(Grade::of(80.0, 75.0), Some(Grade::Good));
        assert_eq!(Grade::of(90.0, 75.0), Some(Grade::Great));
        assert_eq!(Grade::of_misses(0), Grade::Legendary);
        assert_eq!(Grade::of_misses(5), Grade::Good);
        assert!(Grade::Good < Grade::Legendary);
    }

    #[test]
    fn the_star_of_an_exercise_goes_from_gray_to_gold() {
        let layout = LayoutId::QwertyUs.build();
        let mut progress = Progress { layout: LayoutId::QwertyUs, ..Progress::default() };
        let fingers: Vec<usize> = FINGERS.iter().map(|f| *f as usize).collect();
        let mode = ExerciseMode::Fingers;
        assert_eq!(mastery(&progress, mode, &layout), Mastery::NotStarted);
        // Tried and failed, then one drill out of many: under 20% of the way.
        progress.current().sessions.push(session(mode, fingers[0], 50.0));
        assert_eq!(mastery(&progress, mode, &layout), Mastery::Started);
        progress.current().sessions.push(session(mode, fingers[0], crate::drills::GOAL_ACCURACY));
        assert_eq!(mastery(&progress, mode, &layout), Mastery::Started);
        // Every drill at the grade.
        let every = |progress: &mut Progress, accuracy: f64| {
            for f in &fingers {
                progress.current().sessions.push(session(mode, *f, accuracy));
            }
        };
        every(&mut progress, crate::drills::GOAL_ACCURACY);
        assert_eq!(mastery(&progress, mode, &layout), Mastery::Reached(Grade::Good));
        every(&mut progress, 99.9);
        assert_eq!(mastery(&progress, mode, &layout), Mastery::Reached(Grade::Great));
        every(&mut progress, 100.0);
        assert_eq!(mastery(&progress, mode, &layout), Mastery::Reached(Grade::Legendary));
        // An exercise without lessons takes its best session.
        progress.current().sessions.push(Session { lesson: None, ..session(ExerciseMode::Velocity, 0, 96.0) });
        assert_eq!(mastery(&progress, ExerciseMode::Velocity, &layout), Mastery::Reached(Grade::Good));
        // Another keyboard starts over.
        progress.layout = LayoutId::ALL.into_iter().find(|l| *l != LayoutId::QwertyUs).unwrap();
        assert_eq!(mastery(&progress, mode, &layout), Mastery::NotStarted);
    }

    #[test]
    fn the_basic_course_is_graded_locked_and_followed() {
        let layout = LayoutId::QwertyUs.build();
        let grid = crate::klavaro::KbdGrid::qwerty_us();
        let hanzi = HanziData::load();
        let mut progress = Progress { mode: ExerciseMode::Basic, layout: LayoutId::QwertyUs, lock_lessons: true, ..Progress::default() };
        {
            let current = progress.current();
            current.basic_lesson = 4;
            current.basic_unlocked = 4;
            current.sessions.push(session(ExerciseMode::Basic, 1, 100.0));
            current.sessions.push(session(ExerciseMode::Basic, 2, 97.0));
            current.sessions.push(session(ExerciseMode::Basic, 3, 92.0));
            current.sessions.push(session(ExerciseMode::Basic, 3, 96.0));
            current.sessions.push(session(ExerciseMode::Fingers, 1, 98.5));
        }
        let sections = sections(&progress, &grid, &layout, &hanzi);
        // The QWERTY has a layer course of its symbols.
        assert_eq!(
            sections.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
            ["Basic course", "Layer course", "Finger drills", "Exercises", "Free practice"]
        );
        let basic = &sections[0];
        assert_eq!(basic.lessons.len(), N_BASIC_LESSONS);
        assert_eq!(basic.lessons[0].status, Status::Passed(Grade::Legendary));
        assert_eq!(basic.lessons[1].status, Status::Passed(Grade::Good));
        // The best time counts, 96% passes.
        assert_eq!(basic.lessons[2].status, Status::Passed(Grade::Good));
        assert_eq!(basic.lessons[3].status, Status::NotYet);
        assert!(basic.lessons[3].current);
        assert_eq!(basic.lessons[4].status, Status::Ahead { locked: true, next: "lesson 4".into() });
        assert_eq!(basic.passed(), 3);
        assert_eq!(basic.grade(), None);
        assert!(basic.lessons[0].title.starts_with("1  "));
        let fingers = &sections[2];
        assert_eq!(fingers.lessons[0].status, Status::Passed(Grade::Great));
        assert_eq!(fingers.lessons[0].title, "Left pinky");
        // Unlocked lessons are ahead but open to a jump.
        progress.lock_lessons = false;
        let sections = super::sections(&progress, &grid, &layout, &hanzi);
        assert_eq!(sections[0].lessons[10].status, Status::Ahead { locked: false, next: "lesson 4".into() });
        go(&mut progress, Go::Basic(11));
        assert_eq!(progress.current().basic_lesson, 11);
    }

    #[test]
    fn the_points_count_the_grades_of_the_scored_lessons() {
        let lesson = |status| Lesson { title: String::new(), cjk: None, status, current: false, go: Go::Basic(1) };
        let scored = Section {
            name: "Basic course".into(),
            lessons: vec![
                lesson(Status::Passed(Grade::Good)),
                lesson(Status::Passed(Grade::Legendary)),
                lesson(Status::Below(80.0)),
                lesson(Status::NotYet),
            ],
            scored: true,
        };
        let free = Section { name: "Free practice".into(), lessons: vec![lesson(Status::Passed(Grade::Great))], scored: false };
        let points = points(&[scored, free]);
        assert_eq!(points, Points { earned: 4, total: 12, tiers: [2, 1, 1], lessons: 4 });
        assert_eq!(points.tiers.iter().sum::<usize>(), points.earned);
    }

    #[test]
    fn the_hsk_levels_are_the_sections_of_the_hanzi() {
        let layout = LayoutId::QwertyUs.build();
        let grid = crate::klavaro::KbdGrid::qwerty_us();
        let hanzi = HanziData::load();
        let mut progress = Progress { mode: ExerciseMode::Hanzi, ..Progress::default() };
        progress.hanzi.levels_done.insert(0, 0);
        progress.hanzi.levels_done.insert(1, 7);
        progress.hanzi.level = 2;
        let sections = sections(&progress, &grid, &layout, &hanzi);
        assert_eq!(sections[0].name, "HSK 1");
        assert!(sections.len() >= 2);
        assert_eq!(sections[0].lessons[0].status, Status::Passed(Grade::Legendary));
        assert_eq!(sections[0].lessons[1].status, Status::Passed(Grade::Good));
        assert!(sections[0].lessons[2].current);
        assert!(sections[0].lessons[2].cjk.as_ref().is_some_and(|w| !w.is_empty()));
        assert!(matches!(sections[1].lessons[0].status, Status::Ahead { .. }));
        assert!(!sections[1].reached());
        go(&mut progress, Go::Hanzi(5));
        assert_eq!((progress.hanzi.level, progress.hanzi.word), (5, 0));
    }
}
