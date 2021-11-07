// TODO: Show cursor when typing test
// TODO: Display menu where user can choose test duration
// TODO: Display menu which shows users gross and net wpm
// TODO: Add shortcut to restart test
// TODO: Add more words when all words are typed
// TODO: Check if any words were typed or if person is afk

use crossterm::event::{poll, read, Event, KeyCode};
use rand::{distributions::Uniform, Rng};
use std::fs::File;
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

type CrosstermTerminal = Terminal<CrosstermBackend<RawTerminal<Stdout>>>;
const NUMBER_OF_WORDS: usize = 100;
const TEST_DURATION: f64 = 10.0;
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

struct Score {
    correct_characters: f64,
    incorrect_characters: f64,
    total_incorrect_characters: f64,
}

impl Score {
    fn new() -> Score {
        Score {
            correct_characters: 0.0,
            incorrect_characters: 0.0,
            total_incorrect_characters: 0.0,
        }
    }

    fn calculate_gross_wpm(&self, elapsed_seconds: f64) -> f64 {
        if elapsed_seconds == 0.0 {
            0.0
        } else {
            ((self.correct_characters + self.incorrect_characters) / 5.0) / (elapsed_seconds / 60.0)
        }
    }

    fn calculate_net_wpm(&self, elapsed_seconds: f64) -> f64 {
        if elapsed_seconds == 0.0 {
            0.0
        } else {
            self.calculate_gross_wpm(elapsed_seconds)
                - (self.incorrect_characters / (elapsed_seconds / 60.0))
        }
    }

    fn calculate_accuracy(&self) -> f64 {
        if self.correct_characters + self.total_incorrect_characters == 0.0 {
            100.0
        } else {
            (self.correct_characters / (self.correct_characters + self.total_incorrect_characters))
                * 100.0
        }
    }
}

struct TypingTest<'a> {
    filepath: &'a str,
    text: Vec<Span<'a>>,
    position: usize,
    terminal: CrosstermTerminal,
    elapsed_seconds: f64,
    score: Score,
}

impl TypingTest<'_> {
    fn new(filepath: &str) -> TypingTest {
        let text = TypingTest::generate_text(filepath);
        let terminal = TypingTest::setup_terminal().unwrap();
        let score = Score::new();

        TypingTest {
            filepath,
            text,
            terminal,
            position: 0,
            elapsed_seconds: 0.0,
            score,
        }
    }

    // Generates test text from filepath
    fn generate_text(filepath: &str) -> Vec<Span> {
        let file = File::open(filepath).unwrap();
        let file: Vec<String> = io::BufReader::new(file)
            .lines()
            .filter_map(io::Result::ok)
            .collect();

        let rand_nums: Vec<usize> = rand::thread_rng()
            .sample_iter(Uniform::from(0..file.len()))
            .take(NUMBER_OF_WORDS)
            .collect();

        let mut spans = Vec::new();

        for num in rand_nums {
            spans.extend([
                Span::styled(file[num].clone(), Style::default().fg(Color::DarkGray)),
                Span::styled(" ", Style::default().fg(Color::DarkGray)),
            ]);
        }

        spans
    }

    fn update_char(&mut self, character: char) {
        let current_word = &self.text[self.position].content;
        let (former, latter) = current_word.split_at(1);

        let latter = Span::styled(latter.to_string(), Style::default().fg(Color::DarkGray));

        let former = former.chars().next().unwrap();
        let former = match former {
            former if former == character => {
                self.score.correct_characters += 1.0;
                Span::styled(former.to_string(), Style::default().fg(Color::Green))
            }
            ' ' => {
                self.score.incorrect_characters += 1.0;
                self.score.total_incorrect_characters += 1.0;
                Span::styled(former.to_string(), Style::default().bg(Color::Red))
            }
            _ => {
                self.score.incorrect_characters += 1.0;
                self.score.total_incorrect_characters += 1.0;
                Span::styled(former.to_string(), Style::default().fg(Color::Red))
            }
        };

        if latter.content != "" {
            self.text
                .splice(self.position..self.position + 1, [former, latter]);
        } else {
            self.text[self.position] = former;
        }

        self.position += 1;
        self.refresh();
    }

    fn backspace(&mut self) {
        self.position = if self.position > 0 {
            if self.text[self.position - 1].style.fg == Some(Color::Green) {
                self.score.correct_characters -= 1.0;
            } else {
                self.score.incorrect_characters -= 1.0;
            }
            self.text[self.position - 1].style = Style::default().fg(Color::DarkGray);
            self.position - 1
        } else {
            if self.text[0].style.fg == Some(Color::Green) {
                self.score.correct_characters -= 1.0;
            } else {
                self.score.incorrect_characters -= 1.0;
            }
            self.text[0].style = Style::default().fg(Color::DarkGray);
            0
        };
        self.refresh();
    }

    fn refresh(&mut self) -> Result<(), io::Error> {
        let time_block = Block::default()
            .title(Span::styled(
                "Time",
                Style::default().add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(Color::DarkGray))
            .borders(Borders::ALL)
            .border_type(BorderType::Thick);

        let progress = Gauge::default()
            .block(time_block)
            .gauge_style(Style::default().fg(Color::DarkGray))
            .ratio(self.elapsed_seconds / TEST_DURATION)
            .label(format!(
                "{}",
                TEST_DURATION.round() as u8 - (self.elapsed_seconds).round() as u8
            ));

        let gross_wpm_block = Block::default()
            .title(Span::styled(
                "Gross WPM",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(Color::DarkGray))
            .borders(Borders::ALL)
            .border_type(BorderType::Thick);

        let gross_wpm = Paragraph::new(Span::styled(
            format!(
                "{:.1}",
                self.score.calculate_gross_wpm(self.elapsed_seconds)
            ),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))
        .block(gross_wpm_block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

        let net_wpm_block = Block::default()
            .title(Span::styled(
                "Net WPM",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(Color::DarkGray))
            .borders(Borders::ALL)
            .border_type(BorderType::Thick);

        let net_wpm = Paragraph::new(Span::styled(
            format!("{:.1}", self.score.calculate_net_wpm(self.elapsed_seconds)),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))
        .block(net_wpm_block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

        let accuracy_block = Block::default()
            .title(Span::styled(
                "Accuracy",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(Color::DarkGray))
            .borders(Borders::ALL)
            .border_type(BorderType::Thick);

        let accuracy = Paragraph::new(Span::styled(
            format!("{:.1}", self.score.calculate_accuracy()),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))
        .block(accuracy_block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

        let input_block = Block::default()
            .title(Span::styled(
                "BananaType",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(Color::Yellow))
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

            frame.render_widget(progress, layout[0]);
            frame.render_widget(gross_wpm, live_stats_layout[0]);
            frame.render_widget(net_wpm, live_stats_layout[1]);
            frame.render_widget(accuracy, live_stats_layout[2]);
            frame.render_widget(text, layout[2]);
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
        self.text = TypingTest::generate_text(self.filepath);
        self.score = Score::new();
        self.position = 0;
        self.elapsed_seconds = 0.0;
    }

    fn show_results(&mut self) -> Result<(), io::Error> {
        let results_block = Block::default()
            .title(Span::styled(
                "Your Results",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ))
            .border_style(Style::default().fg(Color::Yellow))
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
            if poll(Duration::from_millis(500)).unwrap() {
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

    fn start_test(&mut self) {
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

            if poll(Duration::from_millis(500)).unwrap() {
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

fn main() -> Result<(), io::Error> {
    let mut test = TypingTest::new("src/words.txt");
    test.start_test();
    Ok(())
}
