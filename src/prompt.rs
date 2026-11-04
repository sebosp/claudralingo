//! The text of the exercise: one node per character, colored by its Klavaro state.
use bevy::feathers::constants::fonts;
use bevy::feathers::theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor, ThemedText};
use bevy::prelude::*;
use bevy::text::FontSource;

use crate::game::Exercise;
use crate::klavaro::UPSYM;
use crate::klavaro::tutor::CharState;
use crate::theme::tokens;
use crate::ui::PromptRoot;

pub const LINE_HEIGHT: f32 = 40.0;
const CHAR_WIDTH: f32 = 17.0;
const FONT_SIZE: f32 = 26.0;
/// The large text setting, in the same width.
const LARGE: f32 = 1.4;
/// The paragraphs are wrapped at this many characters.
const MAX_COLUMNS: usize = 56;
/// The lines shown before and after the line of the cursor.
const LINES_BEFORE: usize = 1;
const LINES_AFTER: usize = 2;

/// The node of a character of the exercise, by its index in the text.
#[derive(Component)]
pub struct PromptChar(pub usize);

#[derive(Component)]
struct PromptLine;

#[derive(Resource, Default)]
pub struct PromptView {
    /// The nodes and the texts by the index in the text, `None` for the new lines.
    pub chars: Vec<Option<(Entity, Entity)>>,
    shown: Vec<CharState>,
    /// The visual lines with their first and last char index.
    lines: Vec<(Entity, usize, usize)>,
    cursor: Option<usize>,
    visible_line: Option<usize>,
}

impl PromptView {
    /// The visual lines: their node, their first and their end char index.
    pub fn lines(&self) -> &[(Entity, usize, usize)] {
        &self.lines
    }

    pub fn char_entity(&self, index: usize) -> Option<Entity> {
        self.chars.get(index).copied().flatten().map(|(node, _)| node)
    }
}

pub struct PromptPlugin;

impl Plugin for PromptPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PromptView>()
            .add_systems(Update, (rebuild_prompt, update_prompt).chain().after(crate::game::GameSystems));
    }
}

/// Wraps the paragraphs after a space, returns the char ranges of the visual lines.
pub fn wrap(text: &[char], max_columns: usize) -> Vec<(usize, usize)> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut last_space = None;
    for (i, c) in text.iter().enumerate() {
        if *c == '\n' {
            lines.push((start, i));
            start = i + 1;
            last_space = None;
            continue;
        }
        if *c == ' ' {
            last_space = Some(i);
        }
        if i + 1 - start > max_columns
            && let Some(space) = last_space
        {
            lines.push((start, space + 1));
            start = space + 1;
            last_space = None;
        }
    }
    if start < text.len() {
        lines.push((start, text.len()));
    }
    lines
}

fn state_tokens(state: CharState) -> (bevy::feathers::theme::ThemeToken, bevy::feathers::theme::ThemeToken) {
    match state {
        CharState::Untouched => (tokens::CHAR_PENDING, tokens::TRANSPARENT),
        CharState::Correct => (tokens::CHAR_CORRECT, tokens::TRANSPARENT),
        CharState::Wrong => (tokens::CHAR_WRONG, tokens::CHAR_WRONG_BG),
        CharState::Retouched => (tokens::CHAR_RETOUCHED, tokens::TRANSPARENT),
    }
}

fn rebuild_prompt(
    exercise: Res<Exercise>,
    mut view: ResMut<PromptView>,
    roots: Query<Entity, With<PromptRoot>>,
    asset_server: Res<AssetServer>,
    progress: Res<crate::progress::Progress>,
    mut commands: Commands,
    mut built: Local<Option<(u64, bool)>>,
) {
    let Ok(root) = roots.single() else {
        return;
    };
    // A new exercise has a new text, the large text other sizes.
    let large = progress.large_text;
    if *built == Some((exercise.id, large)) {
        return;
    }
    *built = Some((exercise.id, large));
    let scale = if large { LARGE } else { 1.0 };
    let (line_height, char_width) = (LINE_HEIGHT * scale, CHAR_WIDTH * scale);
    let max_columns = (MAX_COLUMNS as f32 / scale) as usize;
    commands.entity(root).despawn_children();
    let text = &exercise.tutor.text;
    let font = TextFont {
        font: FontSource::Handle(asset_server.load(fonts::MONO)),
        font_size: FontSize::Px(FONT_SIZE * scale),
        ..default()
    };
    *view = PromptView {
        chars: vec![None; text.len()],
        shown: vec![CharState::Untouched; text.len()],
        visible_line: Some(0),
        ..default()
    };
    for (start, end) in wrap(text, max_columns) {
        // The nodes of this frame can not be queried yet, the first lines are visible already.
        let visible = view.lines.len() <= LINES_AFTER;
        let line = commands
            .spawn((
                Node {
                    display: if visible { Display::Flex } else { Display::None },
                    flex_direction: FlexDirection::Row,
                    height: px(line_height),
                    ..default()
                },
                PromptLine,
                ChildOf(root),
            ))
            .id();
        view.lines.push((line, start, end));
        for (i, c) in text.iter().enumerate().take(end).skip(start) {
            if *c == '\n' {
                continue;
            }
            let shown = if *c == UPSYM { '¶' } else { *c };
            let node = commands
                .spawn((
                    Node {
                        width: px(char_width),
                        height: px(line_height - 4.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::bottom(px(3)),
                        border_radius: BorderRadius::all(px(3)),
                        ..default()
                    },
                    ThemeBackgroundColor(tokens::TRANSPARENT),
                    ThemeBorderColor(tokens::TRANSPARENT),
                    UiTransform::default(),
                    PromptChar(i),
                    ChildOf(line),
                ))
                .id();
            let text = commands
                .spawn((
                    Text::new(shown.to_string()),
                    font.clone(),
                    ThemedText,
                    ThemeTextColor(tokens::CHAR_PENDING),
                    ChildOf(node),
                ))
                .id();
            view.chars[i] = Some((node, text));
        }
    }
}

/// The line of a character, the last one for what is after the text.
fn line_of(lines: &[(Entity, usize, usize)], index: usize) -> usize {
    lines.iter().position(|(_, start, end)| (*start..*end).contains(&index)).unwrap_or(lines.len().saturating_sub(1))
}

/// The line to show the others around: the one of the cursor, or while there are errors to
/// erase the one of the first of them, a wrong key at the end of a line must not scroll it away.
/// Typing on and on past the lines shown, the cursor stays on the screen.
fn shown_line(lines: &[(Entity, usize, usize)], tutor: &crate::klavaro::tutor::Tutor) -> usize {
    let cursor = line_of(lines, tutor.cursor);
    match tutor.first_to_erase() {
        Some(wrong) => line_of(lines, wrong).max(cursor.saturating_sub(LINES_AFTER)),
        None => cursor,
    }
}

fn update_prompt(exercise: Res<Exercise>, mut view: ResMut<PromptView>, mut nodes: Query<&mut Node, With<PromptLine>>, mut commands: Commands) {
    let tutor = &exercise.tutor;
    if view.shown.len() != tutor.states.len() {
        return;
    }
    for (i, state) in tutor.states.iter().enumerate() {
        if view.shown[i] == *state {
            continue;
        }
        view.shown[i] = *state;
        if let Some((node, text)) = view.chars[i] {
            let (text_token, bg_token) = state_tokens(*state);
            commands.entity(text).insert(ThemeTextColor(text_token));
            commands.entity(node).insert(ThemeBackgroundColor(bg_token));
        }
    }
    let cursor = (tutor.query != crate::klavaro::tutor::Query::End).then_some(tutor.cursor);
    if view.cursor != cursor {
        if let Some(old) = view.cursor.and_then(|c| view.char_entity(c)) {
            commands.entity(old).insert(ThemeBorderColor(tokens::TRANSPARENT));
        }
        if let Some(new) = cursor.and_then(|c| view.char_entity(c)) {
            commands.entity(new).insert(ThemeBorderColor(tokens::CURSOR));
        }
        view.cursor = cursor;
    }
    let line = shown_line(&view.lines, tutor);
    if view.visible_line != Some(line) {
        view.visible_line = Some(line);
        for (i, (entity, _, _)) in view.lines.iter().enumerate() {
            if let Ok(mut node) = nodes.get_mut(*entity) {
                let visible = i + LINES_BEFORE >= line && i <= line + LINES_AFTER;
                node.display = if visible { Display::Flex } else { Display::None };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lines_wait_for_the_errors() {
        use crate::klavaro::BACKSPACE;
        use crate::klavaro::accuracy::Accuracy;
        use crate::klavaro::tutor::Tutor;
        let text = "fj\u{b6}\n".repeat(6);
        let mut tutor = Tutor::new(&text);
        let lines: Vec<(Entity, usize, usize)> =
            wrap(&tutor.text, MAX_COLUMNS).into_iter().map(|(start, end)| (Entity::PLACEHOLDER, start, end)).collect();
        let grid = crate::klavaro::KbdGrid::qwerty_us();
        let mut accur = Accuracy::default();
        let mut t = 0.0;
        let mut press = |tutor: &mut Tutor, keys: &str| {
            for c in keys.chars() {
                t += 0.2;
                tutor.process_touch(c, t, &mut accur, &grid);
            }
        };
        press(&mut tutor, "fj\u{b6}fj");
        assert_eq!(shown_line(&lines, &tutor), 1);
        // Space instead of Enter on the second line: the view stays there.
        press(&mut tutor, " ");
        assert_eq!(line_of(&lines, tutor.cursor), 2);
        assert_eq!(shown_line(&lines, &tutor), 1);
        // Typing on past the lines shown, the cursor stays visible.
        press(&mut tutor, "xxxxxxxxx");
        assert_eq!(line_of(&lines, tutor.cursor), 5);
        assert_eq!(shown_line(&lines, &tutor), 3);
        // Everything erased, back to the line of the cursor.
        press(&mut tutor, &BACKSPACE.to_string().repeat(10));
        assert_eq!(tutor.retro_pos, 0);
        assert_eq!(shown_line(&lines, &tutor), 1);
    }

    #[test]
    fn wrap_after_spaces() {
        let text: Vec<char> = format!("{}¶\nab cd¶\n", "word ".repeat(20)).chars().collect();
        let lines = wrap(&text, MAX_COLUMNS);
        assert!(lines.iter().all(|(s, e)| e - s <= MAX_COLUMNS + 1));
        assert_eq!(lines.last().map(|(s, e)| text[*s..*e].iter().collect::<String>()), Some("ab cd¶".into()));
        // Every char but the new lines is on a line.
        let covered: usize = lines.iter().map(|(s, e)| e - s).sum();
        assert_eq!(covered + 2, text.len());
    }
}
