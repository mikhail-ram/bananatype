// TODO: Add theme config file
// TODO: Change text source file directory
// TODO: Display menu where user can choose test duration
// TODO: Add more words when all words are typed
// TODO: Check if any words were typed or if person is afk
// TODO: Add line graph at summary page to get statistics
// TODO: simulate Ctrl+PLUS to increase font size of terminal

/* For MacOS
tell application "System Events"
	tell process "Terminal"
		set frontmost to true
	end tell
end tell

tell application "Terminal"
	set font size of first window to "11"
	delay 1.0
	set font size of first window to "25"
end tell
*/

use std::io;

mod lib;

fn main() -> Result<(), io::Error> {
    let mut test = lib::TypingTest::new();
    test.start_test();
    Ok(())
}
