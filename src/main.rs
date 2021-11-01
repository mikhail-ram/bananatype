use rand::{distributions::Uniform, Rng};
use std::fs;
use std::io::{self, stdin, stdout, BufRead, Write};
use termion::color;
use termion::cursor::DetectCursorPos;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

struct Text {
    text: String,
    current_word: usize,
}
impl Text {
    fn new(filepath: &str) -> Text {
        let text = Text::generate_text(filepath);
        Text {
            text,
            current_word: 0,
        }
    }
    fn generate_text(filepath: &str) -> String {
        let file = fs::File::open(filepath).unwrap();
        let file: Vec<String> = io::BufReader::new(file)
            .lines()
            .filter_map(io::Result::ok)
            .collect();
        let range = Uniform::from(0..file.len());
        let rand_nums: Vec<usize> = rand::thread_rng().sample_iter(range).take(100).collect();
        let file: String = rand_nums
            .into_iter()
            .map(|n| file[n].clone())
            .collect::<Vec<String>>()
            .join(" ");
        file
    }
}

fn main() {
    let file = Text::new("src/words.txt");
    let stdin = stdin();
    let stdin = stdin.lock();
    let stdout = stdout();
    let mut stdout = stdout.lock().into_raw_mode().unwrap();
    write!(
        stdout,
        "{}{}{}{}",
        termion::clear::All,
        termion::cursor::Goto(1, 1),
        file.text,
        termion::cursor::Goto(1, 1),
    )
    .unwrap();
    stdout.flush().unwrap();
    let mut keys = stdin.keys();
    let mut position: usize = 0;
    loop {
        let c = keys.next().unwrap().unwrap();
        match c {
            Key::Char(a) => {
                if a == ' ' {
                    if a == file.text.chars().nth(position).unwrap() {
                        write!(
                            stdout,
                            "{}{}",
                            color::Fg(color::Green),
                            file.text.chars().nth(position).unwrap()
                        );
                    } else {
                        write!(
                            stdout,
                            "{}{}",
                            color::Fg(color::Red),
                            file.text.chars().nth(position).unwrap()
                        );
                    }
                } else {
                    if a == file.text.chars().nth(position).unwrap() {
                        write!(stdout, "{}{}", color::Fg(color::Green), a);
                    } else {
                        write!(stdout, "{}{}", color::Fg(color::Red), a);
                    }
                }
                position += 1;
            }
            Key::Backspace => {
                let (x, y) = stdout.cursor_pos().unwrap();
                write!(
                    stdout,
                    "{}{}{}",
                    color::Fg(termion::color::Reset),
                    file.text.chars().nth(x as usize - 1).unwrap(),
                    termion::cursor::Goto(x - 1, y)
                );
                position -= 1;
            }
            Key::Ctrl('q') => break,
            _ => (),
        };
        stdout.flush().unwrap();
    }
}
