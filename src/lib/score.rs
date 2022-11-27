pub struct Score {
    correct_characters: f64,
    incorrect_characters: f64,
    total_incorrect_characters: f64,
}

impl Score {
    pub fn new() -> Score {
        Score {
            correct_characters: 0.0,
            incorrect_characters: 0.0,
            total_incorrect_characters: 0.0,
        }
    }

    pub fn calculate_gross_wpm(&self, elapsed_seconds: f64) -> f64 {
        if elapsed_seconds == 0.0 {
            0.0
        } else {
            ((self.correct_characters + self.incorrect_characters) / 5.0)
             / (elapsed_seconds / 60.0)
        }
    }

    pub fn calculate_net_wpm(&self, elapsed_seconds: f64) -> f64 {
        if elapsed_seconds == 0.0 {
            0.0
        } else {
            self.calculate_gross_wpm(elapsed_seconds)
                - (self.incorrect_characters / (elapsed_seconds / 60.0))
        }
    }

    pub fn calculate_accuracy(&self) -> f64 {
        if self.correct_characters + self.total_incorrect_characters == 0.0 {
            100.0
        } else {
            (self.correct_characters
             / (self.correct_characters + self.total_incorrect_characters))
            * 100.0
        }
    }

    pub fn calculate_correct(&mut self) {
        self.correct_characters += 1.0;
    }

    pub fn calculate_correct_backspace(&mut self) {
        self.correct_characters -= 1.0;
    }

    pub fn calculate_incorrect(&mut self) {
        self.incorrect_characters += 1.0;
        self.total_incorrect_characters += 1.0;
    }

    pub fn calculate_incorrect_backspace(&mut self) {
        self.incorrect_characters -= 1.0;
    }
}
