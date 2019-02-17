use std::collections::VecDeque;
use std::borrow::Cow;
use std::rc::Rc;
use std::cell::RefCell;

enum HistoryPos {
    /// No active input string
    Nothing,
    /// The currently active input string
    Str(String),
    /// The position in the line history
    Pos(isize),
}

/// Effectively a vector of unique strings that drops the oldest items when it overflows
/// the capacity and moves duplicates to the back when inserted.
///
/// Note: It is done in a simple way for now and has a big potential for performance
/// improvement if needed
struct LineHistory {
    lines: VecDeque<String>,
    max_history: usize,
}

impl LineHistory {
    fn new(max_history: usize) -> LineHistory {
        LineHistory {
            lines: Default::default(),
            max_history: max_history,
        }
    }

    fn find_pos(&self, new_s: impl AsRef<str>) -> Option<usize> {
        let new_s = new_s.as_ref();
        self.lines.iter().position(|org_s| org_s == new_s)
    }

    fn add(&mut self, new_s: impl AsRef<str> + Into<String>) {
        if self.max_history == 0 {
            debug_assert!(false, "Add should never be called with no history");
            return;
        }

        if let Some(pos) = self.find_pos(new_s.as_ref()) {
            let s = self.lines.remove(pos).unwrap(); // Failure should be impossible
            self.lines.push_back(s);
        } else {
            if self.lines.len() == self.max_history {
                self.lines.pop_front();
            }
            self.lines.push_back(new_s.into());
        }
    }

    fn get(&self, pos: isize) -> Option<&String> {
        let len = self.lines.len();
        if len == 0 || pos == 0 {
            None
        } else if pos < 0 {
            let pos = pos.checked_abs().unwrap_or(0) as usize % (len + 1);
            if pos == 0 {
                None
            } else {
                Some(&self.lines[pos - 1])
            }
        } else {
            let pos = pos as usize % (len + 1);
            if pos == 0 {
                None
            } else {
                Some(&self.lines[len - pos])
            }
        }
    }
}

/// Structure that keeps track of the history of input and provides functionality
/// to allow up/down arrow like what you get in a shell.
pub struct InputHistory {
    lines: Rc<RefCell<LineHistory>>,
}

impl InputHistory {
    /// Initialize a new instance of input history. This is expected to live as long
    /// as the application.
    pub fn new(max_history: usize) -> InputHistory {
        InputHistory {
            lines: Rc::new(RefCell::new(LineHistory::new(max_history))),
        }
    }

    /// Create a refrence to the input history that is owned and can be used inside
    /// a mode.
    pub fn make_ref(&self, init_query: Option<String>) -> InputHistoryRef {
        let mut result = InputHistoryRef {
            lines: self.lines.clone(),
            current: HistoryPos::Nothing,
            no_history: self.lines.borrow().max_history == 0,
        };
        result.set_current(init_query);
        result
    }
}

/// A refrence to an existing `InputHistory` that is used to manipulate the current
/// state of it. This keeps track of the current input line and adds it to the list
/// when it is cleared or goes out of scope.
pub struct InputHistoryRef {
    lines: Rc<RefCell<LineHistory>>,
    current: HistoryPos,
    no_history: bool,
}

impl Drop for InputHistoryRef {
    fn drop(&mut self) {
        self.clear();
    }
}

impl InputHistoryRef {
    /// Replace the current input string entirely. The old one will be discarded
    /// and not be added to the history.
    pub fn set_current(&mut self, current: Option<String>) {
        if let Some(s) = current {
            self.current = HistoryPos::Str(s);
        } else {
            self.current = HistoryPos::Nothing;
        }
    }

    /// Add a character to the current input string and return a refrence to it.
    pub fn push_char(&mut self, c: char) -> &String {
        if let HistoryPos::Str(ref mut s) = self.current {
            s.push(c);
        } else {
            self.current = HistoryPos::Str(c.to_string());
        }

        if let HistoryPos::Str(ref s) = self.current {
            s
        } else {
            unreachable!();
        }
    }

    /// Remove the last character from the current input string and return
    /// a refrence to it if it existed.
    pub fn pop_char(&mut self) -> Option<&String> {
        if let HistoryPos::Str(ref mut s) = self.current {
            s.pop();
            Some(s)
        } else {
            None
        }
    }

    /// Clear the current input string and push it onto the history
    pub fn clear(&mut self) {
        if self.no_history {
            self.current = HistoryPos::Nothing;
            return;
        }

        let mut tmp = HistoryPos::Nothing;
        std::mem::swap(&mut tmp, &mut self.current);

        match tmp {
            HistoryPos::Nothing => {},
            HistoryPos::Str(s) => {
                self.lines.borrow_mut().add(s);
            },
            HistoryPos::Pos(pos) => {
                let mut lines = self.lines.borrow_mut();
                if let Some(s) = lines.get(pos).cloned() {
                    lines.add(s);
                }
            },
        }
    }

    /// Return the current string as a refrence. Mostly provided to let old code
    /// work as it used to when the input was an Option<String>.
    pub fn as_ref(&self) -> Option<Cow<str>> {
        match self.current {
            HistoryPos::Nothing => {
                None
            },
            HistoryPos::Str(ref s) => {
                Some(Cow::Borrowed(s.as_str()))
            },
            HistoryPos::Pos(pos) => {
                self.lines.borrow().get(pos).map(|s| Cow::Owned(s.clone()))
            },
        }
    }

    /// Move to the previous entry in the history. If there is a current entry it will
    /// be added to the history. The new current entry is returned.
    pub fn move_to_prev(&mut self) -> Option<Cow<str>> {
        if self.no_history {
            return self.as_ref();
        }

        let mut tmp = HistoryPos::Nothing;
        std::mem::swap(&mut tmp, &mut self.current);

        match tmp {
            HistoryPos::Nothing => {
                self.current = HistoryPos::Pos(1);
            },
            HistoryPos::Str(s) => {
                self.lines.borrow_mut().add(s);
                self.current = HistoryPos::Pos(2);
            },
            HistoryPos::Pos(mut pos) => {
                pos += 1;
                self.current = HistoryPos::Pos(pos);
            }
        }
        self.as_ref()
    }

    /// Move to the next entry in the history. If there is a current entry it will
    /// be added to the history. The new current entry is returned.
    pub fn move_to_next(&mut self) -> Option<Cow<str>> {
        if self.no_history {
            return self.as_ref();
        }

        let mut tmp = HistoryPos::Nothing;
        std::mem::swap(&mut tmp, &mut self.current);

        match tmp {
            HistoryPos::Nothing => {
                self.current = HistoryPos::Pos(-1);
            },
            HistoryPos::Str(s) => {
                self.lines.borrow_mut().add(s);
                self.current = HistoryPos::Pos(0);
            },
            HistoryPos::Pos(mut pos) => {
                pos -= 1;
                self.current = HistoryPos::Pos(pos);
            }
        }
        self.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use super::InputHistory;

    #[test]
    fn verify_basic_history() {
        let history = InputHistory::new(4);

        // Initializing a refrence with a default string and dropping it should add it to the history
        history.make_ref(Some("a".to_string()));
        assert_eq!(history.lines.borrow().lines, vec!["a".to_string()]);

        history.make_ref(Some("b".to_string()));
        assert_eq!(history.lines.borrow().lines, vec!["a".to_string(), "b".to_string()]);

        history.make_ref(None);
        assert_eq!(history.lines.borrow().lines, vec!["a".to_string(), "b".to_string()]);

        history.make_ref(None).push_char('c');
        assert_eq!(history.lines.borrow().lines, vec!["a".to_string(), "b".to_string(), "c".to_string()]);

        // Adding a duplicate should put it in the back of the list
        history.make_ref(Some("b".to_string()));
        assert_eq!(history.lines.borrow().lines, vec!["a".to_string(), "c".to_string(), "b".to_string()]);

        history.make_ref(None).push_char('d');
        assert_eq!(history.lines.borrow().lines, vec!["a".to_string(), "c".to_string(), "b".to_string(), "d".to_string()]);

        history.make_ref(None).push_char('e');
        assert_eq!(history.lines.borrow().lines, vec!["c".to_string(), "b".to_string(), "d".to_string(), "e".to_string()]);

        {
            let mut h = history.make_ref(None);
            assert_eq!(h.as_ref(), None);

            h.push_char('f');

            for &c in &["e", "d", "b", "\0", "f", "e", "d"] {
                if c == "\0" {
                    assert_eq!(h.move_to_prev(), None);
                    assert_eq!(h.as_ref(), None);
                } else {
                    assert_eq!(h.move_to_prev(), Some(Cow::Borrowed(c)));
                    assert_eq!(h.as_ref(), Some(Cow::Borrowed(c)));
                }
            }

            h.push_char('g');

            for &c in &["\0", "d", "e", "f", "g", "\0", "d", "e"] {
                if c == "\0" {
                    assert_eq!(h.move_to_next(), None);
                    assert_eq!(h.as_ref(), None);
                } else {
                    assert_eq!(h.move_to_next(), Some(Cow::Borrowed(c)));
                    assert_eq!(h.as_ref(), Some(Cow::Borrowed(c)));
                }
            }
        }

        assert_eq!(history.lines.borrow().lines, vec!["d".to_string(), "f".to_string(), "g".to_string(), "e".to_string()]);

        {
            let mut h = history.make_ref(Some("hi".to_string()));
            assert_eq!(h.as_ref(), Some(Cow::Borrowed("hi")));

            h.pop_char().expect("Failed to pop char");
            assert_eq!(h.as_ref(), Some(Cow::Borrowed("h")));
        }

        assert_eq!(history.lines.borrow().lines, vec!["f".to_string(), "g".to_string(), "e".to_string(), "h".to_string()]);

        assert_eq!(history.make_ref(None).move_to_prev(), Some(Cow::Borrowed("h")));
    }

    #[test]
    fn verify_no_history() {
        let history = InputHistory::new(0);

        history.make_ref(Some("a".to_string()));
        assert_eq!(history.lines.borrow().lines, Vec::<String>::new());

        history.make_ref(Some("b".to_string()));
        assert_eq!(history.lines.borrow().lines, Vec::<String>::new());

        {
            let mut h = history.make_ref(None);
            assert_eq!(h.as_ref(), None);
            assert_eq!(h.move_to_next(), None);
            assert_eq!(h.move_to_next(), None);
            assert_eq!(h.move_to_next(), None);
            assert_eq!(h.move_to_prev(), None);
            assert_eq!(h.move_to_prev(), None);
            assert_eq!(h.move_to_prev(), None);
        }

        {
            let mut h = history.make_ref(Some("c".to_string()));
            assert_eq!(h.as_ref(), Some(Cow::Borrowed("c")));
            assert_eq!(h.move_to_next(), Some(Cow::Borrowed("c")));
            assert_eq!(h.move_to_next(), Some(Cow::Borrowed("c")));
            assert_eq!(h.move_to_next(), Some(Cow::Borrowed("c")));
            assert_eq!(h.move_to_prev(), Some(Cow::Borrowed("c")));
            assert_eq!(h.move_to_prev(), Some(Cow::Borrowed("c")));
            assert_eq!(h.move_to_prev(), Some(Cow::Borrowed("c")));
        }

        {
            let mut h = history.make_ref(None);
            assert_eq!(h.as_ref(), None);
            h.push_char('d');

            assert_eq!(h.move_to_next(), Some(Cow::Borrowed("d")));
            assert_eq!(h.move_to_next(), Some(Cow::Borrowed("d")));
            assert_eq!(h.move_to_next(), Some(Cow::Borrowed("d")));
            assert_eq!(h.move_to_prev(), Some(Cow::Borrowed("d")));
            assert_eq!(h.move_to_prev(), Some(Cow::Borrowed("d")));
            assert_eq!(h.move_to_prev(), Some(Cow::Borrowed("d")));
        }

        {
            let mut h = history.make_ref(None);
            assert_eq!(h.as_ref(), None);
            h.push_char('e');
            h.push_char('f');
            h.pop_char().expect("Failed to pop char");

            assert_eq!(h.move_to_next(), Some(Cow::Borrowed("e")));
            assert_eq!(h.move_to_next(), Some(Cow::Borrowed("e")));
            assert_eq!(h.move_to_next(), Some(Cow::Borrowed("e")));
            assert_eq!(h.move_to_prev(), Some(Cow::Borrowed("e")));
            assert_eq!(h.move_to_prev(), Some(Cow::Borrowed("e")));
            assert_eq!(h.move_to_prev(), Some(Cow::Borrowed("e")));
        }

        assert_eq!(history.lines.borrow().lines, Vec::<String>::new());

        assert_eq!(history.make_ref(None).pop_char(), None);
    }
}
