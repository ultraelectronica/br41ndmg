use crate::resample_job::{BatchOutcome, auto_output_name, is_audio_file};
use br41ndmg::io::{read_audio, write_wav};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph},
};
use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

type Tui = Terminal<CrosstermBackend<io::Stdout>>;

/// Launch the interactive browser in `start_dir`.
pub fn run(start_dir: PathBuf) -> Result<(), String> {
    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        panic_hook(info);
    }));

    if let Err(error) = (|| -> io::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        run_app(&mut terminal, start_dir).map_err(io::Error::other)?;
        Ok(())
    })() {
        restore_terminal();
        return Err(error.to_string());
    }
    restore_terminal();
    Ok(())
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Browse,
    Input,
    PickRate,
    PickDir,
    Progress,
    Done,
}

const RATE_PRESETS: [u32; 12] = [
    8000, 11025, 16000, 22050, 24000, 32000, 44100, 48000, 88200, 96000, 176400, 192000,
];

#[derive(Clone, Debug)]
enum Entry {
    Dir(String, PathBuf),
    Audio(String, PathBuf),
}

impl Entry {
    fn name(&self) -> &str {
        match self {
            Entry::Dir(n, _) | Entry::Audio(n, _) => n,
        }
    }
}

struct Browser {
    dir: PathBuf,
    entries: Vec<Entry>,
    state: ListState,
}

impl Browser {
    fn open(dir: PathBuf) -> Self {
        let entries = read_entries(&dir);
        let mut state = ListState::default();
        state.select(if entries.is_empty() { None } else { Some(0) });
        Browser {
            dir,
            entries,
            state,
        }
    }

    fn reload(&mut self) {
        self.entries = read_entries(&self.dir);
        self.state.select(if self.entries.is_empty() {
            None
        } else {
            Some(0)
        });
    }

    fn selected(&self) -> Option<&Entry> {
        self.state.selected().and_then(|i| self.entries.get(i))
    }

    fn move_cursor(&mut self, delta: i32) {
        let len = self.entries.len();
        if len == 0 {
            return;
        }
        let current = self.state.selected().unwrap_or(0) as i32;
        let mut next = current + delta;
        if next < 0 {
            next = 0;
        }
        if next as usize >= len {
            next = (len - 1) as i32;
        }
        self.state.select(Some(next as usize));
    }

    fn descend(&mut self) {
        if let Some(Entry::Dir(_, path)) = self.selected() {
            self.dir = path.clone();
            self.reload();
        }
    }

    fn go_up(&mut self) {
        if let Some(parent) = self.dir.parent() {
            self.dir = parent.to_path_buf();
            self.reload();
        }
    }
}

fn read_entries(dir: &Path) -> Vec<Entry> {
    let mut dirs: Vec<Entry> = Vec::new();
    let mut files: Vec<Entry> = Vec::new();
    if let Ok(it) = std::fs::read_dir(dir) {
        for entry in it.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                dirs.push(Entry::Dir(name, path));
            } else if path.is_file() && is_audio_file(&path) {
                files.push(Entry::Audio(name, path));
            }
        }
    }
    dirs.sort_by(|a, b| a.name().cmp(b.name()));
    files.sort_by(|a, b| a.name().cmp(b.name()));

    let mut out = Vec::new();
    // ".." sentinel to move up one level, only when a parent exists.
    if dir.parent().is_some() {
        out.push(Entry::Dir("..".into(), dir.parent().unwrap().to_path_buf()));
    }
    out.extend(dirs);
    out.extend(files);
    out
}

fn read_subdirs(dir: &Path) -> Vec<Entry> {
    let mut dirs: Vec<Entry> = Vec::new();
    if let Ok(it) = std::fs::read_dir(dir) {
        for entry in it.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let name = entry.file_name().to_string_lossy().into_owned();
                dirs.push(Entry::Dir(name, entry.path()));
            }
        }
    }
    dirs.sort_by(|a, b| a.name().cmp(b.name()));
    dirs
}

fn marker(focused: bool) -> &'static str {
    if focused { "▶" } else { " " }
}

#[derive(Clone, Copy)]
enum Phase {
    Decode,
    Resample,
    Write,
}

enum Msg {
    Phase {
        index: usize,
        total: usize,
        phase: Phase,
        path: PathBuf,
    },
    Finished(BatchOutcome),
}

struct App {
    mode: Mode,
    browser: Browser,
    selected: BTreeSet<PathBuf>,
    rate_input: String,
    dir_input: String,
    input_focus: usize,
    input_error: Option<String>,
    rate_list_state: ListState,
    rate_custom: String,
    rate_custom_focused: bool,
    dir_picker_current: PathBuf,
    dir_picker_entries: Vec<Entry>,
    dir_picker_state: ListState,
    progress_index: usize,
    progress_total: usize,
    progress_current: PathBuf,
    progress_phase: Phase,
    progress_start: Option<Instant>,
    progress_outcome: Option<BatchOutcome>,
    progress_rx: Option<mpsc::Receiver<Msg>>,
}

impl App {
    fn new(start_dir: PathBuf) -> Self {
        App {
            mode: Mode::Browse,
            browser: Browser::open(start_dir),
            selected: BTreeSet::new(),
            rate_input: "48000".into(),
            dir_input: "./resampled".into(),
            input_focus: 0,
            input_error: None,
            rate_list_state: ListState::default(),
            rate_custom: String::new(),
            rate_custom_focused: false,
            dir_picker_current: PathBuf::new(),
            dir_picker_entries: Vec::new(),
            dir_picker_state: ListState::default(),
            progress_index: 0,
            progress_total: 0,
            progress_current: PathBuf::new(),
            progress_phase: Phase::Decode,
            progress_start: None,
            progress_outcome: None,
            progress_rx: None,
        }
    }

    fn toggle_current(&mut self) {
        if let Some(Entry::Audio(_, path)) = self.browser.selected() {
            if !self.selected.insert(path.clone()) {
                self.selected.remove(path);
            }
        }
    }

    fn select_all_in_dir(&mut self) {
        for entry in &self.browser.entries {
            if let Entry::Audio(_, path) = entry {
                self.selected.insert(path.clone());
            }
        }
    }

    fn handle_browse(&mut self, code: KeyCode) -> Action {
        match code {
            KeyCode::Down | KeyCode::Char('j') => self.browser.move_cursor(1),
            KeyCode::Up | KeyCode::Char('k') => self.browser.move_cursor(-1),
            KeyCode::Enter => {
                if matches!(self.browser.selected(), Some(Entry::Dir(_, _))) {
                    self.browser.descend();
                } else {
                    self.toggle_current();
                }
            }
            KeyCode::Char(' ') => self.toggle_current(),
            KeyCode::Char('u') => self.browser.go_up(),
            KeyCode::Char('a') => self.select_all_in_dir(),
            KeyCode::Char('c') => self.selected.clear(),
            KeyCode::Char('p') if !self.selected.is_empty() => {
                self.input_error = None;
                self.mode = Mode::Input;
            }
            KeyCode::Esc | KeyCode::Char('q') => return Action::Quit,
            _ => {}
        }
        Action::Continue
    }

    fn handle_input(&mut self, code: KeyCode) -> Action {
        match code {
            KeyCode::Tab | KeyCode::Down => self.input_focus = (self.input_focus + 1) % 3,
            KeyCode::Up => self.input_focus = (self.input_focus + 2) % 3,
            KeyCode::Esc => self.mode = Mode::Browse,
            KeyCode::Enter => match self.input_focus {
                0 => self.open_rate_picker(),
                1 => self.open_dir_picker(),
                _ => self.confirm(),
            },
            _ => {}
        }
        Action::Continue
    }

    fn open_rate_picker(&mut self) {
        self.input_error = None;
        self.rate_custom = self.rate_input.clone();
        self.rate_custom_focused = false;
        let current: Option<u32> = self.rate_input.parse().ok();
        let select = RATE_PRESETS
            .iter()
            .position(|&r| Some(r) == current)
            .unwrap_or(RATE_PRESETS.len());
        self.rate_list_state.select(Some(select));
        self.mode = Mode::PickRate;
    }

    fn handle_pick_rate(&mut self, code: KeyCode) -> Action {
        if self.rate_custom_focused {
            match code {
                KeyCode::Esc => self.rate_custom_focused = false,
                KeyCode::Enter => {
                    if let Ok(rate) = self.rate_custom.trim().parse::<u32>() {
                        if rate != 0 {
                            self.rate_input = rate.to_string();
                            self.mode = Mode::Input;
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.rate_custom.pop();
                }
                KeyCode::Char(c) if c.is_ascii_digit() => self.rate_custom.push(c),
                _ => {}
            }
            return Action::Continue;
        }

        let len = RATE_PRESETS.len() + 1;
        match code {
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self.rate_list_state.selected().unwrap_or(0);
                self.rate_list_state.select(Some((i + 1) % len));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.rate_list_state.selected().unwrap_or(0);
                self.rate_list_state.select(Some((i + len - 1) % len));
            }
            KeyCode::Enter => {
                let i = self.rate_list_state.selected().unwrap_or(0);
                if i < RATE_PRESETS.len() {
                    self.rate_input = RATE_PRESETS[i].to_string();
                    self.mode = Mode::Input;
                } else {
                    self.rate_custom_focused = true;
                }
            }
            KeyCode::Esc => self.mode = Mode::Input,
            _ => {}
        }
        Action::Continue
    }

    fn open_dir_picker(&mut self) {
        self.input_error = None;
        self.dir_picker_current = if self.dir_input.trim().is_empty() {
            self.browser.dir.clone()
        } else {
            PathBuf::from(&self.dir_input)
        };
        self.reload_dir_picker();
        self.mode = Mode::PickDir;
    }

    fn reload_dir_picker(&mut self) {
        self.dir_picker_entries = read_subdirs(&self.dir_picker_current);
        self.dir_picker_state
            .select(if self.dir_picker_entries.is_empty() {
                None
            } else {
                Some(0)
            });
    }

    fn handle_pick_dir(&mut self, code: KeyCode) -> Action {
        let len = self.dir_picker_entries.len();
        match code {
            KeyCode::Down | KeyCode::Char('j') if len > 0 => {
                let i = self.dir_picker_state.selected().unwrap_or(0);
                self.dir_picker_state.select(Some((i + 1) % len));
            }
            KeyCode::Up | KeyCode::Char('k') if len > 0 => {
                let i = self.dir_picker_state.selected().unwrap_or(0);
                self.dir_picker_state.select(Some((i + len - 1) % len));
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                let target = self
                    .dir_picker_state
                    .selected()
                    .and_then(|i| self.dir_picker_entries.get(i))
                    .and_then(|e| match e {
                        Entry::Dir(_, p) => Some(p.clone()),
                        _ => None,
                    });
                if let Some(path) = target {
                    self.dir_picker_current = path;
                    self.reload_dir_picker();
                }
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace | KeyCode::Char('u') => {
                if let Some(parent) = self.dir_picker_current.parent() {
                    self.dir_picker_current = parent.to_path_buf();
                    self.reload_dir_picker();
                }
            }
            KeyCode::Char(' ') | KeyCode::Char('m') => {
                self.dir_input = self.dir_picker_current.display().to_string();
                self.mode = Mode::Input;
            }
            KeyCode::Esc => self.mode = Mode::Input,
            _ => {}
        }
        Action::Continue
    }

    fn confirm(&mut self) {
        let rate: u32 = match self.rate_input.parse() {
            Ok(0) | Err(_) => {
                self.input_error = Some("sample rate must be a positive integer".into());
                return;
            }
            Ok(r) => r,
        };
        if self.dir_input.trim().is_empty() {
            self.input_error = Some("output directory is required".into());
            return;
        }

        let inputs: Vec<PathBuf> = self.selected.iter().cloned().collect();
        let output_dir = PathBuf::from(&self.dir_input);
        let total = inputs.len();
        let current = inputs.first().cloned().unwrap_or_default();
        self.progress_total = total;
        self.progress_index = 0;
        self.progress_current = current;
        self.progress_phase = Phase::Decode;
        self.progress_start = Some(Instant::now());
        self.progress_outcome = None;

        let (tx, rx) = mpsc::channel();
        self.progress_rx = Some(rx);
        self.mode = Mode::Progress;

        // ponytail: per-file work on a background thread. We split each file
        // into decode/resample/write phases so the UI can report what is
        // happening (and the spinner proves liveness) even on one huge FLAC.
        // No mid-file cancellation — add a shared AtomicBool to stop cleanly.
        std::thread::spawn(move || {
            let _ = std::fs::create_dir_all(&output_dir);
            let total = inputs.len();
            let mut ok = 0usize;
            let mut failed: Vec<(PathBuf, String)> = Vec::new();

            for (index, input) in inputs.iter().enumerate() {
                let _ = tx.send(Msg::Phase {
                    index,
                    total,
                    phase: Phase::Decode,
                    path: input.clone(),
                });
                let buf = match read_audio(input) {
                    Ok(b) => b,
                    Err(e) => {
                        failed.push((input.clone(), e.to_string()));
                        continue;
                    }
                };

                let _ = tx.send(Msg::Phase {
                    index,
                    total,
                    phase: Phase::Resample,
                    path: input.clone(),
                });
                let result = match buf.resample_to(rate) {
                    Ok(r) => r,
                    Err(e) => {
                        failed.push((input.clone(), e.to_string()));
                        continue;
                    }
                };

                let out = output_dir.join(auto_output_name(input, rate));
                let _ = tx.send(Msg::Phase {
                    index,
                    total,
                    phase: Phase::Write,
                    path: out.clone(),
                });
                match write_wav(&out, &result) {
                    Ok(()) => ok += 1,
                    Err(e) => failed.push((input.clone(), e.to_string())),
                }
            }

            let _ = tx.send(Msg::Finished(BatchOutcome { ok, failed }));
        });
    }

    fn drain_progress(&mut self) {
        loop {
            let msg = match &self.progress_rx {
                Some(rx) => match rx.try_recv() {
                    Ok(m) => m,
                    Err(_) => break,
                },
                None => break,
            };
            match msg {
                Msg::Phase {
                    index,
                    total,
                    phase,
                    path,
                } => {
                    self.progress_index = index;
                    self.progress_total = total;
                    self.progress_phase = phase;
                    self.progress_current = path;
                }
                Msg::Finished(outcome) => {
                    self.progress_outcome = Some(outcome);
                    self.progress_rx = None;
                    self.mode = Mode::Done;
                    break;
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Action {
    Continue,
    Quit,
}

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn spinner(elapsed: Duration) -> &'static str {
    SPINNER[(elapsed.as_millis() / 80) as usize % SPINNER.len()]
}

fn phase_label(phase: Phase) -> &'static str {
    match phase {
        Phase::Decode => "decoding",
        Phase::Resample => "resampling",
        Phase::Write => "writing",
    }
}

fn phase_fraction(phase: Phase) -> f64 {
    match phase {
        Phase::Decode => 0.1,
        Phase::Resample => 0.55,
        Phase::Write => 0.9,
    }
}

fn run_app(terminal: &mut Tui, start_dir: PathBuf) -> Result<(), String> {
    let mut app = App::new(start_dir);
    loop {
        terminal
            .draw(|frame| app.draw(frame))
            .map_err(|e| e.to_string())?;

        if app.mode == Mode::Progress {
            app.drain_progress();
        }

        if event::poll(Duration::from_millis(50)).map_err(|e| e.to_string())? {
            if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                let action = match app.mode {
                    Mode::Browse => app.handle_browse(key.code),
                    Mode::Input => app.handle_input(key.code),
                    Mode::PickRate => app.handle_pick_rate(key.code),
                    Mode::PickDir => app.handle_pick_dir(key.code),
                    Mode::Progress => Action::Continue,
                    Mode::Done => Action::Quit,
                };
                if matches!(action, Action::Quit) {
                    return Ok(());
                }
            }
        }
    }
}

impl App {
    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(area);

        let header = format!(
            " br41ndmg resampler   |   {} file(s) selected",
            self.selected.len()
        );
        frame.render_widget(
            Paragraph::new(header).style(Style::default().add_modifier(Modifier::BOLD)),
            chunks[0],
        );
        frame.render_widget(
            Paragraph::new(self.footer())
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray)),
            chunks[2],
        );

        match self.mode {
            Mode::Browse => self.draw_browser(frame, chunks[1]),
            Mode::Input => self.draw_input(frame, chunks[1]),
            Mode::PickRate => self.draw_pick_rate(frame, chunks[1]),
            Mode::PickDir => self.draw_pick_dir(frame, chunks[1]),
            Mode::Progress => self.draw_progress(frame, chunks[1]),
            Mode::Done => self.draw_done(frame, chunks[1]),
        }
    }

    fn footer(&self) -> String {
        match self.mode {
            Mode::Browse => {
                "↑/↓ move  Enter open/toggle  Space toggle  u up  a all  c clear  p process  q quit"
            }
            Mode::Input => "Tab/↑↓ move  Enter change  Esc back",
            Mode::PickRate => "↑/↓ choose  Enter select  Esc back",
            Mode::PickDir => "↑/↓ move  Enter/→ open  ←/u up  m/Space use  Esc cancel",
            Mode::Progress => "resampling...",
            Mode::Done => "press any key to exit",
        }
        .into()
    }

    fn draw_browser(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .browser
            .entries
            .iter()
            .map(|entry| match entry {
                Entry::Dir(name, _) => ListItem::new(Line::from(format!("  /   {name}"))),
                Entry::Audio(name, path) => {
                    let mark = if self.selected.contains(path) {
                        "[x]"
                    } else {
                        "[ ]"
                    };
                    ListItem::new(Line::from(format!("  {mark}  {name}")))
                }
            })
            .collect();

        let title = format!(" {} ", self.browser.dir.display());
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        let mut state = self.browser.state.clone();
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn draw_input(&self, frame: &mut Frame, area: Rect) {
        let focus_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(""));
        lines.push(
            Line::from(format!(
                " {} Sample rate ……… {} Hz",
                marker(self.input_focus == 0),
                self.rate_input
            ))
            .style(if self.input_focus == 0 {
                focus_style
            } else {
                Style::default()
            }),
        );
        lines.push(Line::from(""));
        lines.push(
            Line::from(format!(
                " {} Output dir ……… {}",
                marker(self.input_focus == 1),
                self.dir_input
            ))
            .style(if self.input_focus == 1 {
                focus_style
            } else {
                Style::default()
            }),
        );
        lines.push(Line::from(""));
        lines.push(Line::from(""));
        let start = if self.input_focus == 2 {
            "▶  Start resampling  ◀"
        } else {
            "   Start resampling   "
        };
        lines.push(
            Line::from(format!("        {start}")).style(if self.input_focus == 2 {
                focus_style
            } else {
                Style::default()
            }),
        );
        lines.push(Line::from(""));
        if let Some(err) = &self.input_error {
            lines.push(Line::from(format!(" {err}")).style(Style::default().fg(Color::Red)));
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Settings — Tab/↑↓ to move, Enter to change, Esc to go back ");
        frame.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn draw_pick_rate(&self, frame: &mut Frame, area: Rect) {
        let current: Option<u32> = self.rate_input.parse().ok();
        let mut items: Vec<ListItem> = RATE_PRESETS
            .iter()
            .map(|&rate| {
                let mark = if Some(rate) == current { "●" } else { " " };
                ListItem::new(Line::from(format!("  {mark}  {rate} Hz")))
            })
            .collect();
        let custom_mark = if current.is_some_and(|c| !RATE_PRESETS.contains(&c)) {
            "●"
        } else {
            " "
        };
        items.push(ListItem::new(Line::from(format!(
            "  {custom_mark}  Custom…"
        ))));

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Sample rate "),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        let constraints = if self.rate_custom_focused {
            vec![Constraint::Min(1), Constraint::Length(3)]
        } else {
            vec![Constraint::Min(1)]
        };
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);
        let mut state = self.rate_list_state.clone();
        frame.render_stateful_widget(list, chunks[0], &mut state);

        if self.rate_custom_focused {
            let field = format!(" Custom rate: {}_", self.rate_custom);
            let paragraph = Paragraph::new(field)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Type a rate (digits only), Enter to use, Esc back "),
                )
                .style(Style::default().fg(Color::Cyan));
            frame.render_widget(paragraph, chunks[1]);
        }
    }

    fn draw_pick_dir(&self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .dir_picker_entries
            .iter()
            .map(|entry| match entry {
                Entry::Dir(name, _) => ListItem::new(Line::from(format!("  /  {name}/"))),
                _ => ListItem::new(Line::from("")),
            })
            .collect();

        let title = format!(
            " Output: {}  —  Enter/→ open · ←/u up · m/Space use · Esc cancel ",
            self.dir_picker_current.display()
        );
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title))
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");
        let mut state = self.dir_picker_state.clone();
        frame.render_stateful_widget(list, area, &mut state);
    }

    fn draw_progress(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(1)])
            .split(area);

        let elapsed = self.progress_start.map(|t| t.elapsed()).unwrap_or_default();
        let status = format!(
            " {} {}…  {:02}:{:02} elapsed",
            spinner(elapsed),
            phase_label(self.progress_phase),
            elapsed.as_secs() / 60,
            elapsed.as_secs() % 60,
        );
        frame.render_widget(
            Paragraph::new(status).style(Style::default().fg(Color::Cyan)),
            chunks[0],
        );

        let total = self.progress_total.max(1);
        let ratio = ((self.progress_index as f64 + phase_fraction(self.progress_phase))
            / total as f64)
            .clamp(0.0, 1.0);
        let title = format!(
            " file {}/{} — {} ",
            self.progress_index + 1,
            self.progress_total,
            self.progress_current.display(),
        );
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(title))
            .gauge_style(Style::default().fg(Color::Cyan))
            .ratio(ratio);
        frame.render_widget(gauge, chunks[1]);
    }

    fn draw_done(&self, frame: &mut Frame, area: Rect) {
        let (ok, failed) = match &self.progress_outcome {
            Some(o) => (o.ok, o.failed.len()),
            None => (0, 0),
        };
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(format!(
            " Finished: {ok} succeeded, {failed} failed"
        )));
        lines.push(Line::from(""));
        if let Some(outcome) = &self.progress_outcome {
            for (path, error) in &outcome.failed {
                lines.push(Line::from(format!(
                    "  failed: {} — {error}",
                    path.display()
                )));
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(" press any key to exit"));

        let block = Block::default().borders(Borders::ALL).title(" Summary ");
        frame.render_widget(Paragraph::new(lines).block(block), area);
    }
}
