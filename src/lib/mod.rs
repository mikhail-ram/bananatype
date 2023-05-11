use crossterm::event::{poll, read, Event, KeyCode};
use rand::{distributions::Uniform, Rng};
use std::io::{self, BufRead, Stdout};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use termion::raw::{IntoRawMode, RawTerminal};
use tui::backend::CrosstermBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans, Text};
use tui::symbols;
use tui::widgets::{Block, BorderType, Borders, Gauge, Paragraph, Wrap, Dataset, Chart, Axis, GraphType};
use tui::Terminal;
use std::iter;

mod score;

type CrosstermTerminal = Terminal<CrosstermBackend<RawTerminal<Stdout>>>;

const NUMBER_OF_WORDS: usize = 100;
const TEST_DURATION: f64 = 30.0;
const TIMER_REFRESH_RATE: f64 = 2.0;
const BANANATYPE: &str = r"
 /$$                                                           /$$                                  
| $$                                                          | $$                                  
| $$$$$$$   /$$$$$$  /$$$$$$$   /$$$$$$  /$$$$$$$   /$$$$$$  /$$$$$$   /$$   /$$  /$$$$$$   /$$$$$$ 
| $$__  $$ |____  $$| $$__  $$ |____  $$| $$__  $$ |____  $$|_  $$_/  | $$  | $$ /$$__  $$ /$$__  $$
| $$  \ $$  /$$$$$$$| $$  \ $$  /$$$$$$$| $$  \ $$  /$$$$$$$  | $$    | $$  | $$| $$  \ $$| $$$$$$$$
| $$  | $$ /$$__  $$| $$  | $$ /$$__  $$| $$  | $$ /$$__  $$  | $$ /$$| $$  | $$| $$  | $$| $$_____/
| $$$$$$$/|  $$$$$$$| $$  | $$|  $$$$$$$| $$  | $$|  $$$$$$$  |  $$$$/|  $$$$$$$| $$$$$$$/|  $$$$$$$
|_______/  \_______/|__/  |__/ \_______/|__/  |__/ \_______/   \___/   \____  $$| $$____/  \_______/
                                                                       /$$  | $$| $$                
                                                                      |  $$$$$$/| $$                
                                                                       \______/ |__/                

";

// TODO: draw line graph for raw wpm and net wpm
// TODO: separate styling from mechanics of the test
struct Theme {
    fg: Color,
    bg: Color,
    highlight: Color,
    cursor: Color,
    correct: Color,
    incorrect: Color,
}

impl Theme {
    fn new () -> Theme {
        Theme {
            fg: Color::DarkGray,
            bg: Color::Reset,
            highlight: Color::Yellow,
            cursor: Color::Gray,
            correct: Color::Green,
            incorrect: Color::Red,
        }
    }
}

struct Settings {}

struct Log {
    time: Vec<f64>,
    net_wpm: Vec<f64>,
    gross_wpm: Vec<f64>,
}

impl Log {
    fn new() -> Log {
        Log {
            time: Vec::new(),
            net_wpm: Vec::new(),
            gross_wpm: Vec::new(),
        }
    }

    fn update(&mut self, time: f64, net_wpm: f64, gross_wpm: f64) {
        if self.time.len() == 0 || self.time[self.time.len() - 1] < time {
            self.time.push(time);
            self.net_wpm.push(net_wpm);
            self.gross_wpm.push(gross_wpm);
        }
    }
}

pub struct TypingTest<'a> {
    text: Vec<Span<'a>>,
    position: usize,
    terminal: CrosstermTerminal,
    elapsed_seconds: f64,
    score: score::Score,
    theme: Theme,
    log: Log,
}

impl TypingTest<'_> {
    pub fn new() -> TypingTest<'static> {
        //let text = TypingTest::generate_text();
        let terminal = TypingTest::setup_terminal().unwrap();

        let mut typing_test = TypingTest {
                             text: Vec::new(),
                             terminal,
                             position: 0,
                             elapsed_seconds: 0.0,
                             score: score::Score::new(),
                             theme: Theme::new(),
                             log: Log::new()
                         };

        typing_test.generate_text();
        typing_test
    }

    fn generate_text(&mut self) {
        let file = include_str!("words.txt");
        let file: Vec<String> = file
            .lines()
            .map(|line| line.to_string())
            .collect();

        let rand_nums: Vec<usize> = rand::thread_rng()
            .sample_iter(Uniform::from(0..file.len()))
            .take(NUMBER_OF_WORDS)
            .collect();

        let mut spans = Vec::new();

        for num in rand_nums {
            spans.extend([
                Span::styled(file[num].clone(),
                             Style::default().fg(self.theme.fg)),
                Span::styled(" ", Style::default().fg(self.theme.fg)),
            ]);
        }

        self.text = spans;
    }

    fn update_char(&mut self, character: char) {
        let current_word = &self.text[self.position].content;
        let next_word = &self.text[self.position + 1].content;

        let (former, latter) = current_word.split_at(1);
        let former = former.chars().next().unwrap();

        let former = match former {
            former if former == character => {
                self.score.calculate_correct();
                Span::styled(former.to_string(), Style::default().fg(self.theme.correct))
            }
            former if former == ' ' => {
                self.score.calculate_incorrect();
                Span::styled(' '.to_string(), Style::default().bg(self.theme.incorrect))
            }
            _ => {
                self.score.calculate_incorrect();
                Span::styled(former.to_string(), Style::default().fg(self.theme.incorrect))
            }
        };

        if latter.len() > 1 {
            let (cursor, latter) = latter.split_at(1);

            let cursor = Span::styled(cursor.to_string(),
                                      Style::default().fg(self.theme.cursor).bg(self.theme.fg));

            let latter = Span::styled(latter.to_string(),
                                      Style::default().fg(self.theme.fg));

            self.text
                .splice(self.position..self.position + 1, [former, cursor, latter]);
        } else if latter.len() == 1 {
            let cursor = Span::styled(latter.to_string(),
                                      Style::default().fg(self.theme.cursor).bg(self.theme.fg));

            self.text
                .splice(self.position..self.position + 1, [former, cursor]);
        } else if next_word.len() > 1 {
            let (cursor, latter) = next_word.split_at(1);

            let cursor = Span::styled(cursor.to_string(),
                                      Style::default().fg(self.theme.cursor).bg(self.theme.fg));

            let latter = Span::styled(latter.to_string(),
                                      Style::default().fg(self.theme.fg));

            self.text
                .splice(self.position..self.position + 2, [former, cursor, latter]);
        } else if next_word.len() == 1 {
            let cursor = Span::styled(next_word.to_string(),
                                      Style::default().fg(self.theme.cursor).bg(self.theme.fg));

            self.text
                .splice(self.position..self.position + 2, [former, cursor]);
        }

        self.position += 1;
        self.refresh();
    }

    fn backspace(&mut self) {
        if self.position > 0 {
            if (self.text[self.position - 1].content == " " && self.text[self.position - 1].style.bg == Some(self.theme.bg))
                || self.text[self.position - 1].style.fg == Some(self.theme.correct) {
                self.score.calculate_correct_backspace();
            } else {
                self.score.calculate_incorrect_backspace();
            }
            self.text[self.position].style = Style::default().fg(self.theme.fg).bg(self.theme.bg);
            self.position = self.position - 1;
            self.text[self.position].style = Style::default().fg(self.theme.cursor).bg(self.theme.fg);
        }
        self.refresh();
    }

    fn refresh(&mut self) -> Result<(), io::Error> {
        let net_wpm = self.score.calculate_net_wpm(self.elapsed_seconds);
        let gross_wpm = self.score.calculate_gross_wpm(self.elapsed_seconds);

        if self.elapsed_seconds % ((1.0 / TIMER_REFRESH_RATE) * 2.0) == 0.0 {
            self.log.update(self.elapsed_seconds, net_wpm, gross_wpm);
        }

        let time_block = Block::default()
            .title(Span::styled(
                "Time",
                Style::default().add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(self.theme.fg))
            .borders(Borders::ALL)
            .border_type(BorderType::Thick);

        let progress = Gauge::default()
            .block(time_block)
            .gauge_style(Style::default().fg(self.theme.fg))
            .ratio(self.elapsed_seconds / TEST_DURATION)
            .label(format!(
                "{}",
                TEST_DURATION.round() as u8 - (self.elapsed_seconds).round() as u8
            ));

        let gross_wpm_block = Block::default()
            .title(Span::styled(
                "Gross WPM",
                Style::default()
                    .fg(self.theme.fg)
                    .add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(self.theme.fg))
            .borders(Borders::ALL)
            .border_type(BorderType::Thick);

        let gross_wpm = Paragraph::new(Span::styled(
            format!(
                "{:.1}",
                gross_wpm
            ),
            Style::default()
                .fg(self.theme.fg)
                .add_modifier(Modifier::BOLD),
        ))
        .block(gross_wpm_block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

        let net_wpm_block = Block::default()
            .title(Span::styled(
                "Net WPM",
                Style::default()
                    .fg(self.theme.fg)
                    .add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(self.theme.fg))
            .borders(Borders::ALL)
            .border_type(BorderType::Thick);

        let net_wpm = Paragraph::new(Span::styled(
            format!("{:.1}", net_wpm),
            Style::default()
                .fg(self.theme.fg)
                .add_modifier(Modifier::BOLD),
        ))
        .block(net_wpm_block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

        let accuracy_block = Block::default()
            .title(Span::styled(
                "Accuracy",
                Style::default()
                    .fg(self.theme.fg)
                    .add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(self.theme.fg))
            .borders(Borders::ALL)
            .border_type(BorderType::Thick);

        let accuracy = Paragraph::new(Span::styled(
            format!("{:.1}", self.score.calculate_accuracy()),
            Style::default()
                .fg(self.theme.fg)
                .add_modifier(Modifier::BOLD),
        ))
        .block(accuracy_block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

        let input_block = Block::default()
            .title(Span::styled(
                "BananaType",
                Style::default()
                    .fg(self.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(self.theme.highlight))
            .borders(Borders::ALL)
            .border_type(BorderType::Thick);

        let text = Paragraph::new(Spans::from(self.text.clone()))
            .block(input_block)
            .wrap(Wrap { trim: true });

        self.terminal.draw(|frame| {
            let size = frame.size();
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Min(8),
                    ]
                    .as_ref(),
                )
                .split(size);

            let live_stats_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Ratio(1, 3),
                        Constraint::Ratio(1, 3),
                        Constraint::Ratio(1, 3),
                    ]
                    .as_ref(),
                )
                .split(layout[1]);

            let text_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Ratio(1, 1),
                    ]
                    .as_ref(),
                )
                .split(layout[2]);

            frame.render_widget(progress, layout[0]);
            frame.render_widget(gross_wpm, live_stats_layout[0]);
            frame.render_widget(net_wpm, live_stats_layout[1]);
            frame.render_widget(accuracy, live_stats_layout[2]);
            frame.render_widget(text, text_layout[0]);
            //frame.render_widget(text, layout[0]);
        })?;

        Ok(())
    }

    fn setup_terminal() -> Result<CrosstermTerminal, io::Error> {
        let stdout = io::stdout().into_raw_mode()?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;
        Ok(terminal)
    }

    fn cleanup_terminal(&mut self) {
        self.terminal.clear();
        self.terminal.set_cursor(0, 0);
    }

    fn start_timer() -> mpsc::Receiver<()> {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            for _ in 1..=(TEST_DURATION * TIMER_REFRESH_RATE).round() as u64 {
                tx.send(()).unwrap();
                thread::sleep(Duration::from_millis(
                    ((1.0 / TIMER_REFRESH_RATE) * 1000.0).round() as u64,
                ));
            }
        });
        rx
    }

    fn reset(&mut self) {
        self.generate_text();
        self.score = score::Score::new();
        self.log = Log::new();
        self.position = 0;
        self.elapsed_seconds = 0.0;
    }

    fn show_results(&mut self) -> Result<(), io::Error> {
        let results_block = Block::default()
            .title(Span::styled(
                "Your Results",
                Style::default()
                    .fg(self.theme.highlight)
                    .add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(self.theme.highlight))
            .borders(Borders::ALL)
            .border_type(BorderType::Thick);

        let results = Paragraph::new(Text::from(vec![
            Spans::from(vec![
                Span::raw("Gross WPM: "),
                Span::styled(
                    format!(
                        "{:.1}",
                        self.score.calculate_gross_wpm(self.elapsed_seconds)
                    ),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
            Spans::from(vec![
                Span::raw("Net WPM: "),
                Span::styled(
                    format!(
                        "{:.1}",
                        self.score.calculate_net_wpm(self.elapsed_seconds)
                    ),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
            Spans::from(vec![
                Span::raw("Accuracy: "),
                Span::styled(
                    format!(
                        "{:.1}",
                        self.score.calculate_accuracy()
                    ),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
            Spans::from(vec![
                Span::raw("Press "),
                Span::styled("r", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to restart or "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to quit."),
            ]),
            Spans::from(vec![
                Span::raw("Note: Press "),
                Span::styled("tab", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" during a test to quick restart."),
            ]),
        ]))
        .block(results_block)
        .wrap(Wrap { trim: true });

        let net_wpm_dataset: Vec<(f64, f64)> = iter::zip(self.log.time.clone(), self.log.net_wpm.clone()).collect();
        let gross_wpm_dataset: Vec<(f64, f64)> = iter::zip(self.log.time.clone(), self.log.gross_wpm.clone()).collect();

        let datasets = vec![
            Dataset::default()
                .name("net")
                .marker(symbols::Marker::Block)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Cyan))
                .data(&net_wpm_dataset),
            Dataset::default()
                .name("gross")
                .marker(symbols::Marker::Block)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Magenta))
                .data(&gross_wpm_dataset),
        ];

        let time_labels = ["0", &format!("{:.0}", TEST_DURATION / 2.0), &format!("{:.0}", TEST_DURATION)];
        let max_gross_wpm = *self.log.gross_wpm.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap() + 10.0;
        let wpm_labels = ["0", &format!("{:.0}", max_gross_wpm / 2.0), &format!("{:.0}", max_gross_wpm)];

        let chart = Chart::new(datasets)
            .block(Block::default()
                   .border_style(Style::default().fg(self.theme.highlight))
                   .borders(Borders::ALL)
                   .border_type(BorderType::Thick))
            .x_axis(Axis::default()
                .title(Span::styled("Time", Style::default().fg(self.theme.fg)))
                .style(Style::default().fg(self.theme.highlight))
                .bounds([0.0, TEST_DURATION])
                .labels(time_labels.iter().cloned().map(Span::from).collect()))
            .y_axis(Axis::default()
                .title(Span::styled("Words per Minute", Style::default().fg(self.theme.fg)))
                .style(Style::default().fg(self.theme.highlight))
                .bounds([0.0, max_gross_wpm])
                .labels(wpm_labels.iter().cloned().map(Span::from).collect()));


        self.terminal.draw(|frame| {
            let size = frame.size();
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Ratio(1, 4),
                        Constraint::Ratio(3, 4),
                        Constraint::Min(15),
                    ]
                    .as_ref(),
                )
                .split(size);

            let chart_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Ratio(1, 1),
                    ]
                    .as_ref(),
                )
                .split(layout[1]);

            frame.render_widget(results, layout[0]);
            frame.render_widget(chart, chart_layout[0]);
        })?;

        let mut restart = false;

        loop {
            if poll(Duration::from_millis(((1.0 / TIMER_REFRESH_RATE) * 1000.0).round() as u64)).unwrap() {
                if let Event::Key(event) = read().unwrap() {
                    match event.code {
                        KeyCode::Esc => {
                            self.cleanup_terminal();
                            break;
                        }
                        KeyCode::Char(c) => match c {
                            'q' => {
                                self.cleanup_terminal();
                                break;
                            }
                            'r' => {
                                self.cleanup_terminal();
                                self.reset();
                                restart = true;
                                break;
                            }
                            _ => (),
                        },
                        _ => (),
                    }
                }
            }
        }

        if restart {
            self.start_test();
        }

        Ok(())
    }

    pub fn start_test(&mut self) {
        let (_, mut rx): (_, mpsc::Receiver<()>) = mpsc::channel();
        self.refresh();

        let mut restart = false;

        loop {
            if rx.try_recv().is_ok() {
                self.elapsed_seconds += 1.0 / TIMER_REFRESH_RATE;
                if self.elapsed_seconds < TEST_DURATION {
                    self.refresh();
                } else {
                    break;
                }
            }

            if poll(Duration::from_millis(((1.0 / TIMER_REFRESH_RATE) * 1000.0).round() as u64)).unwrap() {
                if let Event::Key(event) = read().unwrap() {
                    if self.elapsed_seconds == 0.0 {
                        rx = TypingTest::start_timer();
                    }
                    match event.code {
                        KeyCode::Esc => {
                            self.cleanup_terminal();
                            break;
                        }
                        KeyCode::Tab => {
                            self.cleanup_terminal();
                            self.reset();
                            restart = true;
                            break;
                        }
                        KeyCode::Char(c) => {
                            self.update_char(c);
                        }
                        KeyCode::Backspace => {
                            self.backspace();
                        }
                        _ => (),
                    }
                }
            }
        }

        if restart {
            self.start_test();
        } else {
            self.show_results();
        }
    }
}
