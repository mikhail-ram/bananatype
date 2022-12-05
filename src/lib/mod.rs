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
use tui::widgets::{Block, BorderType, Borders, Gauge, Paragraph, Wrap};
use tui::Terminal;

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

// TODO: use structs to encode colors to generalise themes
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

pub struct TypingTest<'a> {
    text: Vec<Span<'a>>,
    position: usize,
    terminal: CrosstermTerminal,
    elapsed_seconds: f64,
    score: score::Score,
    theme: Theme,
}

impl TypingTest<'_> {
    pub fn new() -> TypingTest<'static> {
        //let text = TypingTest::generate_text();
        let terminal = TypingTest::setup_terminal().unwrap();
        let score = score::Score::new();
        let theme = Theme::new();

        let mut typing_test = TypingTest {
                             text: Vec::new(),
                             terminal,
                             position: 0,
                             elapsed_seconds: 0.0,
                             score,
                             theme,
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

        if current_word.len() > 1 {
            let (former, latter) = current_word.split_at(1);
            let former = former.chars().next().unwrap();

            let former = match former {
                former if former == character => {
                    self.score.calculate_correct();
                    Span::styled(former.to_string(), Style::default().fg(self.theme.correct))
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
            } else {
                let cursor = Span::styled(latter.to_string(),
                                          Style::default().fg(self.theme.cursor).bg(self.theme.fg));

                self.text
                    .splice(self.position..self.position + 1, [former, cursor]);
            }
        } else if current_word.len() == 1 {
            let former = current_word.chars().next().unwrap();

            let former = match former {
                former if former == character => {
                    self.score.calculate_correct();
                    Span::styled(former.to_string(), Style::default().fg(self.theme.correct))
                }
                _ => {
                    self.score.calculate_incorrect();
                    Span::styled(former.to_string(), Style::default().fg(self.theme.incorrect))
                }
            };

            if next_word.len() > 1 {
                let (cursor, latter) = next_word.split_at(1);

                let cursor = Span::styled(cursor.to_string(),
                                          Style::default().fg(self.theme.cursor).bg(self.theme.fg));

                let latter = Span::styled(latter.to_string(),
                                          Style::default().fg(self.theme.fg));

                self.text
                    .splice(self.position..self.position + 2, [former, cursor, latter]);
            } else {
                let cursor = Span::styled(next_word.to_string(),
                                          Style::default().fg(self.theme.cursor).bg(self.theme.fg));

                self.text
                    .splice(self.position..self.position + 2, [former, cursor]);
            }

        }


        self.position += 1;
        self.refresh();
    }

    fn backspace(&mut self) {
        if self.position > 0 {
            if self.text[self.position - 1].style.fg == Some(self.theme.correct) {
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
                self.score.calculate_gross_wpm(self.elapsed_seconds)
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
            format!("{:.1}", self.score.calculate_net_wpm(self.elapsed_seconds)),
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
                //.margin(2)
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
                //.margin(2)
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
            Spans::from(Span::raw("Gross WPM:")),
            Spans::from(Span::styled(
                format!(
                    "{:.1}",
                    self.score.calculate_gross_wpm(self.elapsed_seconds)
                ),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Spans::from(Span::raw("Net WPM:")),
            Spans::from(Span::styled(
                format!("{:.1}", self.score.calculate_net_wpm(self.elapsed_seconds)),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Spans::from(Span::raw("Accuracy:")),
            Spans::from(Span::styled(
                format!("{:.1}", self.score.calculate_accuracy()),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Spans::from(Span::raw("")),
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

        self.terminal.draw(|frame| {
            let size = frame.size();
            frame.render_widget(results, size);
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
                self.elapsed_seconds += 0.5;
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
