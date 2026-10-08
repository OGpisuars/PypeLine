//! The KiloKilo Games logo, drawn in code and animated when the game starts:
//! the shield's double outline draws itself in, the bold KK pops in, the
//! wings sweep out, KILOKILO rises letter by letter, GAMES fades in between
//! two rules, then it all fades into the PypeLine loading screen.
//!
//! Black on off-white, drawn with egui's painter at full window resolution
//! so the edges stay crisp. Any key or click skips ahead.

use bevy::prelude::*;
use bevy_egui::{
    EguiContexts, EguiPrimaryContextPass,
    egui::{self, Color32, Pos2, Shape, Stroke, pos2, vec2},
};

use super::screens::Screen;
use crate::audio::SoundCue;
use crate::audio::sfx::Sfx;

const PAPER: Color32 = Color32::from_rgb(240, 236, 228);
const INK: Color32 = Color32::from_rgb(20, 20, 24);
/// The loading screen's night sky, faded to at the end.
const NEXT: Color32 = Color32::from_rgb(24, 24, 56);

/// When the fade into the loading screen starts, and how long the logo runs.
const FADE_AT: f32 = 3.8;
const LENGTH: f32 = 4.3;
/// When the KK pops in, with its sound.
const POP_AT: f32 = 0.9;

#[derive(Resource, Default)]
struct LogoClock {
    seconds: f32,
    chimed: bool,
}

pub struct CompanyLogoPlugin;

impl Plugin for CompanyLogoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Screen::Logo), |mut commands: Commands| {
            commands.init_resource::<LogoClock>();
        })
        .add_systems(OnExit(Screen::Logo), |mut commands: Commands| {
            commands.remove_resource::<LogoClock>();
        })
        .add_systems(Update, advance.run_if(in_state(Screen::Logo)))
        .add_systems(EguiPrimaryContextPass, draw.run_if(in_state(Screen::Logo)));
    }
}

fn advance(
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    clock: Option<ResMut<LogoClock>>,
    mut next: ResMut<NextState<Screen>>,
    mut sounds: MessageWriter<SoundCue>,
) {
    let Some(mut clock) = clock else { return };
    // Large first frames (window creation, shader loading) would skip the
    // start of the animation, so each frame counts for at most 1/20 s.
    clock.seconds += time.delta_secs().min(0.05);
    let skipped =
        keys.get_just_pressed().next().is_some() || mouse.get_just_pressed().next().is_some();
    if skipped {
        clock.seconds = clock.seconds.max(FADE_AT);
    }
    if !clock.chimed && clock.seconds >= POP_AT {
        clock.chimed = true;
        if !skipped {
            sounds.write(SoundCue(Sfx::Logo));
        }
    }
    if clock.seconds >= LENGTH {
        next.set(Screen::Boot);
    }
}

fn draw(mut contexts: EguiContexts, clock: Option<Res<LogoClock>>) -> Result {
    let Some(clock) = clock else { return Ok(()) };
    let ctx = contexts.ctx_mut()?;
    let screen = ctx.viewport_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("company_logo"),
    ));
    painter.rect_filled(screen, 0.0, PAPER);
    let (origin, unit) = placement(screen);
    for shape in logo(origin, unit, clock.seconds) {
        painter.add(shape);
    }
    let fade = progress(clock.seconds, FADE_AT, LENGTH - FADE_AT);
    if fade > 0.0 {
        painter.rect_filled(screen, 0.0, NEXT.gamma_multiply(fade));
    }
    // Keep animating even without input.
    ctx.request_repaint();
    Ok(())
}

/// Where the shield's center goes, and the size of one logo unit, so the
/// whole logo fills about 60% of the window's height.
fn placement(screen: egui::Rect) -> (Pos2, f32) {
    // The logo spans x -2.6..2.6 and y -1.45..2.95 in units.
    let unit = (screen.height() * 0.6 / 4.4).min(screen.width() * 0.85 / 5.2);
    let center = screen.center();
    (pos2(center.x, center.y - 0.75 * unit), unit)
}

/// 0 before `start`, 1 after `start + length`, linear between.
fn progress(t: f32, start: f32, length: f32) -> f32 {
    ((t - start) / length).clamp(0.0, 1.0)
}

fn ease_out_cubic(x: f32) -> f32 {
    1.0 - (1.0 - x).powi(3)
}

fn ease_in_out(x: f32) -> f32 {
    x * x * (3.0 - 2.0 * x)
}

/// Overshoots a little before settling: for things that pop in.
fn ease_out_back(x: f32) -> f32 {
    let c = 1.70158;
    1.0 + (c + 1.0) * (x - 1.0).powi(3) + c * (x - 1.0).powi(2)
}

/// `INK` faded toward the paper: 0 = invisible, 1 = full ink.
fn ink(amount: f32) -> Color32 {
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * amount.clamp(0.0, 1.0)) as u8;
    Color32::from_rgb(
        mix(PAPER.r(), INK.r()),
        mix(PAPER.g(), INK.g()),
        mix(PAPER.b(), INK.b()),
    )
}

/// The shield's outline, in units, clockwise from the top left.
const SHIELD: [(f32, f32); 7] = [
    (-0.78, -1.2),
    (0.78, -1.2),
    (1.02, -0.9),
    (1.02, 0.3),
    (0.0, 1.25),
    (-1.02, 0.3),
    (-1.02, -0.9),
];

/// Everything the logo draws at time `t`, back to front.
fn logo(origin: Pos2, unit: f32, t: f32) -> Vec<Shape> {
    let at = |x: f32, y: f32| origin + vec2(x, y) * unit;
    let mut shapes = Vec::new();

    // Wings, behind the shield: they slide out from under it.
    for side in [-1.0f32, 1.0] {
        for i in 0..4 {
            let p = ease_out_cubic(progress(t, 1.05 + i as f32 * 0.07, 0.55));
            if p <= 0.0 {
                continue;
            }
            let slide = (1.0 - p) * 1.5;
            let i = i as f32;
            let y = -0.92 + i * 0.3;
            let tip = 2.55 - i * 0.3;
            let lift = 0.5 - i * 0.07;
            let feather = [
                (0.9, y),
                (tip, y - lift),
                (tip - 0.2, y - lift + 0.22),
                (0.9, y + 0.24),
            ];
            let points: Vec<Pos2> = feather
                .iter()
                .map(|&(x, y)| at(side * (x - slide), y))
                .collect();
            shapes.push(Shape::convex_polygon(points, ink(p * 1.5), Stroke::NONE));
        }
    }

    // The shield covers the wings' roots.
    let outer: Vec<Pos2> = SHIELD.iter().map(|&(x, y)| at(x, y)).collect();
    shapes.push(Shape::convex_polygon(outer.clone(), PAPER, Stroke::NONE));
    let inner: Vec<Pos2> = SHIELD
        .iter()
        .map(|&(x, y)| at(x * 0.82, y * 0.82 + 0.02))
        .collect();
    let outer_p = ease_in_out(progress(t, 0.15, 0.8));
    let inner_p = ease_in_out(progress(t, 0.55, 0.6));
    shapes.extend(outline(&outer, outer_p, Stroke::new(0.09 * unit, INK)));
    shapes.extend(outline(&inner, inner_p, Stroke::new(0.045 * unit, INK)));

    // KK pops in.
    let pop = progress(t, POP_AT, 0.45);
    if pop > 0.0 {
        let scale = ease_out_back(pop);
        let center = at(0.0, -0.12);
        let (w, h, gap) = (0.62, 0.95, 0.1);
        for left in [-(w + gap / 2.0), gap / 2.0] {
            for part in letter_k(w, h) {
                let points = part
                    .iter()
                    .map(|&(x, y)| {
                        let local = vec2(left + x, y - h / 2.0) * unit * scale;
                        center + local
                    })
                    .collect();
                shapes.push(Shape::convex_polygon(points, INK, Stroke::NONE));
            }
        }
    }

    // KILOKILO rises letter by letter.
    let em = 0.8;
    let spacing = 0.16;
    let word = "KILOKILO";
    let width = text_width(word, spacing) * em;
    let mut x = -width / 2.0;
    for (i, ch) in word.chars().enumerate() {
        let p = ease_out_cubic(progress(t, 1.5 + i as f32 * 0.055, 0.3));
        if p > 0.0 {
            let top = 1.55 + (1.0 - p) * 0.35;
            shapes.extend(letter(ch, at(x, top), em * unit, ink(p)));
        }
        x += (glyph_width(ch) + spacing) * em;
    }

    // GAMES, widely spaced, between two rules.
    let p = ease_out_cubic(progress(t, 2.1, 0.5));
    if p > 0.0 {
        let em = 0.36;
        let spacing = 0.2 + 0.25 * p;
        let word = "GAMES";
        let width = text_width(word, spacing) * em;
        let mut x = -width / 2.0;
        for ch in word.chars() {
            shapes.extend(letter(ch, at(x, 2.6), em * unit, ink(p)));
            x += (glyph_width(ch) + spacing) * em;
        }
        let rule_y = 2.6 + em / 2.0;
        let end = width / 2.0 + 0.18;
        let reach = end + (2.4 - end) * p;
        let rule = Stroke::new(0.05 * unit, ink(p));
        shapes.push(Shape::line_segment(
            [at(-end, rule_y), at(-reach, rule_y)],
            rule,
        ));
        shapes.push(Shape::line_segment(
            [at(end, rule_y), at(reach, rule_y)],
            rule,
        ));
    }
    shapes
}

/// A closed outline drawn in from the bottom point up both sides at once,
/// `p` of the way round (1 = whole).
fn outline(points: &[Pos2], p: f32, stroke: Stroke) -> Vec<Shape> {
    if p <= 0.0 {
        return Vec::new();
    }
    if p >= 1.0 {
        return vec![Shape::closed_line(points.to_vec(), stroke)];
    }
    // From the bottom point (index 4) to the top middle, each way round.
    let top = Pos2::new((points[0].x + points[1].x) / 2.0, points[0].y);
    let right = vec![points[4], points[3], points[2], points[1], top];
    let left = vec![points[4], points[5], points[6], points[0], top];
    [right, left]
        .into_iter()
        .map(|path| Shape::line(partial(&path, p), stroke))
        .collect()
}

/// The first `p` (0..1) of a path, by length.
fn partial(path: &[Pos2], p: f32) -> Vec<Pos2> {
    let total: f32 = path.windows(2).map(|w| w[0].distance(w[1])).sum();
    let mut left = total * p;
    let mut out = vec![path[0]];
    for w in path.windows(2) {
        if left <= 0.0 {
            break;
        }
        let length = w[0].distance(w[1]);
        if left >= length {
            out.push(w[1]);
            left -= length;
        } else {
            out.push(w[0] + (w[1] - w[0]) * (left / length));
            break;
        }
    }
    out
}

/// A bold K in a `w` by `h` box: a bar, two slanted arms, and the joint
/// where the arms meet the bar (without it a notch shows between them).
fn letter_k(w: f32, h: f32) -> [Vec<(f32, f32)>; 4] {
    let t = w * 0.32;
    let tw = t * 1.25;
    let a = h * 0.6;
    [
        vec![(0.0, 0.0), (t, 0.0), (t, h), (0.0, h)],
        vec![(t, a), (w - tw, 0.0), (w, 0.0), (t + tw, a)],
        vec![(t, h - a), (t + tw, h - a), (w, h), (w - tw, h)],
        vec![(t, h - a), (t + tw, h - a), (t + tw, a), (t, a)],
    ]
}

/// Letter widths, in ems (the letters are 1 em tall).
fn glyph_width(ch: char) -> f32 {
    match ch {
        'K' => 0.74,
        'I' => 0.26,
        'L' => 0.62,
        'O' | 'G' | 'A' => 0.82,
        'M' => 0.95,
        'E' => 0.66,
        'S' => 0.8,
        _ => 0.5,
    }
}

fn text_width(text: &str, spacing: f32) -> f32 {
    text.chars().map(|c| glyph_width(c) + spacing).sum::<f32>() - spacing
}

/// One heavy geometric letter with its top left at `top_left`, `em` tall.
fn letter(ch: char, top_left: Pos2, em: f32, color: Color32) -> Vec<Shape> {
    let t = 0.26;
    let h = t / 2.0;
    let c = 0.22;
    let w = glyph_width(ch);
    let pt = |x: f32, y: f32| top_left + vec2(x, y) * em;
    let stroke = Stroke::new(t * em, color);
    let line = |points: &[(f32, f32)]| {
        Shape::line(points.iter().map(|&(x, y)| pt(x, y)).collect(), stroke)
    };
    let block = |x: f32, y: f32, bw: f32, bh: f32| {
        Shape::rect_filled(
            egui::Rect::from_min_size(pt(x, y), vec2(bw, bh) * em),
            0.0,
            color,
        )
    };
    match ch {
        'K' => letter_k(w, 1.0)
            .into_iter()
            .map(|part| {
                Shape::convex_polygon(
                    part.into_iter().map(|(x, y)| pt(x, y)).collect(),
                    color,
                    Stroke::NONE,
                )
            })
            .collect(),
        'I' => vec![block(0.0, 0.0, t, 1.0)],
        'L' => vec![block(0.0, 0.0, t, 1.0), block(0.0, 1.0 - t, w, t)],
        'E' => vec![
            block(0.0, 0.0, t, 1.0),
            block(0.0, 0.0, w, t),
            block(0.0, 0.5 - t / 2.0, w * 0.85, t),
            block(0.0, 1.0 - t, w, t),
        ],
        'O' => vec![Shape::closed_line(
            [
                (c, h),
                (w - c, h),
                (w - h, c),
                (w - h, 1.0 - c),
                (w - c, 1.0 - h),
                (c, 1.0 - h),
                (h, 1.0 - c),
                (h, c),
            ]
            .iter()
            .map(|&(x, y)| pt(x, y))
            .collect(),
            stroke,
        )],
        'G' => vec![line(&[
            (w, h),
            (c, h),
            (h, c),
            (h, 1.0 - c),
            (c, 1.0 - h),
            (w - h, 1.0 - h),
            (w - h, 0.5),
            (0.42, 0.5),
        ])],
        'A' => vec![
            line(&[
                (h, 1.0),
                (h, c),
                (c, h),
                (w - c, h),
                (w - h, c),
                (w - h, 1.0),
            ]),
            block(0.0, 0.58 - t / 2.0, w, t),
        ],
        'M' => vec![line(&[
            (h, 1.0),
            (h, h),
            (w / 2.0, 0.55),
            (w - h, h),
            (w - h, 1.0),
        ])],
        'S' => vec![line(&[
            (w, h),
            (c, h),
            (h, c),
            (h, 0.5),
            (w - h, 0.5),
            (w - h, 1.0 - c),
            (w - c, 1.0 - h),
            (0.0, 1.0 - h),
        ])],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_logo_fits_the_window() {
        for size in [
            vec2(1440.0, 960.0),
            vec2(800.0, 1000.0),
            vec2(1920.0, 600.0),
        ] {
            let screen = egui::Rect::from_min_size(Pos2::ZERO, size);
            let (origin, unit) = placement(screen);
            let shapes = logo(origin, unit, FADE_AT);
            assert!(!shapes.is_empty());
            let bounds = shapes.iter().fold(egui::Rect::NOTHING, |r, s| {
                r.union(s.visual_bounding_rect())
            });
            assert!(screen.contains_rect(bounds), "{size:?}: {bounds:?}");
        }
    }

    #[test]
    fn nothing_shows_before_it_starts_and_all_of_it_by_the_hold() {
        let (origin, unit) = (Pos2::new(500.0, 400.0), 100.0);
        // Only the shield's paper fill at the very start.
        assert_eq!(logo(origin, unit, 0.0).len(), 1);
        let full = logo(origin, unit, FADE_AT).len();
        assert_eq!(full, logo(origin, unit, LENGTH).len());
        assert!(full > 20);
    }

    #[test]
    fn partial_paths_end_on_the_path() {
        let path = [pos2(0.0, 0.0), pos2(10.0, 0.0), pos2(10.0, 10.0)];
        assert_eq!(partial(&path, 0.5), vec![path[0], path[1]]);
        assert_eq!(partial(&path, 0.75).last(), Some(&pos2(10.0, 5.0)));
    }
}
