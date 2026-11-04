//! The printable exercises: a page of the hanzi word in practice grids, or of the figures of the
//! penmanship lesson on their ruled rows, repeated across the page to trace, in black and
//! white. The page is a list of shapes in PDF points (A4), drawn with tiny-skia for the preview
//! and written to a PDF by hand: a page of paths needs no library.
use std::fmt::Write as _;

use bevy::asset::RenderAssetUsages;
use bevy::feathers::containers::{group, group_body, group_header};
use bevy::feathers::controls::*;
use bevy::feathers::display::{caption, label, label_dim};
use bevy::feathers::theme::UiTheme;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui_widgets::Activate;
use tiny_skia::{FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, StrokeDash, Transform};

use crate::hanzi::data::{BASELINE, BOX, HanziData, PathOp};
use crate::penmanship::letters::{Figure, Style};
use crate::progress::{ExerciseMode, Progress};
use crate::ui::Panel;

/// A4 in PDF points, the margin around the exercise.
pub const PAGE: Vec2 = Vec2::new(595.276, 841.89);
const MARGIN: f32 = 36.0;

/// A shape on the page, in points from the top left, gray 0 black to 1 white.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Line { from: Vec2, to: Vec2, width: f32, gray: f32, dash: Option<f32> },
    Polyline { points: Vec<Vec2>, width: f32, gray: f32 },
    Fill { ops: Vec<PathOp>, gray: f32 },
    Dot { at: Vec2, radius: f32, gray: f32 },
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Page {
    pub shapes: Vec<Shape>,
}

/// How many times the exercise repeats: across a row and the rows down the page, clamped to
/// what fits.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrintSetup {
    pub across: usize,
    pub rows: usize,
}

impl Default for PrintSetup {
    fn default() -> Self {
        PrintSetup { across: 8, rows: 10 }
    }
}

/// The gray of the model copy (the first of a row) and of the copies to trace.
const MODEL_GRAY: f32 = 0.1;
const TRACE_GRAY: f32 = 0.78;

/// The hanzi word in practice grids (米字格): the characters of the word in turn across each
/// row, the first copy of each in black, the others light to trace over.
pub fn hanzi_page(word: &str, data: &HanziData, setup: PrintSetup) -> Page {
    let mut page = Page::default();
    let chars: Vec<char> = word.chars().filter(|c| data.glyph(*c).is_some()).collect();
    if chars.is_empty() {
        return page;
    }
    let across = setup.across.clamp(1, 16);
    let cell = (PAGE.x - 2.0 * MARGIN) / across as f32;
    let rows = setup.rows.clamp(1, ((PAGE.y - 2.0 * MARGIN) / cell).floor().max(1.0) as usize);
    for row in 0..rows {
        for col in 0..across {
            let origin = Vec2::new(MARGIN + col as f32 * cell, MARGIN + row as f32 * cell);
            grid_cell(&mut page, origin, cell);
            let c = chars[col % chars.len()];
            if let Some(glyph) = data.glyph(c) {
                let gray = if col < chars.len() { MODEL_GRAY } else { TRACE_GRAY };
                for outline in &glyph.outlines {
                    page.shapes.push(Shape::Fill { ops: outline.iter().map(|op| op.map(|p| glyph_point(p, origin, cell))).collect(), gray });
                }
            }
        }
    }
    page
}

/// A point of the 1024 box of Make Me a Hanzi (y up from the baseline) in a cell of the page.
fn glyph_point(p: Vec2, origin: Vec2, cell: f32) -> Vec2 {
    let scale = cell / BOX;
    origin + Vec2::new(p.x * scale, (BASELINE - p.y) * scale)
}

/// The practice grid of a cell: the border, the dashed middle lines and diagonals.
fn grid_cell(page: &mut Page, origin: Vec2, cell: f32) {
    let corner = |x: f32, y: f32| origin + Vec2::new(x, y) * cell;
    let edges = [(corner(0.0, 0.0), corner(1.0, 0.0)), (corner(1.0, 0.0), corner(1.0, 1.0)), (corner(1.0, 1.0), corner(0.0, 1.0)), (corner(0.0, 1.0), corner(0.0, 0.0))];
    for (from, to) in edges {
        page.shapes.push(Shape::Line { from, to, width: 0.8, gray: 0.45, dash: None });
    }
    for (from, to) in [(corner(0.0, 0.5), corner(1.0, 0.5)), (corner(0.5, 0.0), corner(0.5, 1.0)), (corner(0.0, 0.0), corner(1.0, 1.0)), (corner(1.0, 0.0), corner(0.0, 1.0))] {
        page.shapes.push(Shape::Line { from, to, width: 0.4, gray: 0.7, dash: Some(3.0) });
    }
}

/// The figures of a penmanship lesson on ruled rows: the base and head lines, the ascender
/// and descender lines, the slant lines; the figures in turn across each row, the first copy
/// of each dark, the others light to trace over, a dot where each stroke starts.
pub fn penmanship_page(figures: &[Figure], style: Style, setup: PrintSetup) -> Page {
    let mut page = Page::default();
    if figures.is_empty() {
        return page;
    }
    let m = style.metrics();
    // The x-height on the page.
    let unit = 13.0;
    let band = (m.ascender - m.descender) * unit;
    let gap = 14.0;
    let rows = setup.rows.clamp(1, ((PAGE.y - 2.0 * MARGIN + gap) / (band + gap)).floor().max(1.0) as usize);
    let sheared: Vec<Vec<crate::penmanship::letters::Stroke>> = figures.iter().map(|f| f.on_sheet(style)).collect();
    let lean = 1.0 / m.slant.to_radians().tan();
    for row in 0..rows {
        let top = MARGIN + row as f32 * (band + gap);
        let base = top + m.ascender * unit;
        let y = |h: f32| base - h * unit;
        let line = |page: &mut Page, h: f32, width: f32, gray: f32, dash: Option<f32>| {
            page.shapes.push(Shape::Line { from: Vec2::new(MARGIN, y(h)), to: Vec2::new(PAGE.x - MARGIN, y(h)), width, gray, dash });
        };
        line(&mut page, 0.0, 0.9, 0.2, None);
        line(&mut page, 1.0, 0.7, 0.35, None);
        line(&mut page, m.ascender, 0.5, 0.6, None);
        line(&mut page, m.descender, 0.5, 0.6, None);
        line(&mut page, m.half_ascender, 0.4, 0.7, Some(2.5));
        line(&mut page, m.half_descender, 0.4, 0.7, Some(2.5));
        // The slant lines every space, across the band.
        let rise = (m.ascender - m.descender) * lean * unit;
        let mut x = MARGIN - rise;
        while x < PAGE.x - MARGIN {
            let (x0, x1) = (x.max(MARGIN), (x + rise).min(PAGE.x - MARGIN));
            if x1 > x0 {
                // Clipped to the margins along the slant.
                let t0 = (x0 - x) / rise;
                let t1 = (x1 - x) / rise;
                let from = Vec2::new(x0, y(m.descender) - t0 * (y(m.descender) - y(m.ascender)));
                let to = Vec2::new(x1, y(m.descender) - t1 * (y(m.descender) - y(m.ascender)));
                page.shapes.push(Shape::Line { from, to, width: 0.3, gray: 0.8, dash: None });
            }
            x += unit;
        }
        // The figures across the row, the model copy first.
        let mut x = MARGIN + 0.5 * unit;
        let mut copies = 0;
        let mut i = 0;
        while copies < setup.across.max(1) {
            let strokes = &sheared[i % sheared.len()];
            let (min_x, max_x) = strokes.iter().flat_map(|s| &s.points).fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.x), hi.max(p.x)));
            let width = (max_x - min_x) * unit;
            if x + width > PAGE.x - MARGIN {
                break;
            }
            let gray = if i < sheared.len() { MODEL_GRAY } else { TRACE_GRAY };
            for stroke in strokes {
                let points: Vec<Vec2> = stroke.points.iter().map(|p| Vec2::new(x + (p.x - min_x) * unit, y(p.y))).collect();
                if stroke.dot {
                    if let Some(p) = points.first() {
                        page.shapes.push(Shape::Dot { at: *p, radius: 1.2, gray });
                    }
                } else {
                    if let Some(start) = points.first() {
                        page.shapes.push(Shape::Dot { at: *start, radius: 1.6, gray: 0.3 });
                    }
                    page.shapes.push(Shape::Polyline { points, width: 1.0, gray });
                }
            }
            x += width + 1.2 * unit;
            copies += 1;
            i += 1;
        }
    }
    page
}

impl PathOp {
    fn map(self, f: impl Fn(Vec2) -> Vec2) -> PathOp {
        match self {
            PathOp::Move(p) => PathOp::Move(f(p)),
            PathOp::Line(p) => PathOp::Line(f(p)),
            PathOp::Quad(c, p) => PathOp::Quad(f(c), f(p)),
            PathOp::Cubic(a, b, p) => PathOp::Cubic(f(a), f(b), f(p)),
            PathOp::Close => PathOp::Close,
        }
    }
}

/// The page drawn in black and white, `scale` pixels per point.
pub fn render(page: &Page, scale: f32) -> Option<Pixmap> {
    let mut pixmap = Pixmap::new((PAGE.x * scale).ceil() as u32, (PAGE.y * scale).ceil() as u32)?;
    pixmap.fill(tiny_skia::Color::WHITE);
    let ts = Transform::from_scale(scale, scale);
    let paint = |gray: f32| {
        let v = (gray.clamp(0.0, 1.0) * 255.0) as u8;
        let mut paint = Paint::default();
        paint.set_color_rgba8(v, v, v, 255);
        paint.anti_alias = true;
        paint
    };
    for shape in &page.shapes {
        match shape {
            Shape::Line { from, to, width, gray, dash } => {
                let mut pb = PathBuilder::new();
                pb.move_to(from.x, from.y);
                pb.line_to(to.x, to.y);
                if let Some(path) = pb.finish() {
                    let stroke = Stroke { width: *width, dash: dash.and_then(|d| StrokeDash::new(vec![d, d], 0.0)), ..Default::default() };
                    pixmap.stroke_path(&path, &paint(*gray), &stroke, ts, None);
                }
            }
            Shape::Polyline { points, width, gray } => {
                let mut pb = PathBuilder::new();
                for (i, p) in points.iter().enumerate() {
                    if i == 0 { pb.move_to(p.x, p.y) } else { pb.line_to(p.x, p.y) }
                }
                if let Some(path) = pb.finish() {
                    let stroke = Stroke { width: *width, line_cap: LineCap::Round, line_join: LineJoin::Round, ..Default::default() };
                    pixmap.stroke_path(&path, &paint(*gray), &stroke, ts, None);
                }
            }
            Shape::Fill { ops, gray } => {
                let mut pb = PathBuilder::new();
                for op in ops {
                    match *op {
                        PathOp::Move(p) => pb.move_to(p.x, p.y),
                        PathOp::Line(p) => pb.line_to(p.x, p.y),
                        PathOp::Quad(c, p) => pb.quad_to(c.x, c.y, p.x, p.y),
                        PathOp::Cubic(a, b, p) => pb.cubic_to(a.x, a.y, b.x, b.y, p.x, p.y),
                        PathOp::Close => pb.close(),
                    }
                }
                if let Some(path) = pb.finish() {
                    pixmap.fill_path(&path, &paint(*gray), FillRule::Winding, ts, None);
                }
            }
            Shape::Dot { at, radius, gray } => {
                let mut pb = PathBuilder::new();
                pb.push_circle(at.x, at.y, *radius);
                if let Some(path) = pb.finish() {
                    pixmap.fill_path(&path, &paint(*gray), FillRule::Winding, ts, None);
                }
            }
        }
    }
    Some(pixmap)
}

/// The page as a PDF file: one A4 page, its shapes as a content stream.
pub fn pdf(page: &Page) -> Vec<u8> {
    let mut content = String::new();
    // PDF counts from the bottom left: the page is flipped.
    let flip = |p: Vec2| (p.x, PAGE.y - p.y);
    let _ = writeln!(content, "1 J 1 j");
    for shape in &page.shapes {
        match shape {
            Shape::Line { from, to, width, gray, dash } => {
                let (a, b) = (flip(*from), flip(*to));
                match dash {
                    Some(d) => {
                        let _ = writeln!(content, "[{d:.2} {d:.2}] 0 d");
                    }
                    None => {
                        let _ = writeln!(content, "[] 0 d");
                    }
                }
                let _ = writeln!(content, "{gray:.3} G {width:.2} w {:.2} {:.2} m {:.2} {:.2} l S", a.0, a.1, b.0, b.1);
            }
            Shape::Polyline { points, width, gray } => {
                let _ = writeln!(content, "[] 0 d {gray:.3} G {width:.2} w");
                for (i, p) in points.iter().enumerate() {
                    let (x, y) = flip(*p);
                    let _ = writeln!(content, "{x:.2} {y:.2} {}", if i == 0 { "m" } else { "l" });
                }
                let _ = writeln!(content, "S");
            }
            Shape::Fill { ops, gray } => {
                let _ = writeln!(content, "{gray:.3} g");
                let mut last = Vec2::ZERO;
                for op in ops {
                    match *op {
                        PathOp::Move(p) => {
                            let (x, y) = flip(p);
                            let _ = writeln!(content, "{x:.2} {y:.2} m");
                            last = p;
                        }
                        PathOp::Line(p) => {
                            let (x, y) = flip(p);
                            let _ = writeln!(content, "{x:.2} {y:.2} l");
                            last = p;
                        }
                        PathOp::Quad(c, p) => {
                            // A quadratic curve as a cubic one.
                            let c1 = last + (c - last) * (2.0 / 3.0);
                            let c2 = p + (c - p) * (2.0 / 3.0);
                            let ((x1, y1), (x2, y2), (x, y)) = (flip(c1), flip(c2), flip(p));
                            let _ = writeln!(content, "{x1:.2} {y1:.2} {x2:.2} {y2:.2} {x:.2} {y:.2} c");
                            last = p;
                        }
                        PathOp::Cubic(a, b, p) => {
                            let ((x1, y1), (x2, y2), (x, y)) = (flip(a), flip(b), flip(p));
                            let _ = writeln!(content, "{x1:.2} {y1:.2} {x2:.2} {y2:.2} {x:.2} {y:.2} c");
                            last = p;
                        }
                        PathOp::Close => {
                            let _ = writeln!(content, "h");
                        }
                    }
                }
                let _ = writeln!(content, "f");
            }
            Shape::Dot { at, radius, gray } => {
                // A circle of four Bézier arcs.
                let (cx, cy) = flip(*at);
                let (r, k) = (*radius, 0.5523 * radius);
                let _ = writeln!(content, "{gray:.3} g {:.2} {cy:.2} m", cx + r);
                let _ = writeln!(content, "{:.2} {:.2} {:.2} {:.2} {cx:.2} {:.2} c", cx + r, cy + k, cx + k, cy + r, cy + r);
                let _ = writeln!(content, "{:.2} {:.2} {:.2} {:.2} {:.2} {cy:.2} c", cx - k, cy + r, cx - r, cy + k, cx - r);
                let _ = writeln!(content, "{:.2} {:.2} {:.2} {:.2} {cx:.2} {:.2} c", cx - r, cy - k, cx - k, cy - r, cy - r);
                let _ = writeln!(content, "{:.2} {:.2} {:.2} {:.2} {:.2} {cy:.2} c f", cx + k, cy - r, cx + r, cy - k, cx + r);
            }
        }
    }
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.3} {:.3}] /Contents 4 0 R >>", PAGE.x, PAGE.y),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len() + 1),
    ];
    let mut out = String::from("%PDF-1.4\n%\u{e2}\u{e3}\u{cf}\u{d3}\n");
    let mut offsets = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{object}\nendobj", i + 1);
    }
    let xref = out.len();
    let _ = writeln!(out, "xref\n0 {}\n0000000000 65535 f ", objects.len() + 1);
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(out, "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1);
    out.into_bytes()
}

/// The page of the exercise of the moment, and its file name.
fn current_page(progress: &Progress, data: &HanziData, hanzi: &crate::hanzi::Practice, pen: &crate::penmanship::Practice, setup: PrintSetup) -> Option<(Page, String)> {
    match progress.mode {
        ExerciseMode::Hanzi => {
            let word = hanzi.word(data);
            let name = format!("hanzi-{}", crate::hanzi::data::toneless(&word.pinyin).replace(' ', ""));
            Some((hanzi_page(&word.word, data, setup), name))
        }
        ExerciseMode::Penmanship => {
            let lesson = pen.lesson()?;
            let name = format!("penmanship-{}", lesson.name.to_lowercase().replace(|c: char| !c.is_ascii_alphanumeric(), "-"));
            Some((penmanship_page(&lesson.figures, pen.style(), setup), name))
        }
        _ => None,
    }
}

// The preview panel.

#[derive(Component, Default, Clone)]
struct PrintOverlay;
#[derive(Component, Default, Clone)]
struct PreviewNode;
#[derive(Component, Default, Clone)]
struct AcrossLabel;
#[derive(Component, Default, Clone)]
struct RowsLabel;
#[derive(Component, Default, Clone)]
struct StatusText;
#[derive(Component, Default, Clone)]
struct PrintTitle;

/// The image of the preview, and the page shown, kept for the PDF.
#[derive(Resource, Default)]
struct Preview {
    image: Handle<Image>,
    page: Option<(Page, String)>,
    shown: Option<(PrintSetup, ExerciseMode, usize, String)>,
}

/// The preview is rendered at this many pixels per point.
const PREVIEW_SCALE: f32 = 1.5;

pub struct PrintPlugin;

impl Plugin for PrintPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PrintSetup>().init_resource::<Preview>().add_systems(Startup, setup).add_systems(Update, (show, refresh));
    }
}

fn setup(mut preview: ResMut<Preview>, mut images: ResMut<Assets<Image>>) {
    let image = Image::new_fill(
        Extent3d { width: (PAGE.x * PREVIEW_SCALE).ceil() as u32, height: (PAGE.y * PREVIEW_SCALE).ceil() as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    preview.image = images.add(image);
}

/// The print preview: the page, how many times across and down, the PDF.
pub fn overlay() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            display: Display::None,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        GlobalZIndex(50)
        PrintOverlay
        Children [
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
            }
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6))
            on(close_click)
            --
            @group()
            Node {
                max_width: percent(96),
                max_height: percent(96),
            }
            Children [
                @group_header() Children [ @label("") PrintTitle ]
                --
                @group_body()
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(16),
                    padding: px(12),
                    align_items: AlignItems::Start,
                }
                Children [
                    // The page, white, at the proportions of A4.
                    Node {
                        width: px(440),
                        height: px(440.0 * 841.89 / 595.276),
                        border: UiRect::all(px(1)),
                    }
                    BorderColor::all(Color::srgb(0.5, 0.5, 0.5))
                    ImageNode
                    PreviewNode
                    --
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: px(12),
                        width: px(300),
                    }
                    Children [
                        @label_dim("Black and white, A4. The first copy of each row is the model, the others are light to trace over.")
                        --
                        Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(8) }
                        Children [
                            @label("Across") Node { min_width: px(70) }
                            --
                            @FeathersButton { @caption: bsn! { @caption("−") } }
                            on(across_less)
                            --
                            @label("") AcrossLabel Node { min_width: px(36), justify_content: JustifyContent::Center }
                            --
                            @FeathersButton { @caption: bsn! { @caption("+") } }
                            on(across_more)
                        ]
                        --
                        Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(8) }
                        Children [
                            @label("Rows") Node { min_width: px(70) }
                            --
                            @FeathersButton { @caption: bsn! { @caption("−") } }
                            on(rows_less)
                            --
                            @label("") RowsLabel Node { min_width: px(36), justify_content: JustifyContent::Center }
                            --
                            @FeathersButton { @caption: bsn! { @caption("+") } }
                            on(rows_more)
                        ]
                        --
                        @FeathersButton {
                            @caption: bsn! { @caption("Save the PDF") }
                            @variant: ButtonVariant::Primary,
                        }
                        on(save_pdf)
                        --
                        @label_dim("") StatusText
                        --
                        @FeathersButton { @caption: bsn! { @caption("Close") } }
                        on(close)
                    ]
                ]
            ]
        ]
    }
}

/// The printer button opens the preview.
pub fn open(_: On<Activate>, mut panel: ResMut<Panel>, mut focus: ResMut<InputFocus>) {
    *panel = Panel::Print;
    focus.clear();
}

fn close(_: On<Activate>, mut panel: ResMut<Panel>, mut focus: ResMut<InputFocus>) {
    *panel = Panel::None;
    focus.clear();
}

fn close_click(_: On<bevy::picking::events::PointerClick>, mut panel: ResMut<Panel>) {
    *panel = Panel::None;
}

fn step(setup: &mut PrintSetup, across: isize, rows: isize) {
    setup.across = (setup.across as isize + across).clamp(1, 16) as usize;
    setup.rows = (setup.rows as isize + rows).clamp(1, 24) as usize;
}

fn across_less(_: On<Activate>, mut setup: ResMut<PrintSetup>, mut focus: ResMut<InputFocus>) {
    step(&mut setup, -1, 0);
    focus.clear();
}

fn across_more(_: On<Activate>, mut setup: ResMut<PrintSetup>, mut focus: ResMut<InputFocus>) {
    step(&mut setup, 1, 0);
    focus.clear();
}

fn rows_less(_: On<Activate>, mut setup: ResMut<PrintSetup>, mut focus: ResMut<InputFocus>) {
    step(&mut setup, 0, -1);
    focus.clear();
}

fn rows_more(_: On<Activate>, mut setup: ResMut<PrintSetup>, mut focus: ResMut<InputFocus>) {
    step(&mut setup, 0, 1);
    focus.clear();
}

/// Where the PDFs go: the Downloads folder, else the home (`%USERPROFILE%` on Windows).
#[cfg(not(target_arch = "wasm32"))]
fn pdf_dir() -> Option<std::path::PathBuf> {
    let home = std::path::PathBuf::from(std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?);
    let downloads = home.join("Downloads");
    Some(if downloads.is_dir() { downloads } else { home })
}

/// The page written as `claudralingo-<name>.pdf` in the Downloads folder, or at `path`; what
/// happened, for the panel.
pub fn save(page: &Page, name: &str, path: Option<std::path::PathBuf>) -> String {
    let bytes = pdf(page);
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        format!("The browser cannot save files yet: {} bytes of PDF for claudralingo-{name}.pdf", bytes.len())
    }
    #[cfg(not(target_arch = "wasm32"))]
    match path.or_else(|| pdf_dir().map(|dir| dir.join(format!("claudralingo-{name}.pdf")))) {
        Some(path) => match std::fs::write(&path, &bytes) {
            Ok(()) => format!("Saved: {}", path.display()),
            Err(e) => format!("Not saved: {e}"),
        },
        None => "Not saved: no home folder".to_string(),
    }
}

/// The page of the exercise of the moment, for the screenshot harness.
pub fn save_current(progress: &Progress, data: &HanziData, hanzi: &crate::hanzi::Practice, pen: &crate::penmanship::Practice, path: std::path::PathBuf) -> Option<String> {
    let (page, name) = current_page(progress, data, hanzi, pen, PrintSetup::default())?;
    Some(save(&page, &name, Some(path)))
}

fn save_pdf(_: On<Activate>, preview: Res<Preview>, mut texts: Query<&mut Text, With<StatusText>>, mut focus: ResMut<InputFocus>) {
    focus.clear();
    let Some((page, name)) = &preview.page else {
        return;
    };
    let status = save(page, name, None);
    for mut text in &mut texts {
        text.0.clone_from(&status);
    }
}

fn show(panel: Res<Panel>, mut overlays: Query<&mut Node, With<PrintOverlay>>, mut texts: Query<&mut Text, With<StatusText>>) {
    if !panel.is_changed() {
        return;
    }
    for mut node in &mut overlays {
        node.display = if *panel == Panel::Print { Display::Flex } else { Display::None };
    }
    if *panel == Panel::Print {
        for mut text in &mut texts {
            text.0.clear();
        }
    }
}

/// The page of the moment rendered when the preview opens or its setup changes.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn refresh(
    panel: Res<Panel>,
    setup: Res<PrintSetup>,
    progress: Res<Progress>,
    data: Res<HanziData>,
    hanzi: Res<crate::hanzi::Practice>,
    pen: Res<crate::penmanship::Practice>,
    mut preview: ResMut<Preview>,
    mut images: ResMut<Assets<Image>>,
    mut nodes: Query<&mut ImageNode, With<PreviewNode>>,
    mut texts: ParamSet<(Query<&mut Text, With<AcrossLabel>>, Query<&mut Text, With<RowsLabel>>, Query<&mut Text, With<PrintTitle>>)>,
    _theme: Res<UiTheme>,
) {
    if *panel != Panel::Print {
        return;
    }
    let Some((page, name)) = current_page(&progress, &data, &hanzi, &pen, *setup) else {
        return;
    };
    let key = (*setup, progress.mode, page.shapes.len(), name.clone());
    if preview.shown.as_ref() == Some(&key) && !panel.is_changed() {
        return;
    }
    preview.shown = Some(key);
    for mut node in &mut nodes {
        if node.image != preview.image {
            node.image = preview.image.clone();
        }
    }
    if let Some(pixmap) = render(&page, PREVIEW_SCALE)
        && let Some(mut image) = images.get_mut(&preview.image)
    {
        image.data = Some(pixmap.data().to_vec());
    }
    for mut text in &mut texts.p0() {
        text.0 = setup.across.to_string();
    }
    for mut text in &mut texts.p1() {
        text.0 = setup.rows.to_string();
    }
    for mut text in &mut texts.p2() {
        text.0 = format!("Print · {}", name.replace('-', " "));
    }
    preview.page = Some((page, name));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hanzi_page_has_its_grids_and_the_pdf_its_parts() {
        let data = HanziData::load();
        let setup = PrintSetup { across: 6, rows: 4 };
        let page = hanzi_page("你好", &data, setup);
        let cells = page.shapes.iter().filter(|s| matches!(s, Shape::Line { dash: None, .. })).count() / 4;
        assert_eq!(cells, 24);
        let fills = page.shapes.iter().filter(|s| matches!(s, Shape::Fill { .. })).count();
        assert!(fills > 24, "{fills} strokes");
        // The characters take turns, the first two of a row are the models.
        let models = page.shapes.iter().filter(|s| matches!(s, Shape::Fill { gray, .. } if *gray == MODEL_GRAY)).count();
        assert!(models > 0 && models < fills);
        let bytes = pdf(&page);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.contains("/Type /Page ") && text.contains(" c\n") && text.ends_with("%%EOF\n"));
        // The xref offsets point at the objects.
        let offset: usize = text.lines().find(|l| l.ends_with(" 00000 n ")).map(|l| l[..10].parse().unwrap()).unwrap();
        assert!(text[offset..].starts_with("1 0 obj"));
        assert!(render(&page, 0.5).is_some());
        // Too many rows are cut to the page.
        assert_eq!(hanzi_page("人", &data, PrintSetup { across: 4, rows: 99 }).shapes.iter().filter(|s| matches!(s, Shape::Line { dash: None, .. })).count() / 4, 4 * 5);
    }

    #[test]
    fn the_penmanship_page_repeats_the_figures_on_ruled_rows() {
        let lessons = crate::penmanship::letters::lessons();
        let setup = PrintSetup { across: 5, rows: 3 };
        let page = penmanship_page(&lessons[0].figures, Style::Spencerian, setup);
        // Six push-pulls per copy, five copies, three rows.
        let strokes = page.shapes.iter().filter(|s| matches!(s, Shape::Polyline { .. })).count();
        assert_eq!(strokes, 6 * 5 * 3);
        let base_lines = page.shapes.iter().filter(|s| matches!(s, Shape::Line { width, .. } if *width == 0.9)).count();
        assert_eq!(base_lines, 3);
        // A page of letters: the three figures of the lesson take turns.
        let letters = penmanship_page(&lessons[3].figures, Style::Copperplate, PrintSetup { across: 9, rows: 2 });
        assert!(letters.shapes.iter().any(|s| matches!(s, Shape::Dot { radius, .. } if *radius == 1.2)), "the dots of the i");
        assert!(pdf(&letters).len() > 1000);
    }
}
