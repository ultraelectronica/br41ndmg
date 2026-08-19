//! Full-screen file info + spectrogram view.
//!
//! [`Viewer`] owns the scroll/zoom state for one file. The spectrogram itself
//! lives in a caller-owned cache; when it is absent the viewer shows a
//! loading spinner (the caller decodes on a background thread).

use crate::spectrogram::{BINS, DB_FLOOR, Spectrogram, color};
use crate::tui::spinner;
use br41ndmg::io::FileInfo;
use br41ndmg::tags::Tags;
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};
use std::path::PathBuf;
use std::time::Duration;

pub enum ViewAction {
    Continue,
    Back,
}

pub struct Viewer {
    pub path: PathBuf,
    pub info: Option<FileInfo>,
    pub info_error: Option<String>,
    pub tags: Tags,
    scroll: usize, // first visible grid column
    step: usize,   // grid columns per screen cell (1 = max zoom)
    // Max-pooled dB per screen cell, rebuilt only when the view parameters or
    // terminal size change.
    cache: Vec<f32>,
    cache_key: (usize, usize, u16, u16),
}

impl Viewer {
    pub fn new(path: PathBuf, info: Result<FileInfo, String>, tags: Tags) -> Self {
        let (info, info_error) = match info {
            Ok(info) => (Some(info), None),
            Err(error) => (None, Some(error)),
        };
        Viewer {
            path,
            info,
            info_error,
            tags,
            scroll: 0,
            step: usize::MAX, // fit whole file initially
            cache: Vec::new(),
            cache_key: (0, 0, 0, 0),
        }
    }

    pub fn handle_key(&mut self, code: KeyCode, cols: Option<usize>) -> ViewAction {
        let Some(cols) = cols.map(|c| c.max(1)) else {
            return match code {
                KeyCode::Esc | KeyCode::Char('q') => ViewAction::Back,
                _ => ViewAction::Continue,
            };
        };
        // usize::MAX is the "fit whole file" sentinel; pin it to a real step
        // once the column count is known.
        if self.step == usize::MAX {
            self.step = cols;
        }
        match code {
            KeyCode::Esc | KeyCode::Char('q') => return ViewAction::Back,
            KeyCode::Left | KeyCode::Char('h') => {
                let delta = self.page(cols) / 4;
                self.scroll = self.scroll.saturating_sub(delta.max(1));
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let delta = self.page(cols) / 4;
                self.scroll = (self.scroll + delta.max(1)).min(self.max_scroll(cols));
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.step = (self.step / 2).max(1);
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                self.step = self.step.saturating_mul(2).min(cols.max(1));
            }
            KeyCode::Char('0') => {
                self.scroll = 0;
                self.step = usize::MAX;
            }
            _ => {}
        }
        self.scroll = self.scroll.min(self.max_scroll(cols));
        ViewAction::Continue
    }

    /// Grid columns visible across a nominal 80-cell pane; used for scroll
    /// deltas before any draw fixes the real width.
    fn page(&self, cols: usize) -> usize {
        80_usize.saturating_mul(self.step).min(cols).max(1)
    }

    /// Aggregation clamped so a whole file always fits: `step` beyond
    /// `ceil(cols / width)` would only waste resolution.
    fn effective_step(&self, cols: usize, width: usize) -> usize {
        self.step.min(cols.div_ceil(width.max(1))).max(1)
    }

    fn max_scroll(&self, cols: usize) -> usize {
        cols.saturating_sub(1)
    }

    /// Max-pooled dB per screen cell for a `width x height` pane, cached.
    fn cells(&mut self, spectro: &Spectrogram, width: u16, height: u16) -> &[f32] {
        let key = (self.scroll, self.step, width, height);
        if self.cache_key != key {
            let cols = spectro.columns().max(1);
            let w = width.max(1) as usize;
            let h = height.max(1) as usize;
            let step = self.effective_step(cols, w);
            let mut cells = vec![DB_FLOOR; w * h];
            for x in 0..w {
                let col0 = self.scroll.saturating_add(x.saturating_mul(step));
                if col0 >= cols {
                    break;
                }
                let col1 = col0.saturating_add(step).min(cols);
                for y in 0..h {
                    // Top screen row = highest frequency band.
                    let bin_hi = BINS - (BINS * y / h);
                    let bin_lo = BINS - (BINS * (y + 1) / h);
                    let bin1 = bin_hi.max(bin_lo + 1).min(BINS);
                    cells[y * w + x] = spectro.max_db(col0..col1, bin_lo..bin1);
                }
            }
            self.cache = cells;
            self.cache_key = key;
        }
        &self.cache
    }

    pub fn draw(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        spectro: Option<&Spectrogram>,
        error: Option<&String>,
        loading: bool,
        elapsed: Duration,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(self.info_height()),
                Constraint::Min(3),
                Constraint::Length(1),
            ])
            .split(area);

        self.draw_info(frame, chunks[0]);
        if let Some(error) = error.filter(|_| !loading) {
            self.draw_error(frame, chunks[1], error);
        } else if let Some(spectro) = spectro.filter(|_| !loading) {
            self.draw_spectro(frame, chunks[1], chunks[2], spectro);
        } else {
            self.draw_loading(frame, chunks[1], elapsed);
        }
    }

    fn info_height(&self) -> u16 {
        2 + self.tags.len().min(8) as u16 + 2
    }

    fn draw_info(&self, frame: &mut Frame, area: Rect) {
        let mut lines = vec![
            Line::from(format!(
                " {}",
                self.path
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default()
            ))
            .style(Style::default().add_modifier(Modifier::BOLD)),
        ];

        if let Some(err) = &self.info_error {
            lines.push(Line::from(format!(" {err}")).style(Style::default().fg(Color::Red)));
        }
        if let Some(info) = &self.info {
            let duration = info
                .frames
                .map(|f| fmt_duration(f as f64 / info.sample_rate as f64));
            lines.push(Line::from(format!(
                " format {}   rate {} Hz   channels {}   bits {}{}",
                info.format,
                info.sample_rate,
                info.channels,
                info.bits_per_sample,
                duration
                    .map(|d| format!("   duration {d}"))
                    .unwrap_or_default(),
            )));
        }
        if let Ok(meta) = std::fs::metadata(&self.path) {
            lines.push(Line::from(format!(" size {}", fmt_size(meta.len()))));
        }

        for (name, value) in self.tags.iter().take(8) {
            let text = if value.chars().count() > 60 {
                format!(" {name}: {}…", value.chars().take(60).collect::<String>())
            } else {
                format!(" {name}: {value}")
            };
            lines.push(Line::from(text).style(Style::default().fg(Color::Gray)));
        }

        frame.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Info ")),
            area,
        );
    }

    fn draw_loading(&self, frame: &mut Frame, area: Rect, elapsed: Duration) {
        let text = format!(" {} decoding & analyzing…", spinner(elapsed));
        frame.render_widget(
            Paragraph::new(text)
                .style(Style::default().fg(Color::Cyan))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Spectrogram "),
                ),
            area,
        );
    }

    fn draw_error(&self, frame: &mut Frame, area: Rect, error: &str) {
        frame.render_widget(
            Paragraph::new(format!(" {error}"))
                .style(Style::default().fg(Color::Red))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Spectrogram "),
                ),
            area,
        );
    }

    fn draw_spectro(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        legend_area: Rect,
        spectro: &Spectrogram,
    ) {
        const AXIS_W: u16 = 6; // left frequency axis, last column is a gutter
        let view = inner(area);
        let cols = spectro.columns().max(1);

        // Plot minus the two axis strips; fall back to a plain block when the
        // terminal is too small to fit axes.
        if view.width < AXIS_W + 8 || view.height < 4 {
            let title = format!(
                " Spectrogram — {:.1}s–{:.1}s ",
                spectro.time_at(self.scroll),
                spectro.time_at(cols)
            );
            frame.render_widget(Block::default().borders(Borders::ALL).title(title), area);
            return;
        }

        let plot = Rect {
            x: view.x + AXIS_W,
            y: view.y,
            width: view.width - AXIS_W,
            height: view.height - 1,
        };
        let x_axis = Rect {
            x: plot.x,
            y: plot.y + plot.height,
            width: plot.width,
            height: 1,
        };

        let cols = spectro.columns().max(1);
        let w = plot.width.max(1) as usize;
        let scroll = self.scroll;
        let step = self.effective_step(cols, w);
        // Normalize the "fit" sentinel (and any over-zoom) so the next zoom
        // key halves/doubles the actually-displayed step.
        self.step = step;

        let t0 = spectro.time_at(scroll);
        let t1 = spectro.time_at((scroll + w * step).min(cols)).max(t0);
        let title = format!(" Spectrogram — {t0:.1}s–{t1:.1}s ");

        // Heat map.
        let cells = self.cells(spectro, plot.width, plot.height);
        let buf = frame.buffer_mut();
        for y in 0..plot.height {
            for x in 0..plot.width {
                let db = cells
                    .get(y as usize * plot.width as usize + x as usize)
                    .copied()
                    .unwrap_or(DB_FLOOR);
                if let Some(cell) = buf.cell_mut((plot.x + x, plot.y + y)) {
                    cell.set_symbol(" ")
                        .set_style(Style::default().bg(color(db)));
                }
            }
        }
        frame.render_widget(Block::default().borders(Borders::ALL).title(title), area);

        // X axis: time ticks every nice step across [t0, t1].
        let span = t1 - t0;
        let tick = tick_step(span, (w / 12).max(3));
        let mut xchars: Vec<char> = vec![' '; w];
        let mut t = (t0 / tick).ceil() * tick;
        while t <= t1 + f64::EPSILON {
            let label = fmt_time_tick(t, tick);
            let len = label.chars().count().min(w);
            let x = ((t - t0) / span * (w as f64 - 1.0)).round() as usize;
            let start = x.saturating_sub(len / 2).min(w - len);
            for (i, c) in label.chars().enumerate() {
                if start + i < w {
                    xchars[start + i] = c;
                }
            }
            t += tick;
        }
        let x_label: String = xchars.into_iter().collect();
        frame.render_widget(
            Paragraph::new(x_label).style(Style::default().fg(Color::DarkGray)),
            x_axis,
        );

        // Y axis: frequency ticks, 0 Hz at the bottom, Nyquist at the top.
        let nyquist = f64::from(spectro.sample_rate) / 2.0;
        let hz_tick = tick_step(nyquist, (plot.height as usize / 3).max(3));
        let mut ylines = vec![Line::from(""); plot.height as usize];
        let mut hz = 0.0;
        while hz <= nyquist + 1.0 {
            let frac = hz / nyquist;
            let row = ((plot.height - 1) as f64 - frac * (plot.height - 1) as f64).round() as i64;
            if row >= 0 && (row as usize) < ylines.len() {
                let label = fmt_hz_tick(hz);
                ylines[row as usize] =
                    Line::from(format!("{label:>width$}", width = (AXIS_W - 1) as usize));
            }
            hz += hz_tick;
        }
        let y_axis = Rect {
            x: view.x,
            y: view.y,
            width: AXIS_W - 1,
            height: plot.height,
        };
        frame.render_widget(
            Paragraph::new(ylines).style(Style::default().fg(Color::DarkGray)),
            y_axis,
        );

        let legend = Line::from(format!(
            " y: Hz   x: seconds   dB range {}…0    ←/→ scroll · +/- zoom · 0 fit · Esc back",
            crate::spectrogram::DB_FLOOR
        ));
        frame.render_widget(Paragraph::new(legend), legend_area);
    }
}

/// Nice tick step (1/2/5 × 10^k) aiming for ~`target` intervals in `span`.
fn tick_step(span: f64, target: usize) -> f64 {
    let raw = span / target.max(1) as f64;
    if raw <= 0.0 {
        return 1.0;
    }
    let mag = 10_f64.powf(raw.log10().floor());
    let norm = raw / mag;
    let nice = if norm <= 1.0 {
        1.0
    } else if norm <= 2.0 {
        2.0
    } else if norm <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice * mag
}

fn fmt_time_tick(t: f64, step: f64) -> String {
    if step >= 60.0 {
        fmt_duration(t)
    } else if step >= 1.0 {
        format!("{}s", t.round())
    } else {
        format!("{t:.1}s")
    }
}

fn fmt_hz_tick(hz: f64) -> String {
    if hz >= 1000.0 {
        format!("{:.0}k", hz / 1000.0)
    } else {
        format!("{hz:.0}")
    }
}

fn inner(area: Rect) -> Rect {
    Rect {
        x: area.x.saturating_add(1),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}

pub fn fmt_size(bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < units.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        let u = units[unit];
        format!("{value:.1} {u}")
    }
}

pub fn fmt_duration(secs: f64) -> String {
    let total = secs.round() as u64;
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_helpers() {
        assert_eq!(fmt_size(512), "512 B");
        assert_eq!(fmt_size(2 * 1024 * 1024), "2.0 MB");
        assert_eq!(fmt_duration(62.4), "1:02");
        assert_eq!(fmt_duration(3671.0), "1:01:11");
    }

    #[test]
    fn tick_steps_are_nice_numbers() {
        assert_eq!(tick_step(10.0, 5), 2.0);
        assert_eq!(tick_step(100.0, 4), 50.0);
        assert_eq!(tick_step(22_050.0, 10), 5000.0);
        assert_eq!(tick_step(0.0, 4), 1.0);
        assert_eq!(fmt_time_tick(4.0, 2.0), "4s");
        assert_eq!(fmt_time_tick(4.25, 0.5), "4.2s");
        assert_eq!(fmt_time_tick(120.0, 60.0), "2:00");
        assert_eq!(fmt_hz_tick(5000.0), "5k");
        assert_eq!(fmt_hz_tick(500.0), "500");
    }

    #[test]
    fn initial_step_fits_and_zoom_clamps() {
        let buffer = br41ndmg::io::AudioBuffer::new(16_000, 1, vec![0.0; 16_000]).unwrap();
        let cols = Spectrogram::compute(&buffer).columns().max(1);
        let mut viewer = Viewer::new(PathBuf::new(), Ok(fake_info()), Vec::new());

        // zoom out beyond fit stays clamped
        viewer.handle_key(KeyCode::Char('-'), Some(cols));
        assert_eq!(viewer.step, cols);
        // zoom in halves
        viewer.handle_key(KeyCode::Char('0'), Some(cols));
        viewer.handle_key(KeyCode::Char('+'), Some(cols));
        assert!(viewer.step <= cols / 2);
        // esc goes back
        assert!(matches!(
            viewer.handle_key(KeyCode::Esc, Some(cols)),
            ViewAction::Back
        ));
    }

    fn fake_info() -> FileInfo {
        FileInfo {
            format: "wav",
            sample_rate: 16_000,
            channels: 1,
            bits_per_sample: 16,
            frames: Some(16_000),
        }
    }
}
