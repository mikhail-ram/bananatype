use std::fs::File;
use std::io::{ BufReader, BufRead, self };
use std::{ string::ToString, time::{ Duration, Instant } };
use crossterm::{ execute, terminal, cursor, event::{ read, Event, KeyCode }, style::{ self, Colorize } };
use rand::{ distributions::Uniform, Rng };

fn main() {
    let file = File::open("src/words.txt").unwrap();
    let file: Vec<String> = BufReader::new(file).lines().filter_map(io::Result::ok).collect();
    let mut stdout = io::stdout();
    let mut current_word = String::new();
    let mut word_index = 0;
    let mut correct_to_incorrect = vec![0; file.len()];

    execute!(stdout, terminal::Clear(terminal::ClearType::All), cursor::MoveTo(0, 0)).unwrap();

    for (i, word) in file.iter().enumerate() {
        if correct_to_incorrect[i] == 1 {
            execute!(stdout, style::PrintStyledContent(format!("{} ", word).green())).unwrap();
        }
        else if correct_to_incorrect[i] == -1 {
            execute!(stdout, style::PrintStyledContent(format!("{} ", word).red())).unwrap();
        }
        else {
            execute!(stdout, style::Print(format!("{} ", word))).unwrap();
        }
    }
    println!();
    println!();


    loop {
        // `read()` blocks until an `Event` is available
        if let Event::Key(c) = read().unwrap() {
            match c.code {
                KeyCode::Char(d) => {
                    if d != ' ' {
                        current_word.push(d);
                        if file[word_index].starts_with(current_word.as_str()) {
                            execute!(stdout, style::Print(d)).unwrap();
                        }
                        else {
                            execute!(stdout, style::PrintStyledContent(d.to_owned().red())).unwrap();
                        }
                    }
                    else {
                        if current_word == file[word_index] {
                            correct_to_incorrect[word_index] = 1;
                        }
                        else {
                            correct_to_incorrect[word_index] = -1;
                        }
                        word_index += 1;
                        current_word.clear();
                        execute!(stdout, terminal::Clear(terminal::ClearType::All), cursor::MoveTo(0, 0)).unwrap();
                        for (i, word) in file.iter().enumerate() {
                            if correct_to_incorrect[i] == 1 {
                                execute!(stdout, style::PrintStyledContent(format!("{} ", word).green())).unwrap();
                            }
                            else if correct_to_incorrect[i] == -1 {
                                execute!(stdout, style::PrintStyledContent(format!("{} ", word).red())).unwrap();
                            }
                            else {
                                execute!(stdout, style::Print(format!("{} ", word))).unwrap();
                            }
                        }
                        println!();
                        println!();
                    }
                }
                KeyCode::Backspace => {
                    current_word.pop();
                    execute!(stdout, cursor::MoveLeft(1), terminal::Clear(terminal::ClearType::UntilNewLine)).unwrap();
                },
                KeyCode::Esc => {
                    println!();
                    println!();
                    println!("Quitting...");
                    break;
                },
                _ => ()
            }
        }
    }
}
