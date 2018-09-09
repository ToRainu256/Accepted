use buffer::Buffer;
use core::Cursor;
use draw;
use std;
use std::path::PathBuf;
use termion;
use termion::event::{Event, Key, MouseButton, MouseEvent};

pub enum Transition {
    Nothing,
    Trans(Box<Mode>),
    Exit,
}

pub trait Mode {
    fn event(&mut self, buf: &mut Buffer, event: termion::event::Event) -> Transition;
    fn draw(&self, core: &Buffer, term: &mut draw::Term);
}

pub struct Normal {
    message: String,
}
struct Prefix;
struct Insert;
struct R;
struct Search;
struct Save {
    path: String,
}

impl Normal {
    pub fn new() -> Self {
        Self {
            message: String::new(),
        }
    }

    pub fn with_message(message: String) -> Self {
        Self { message }
    }
}

impl Mode for Normal {
    fn event(&mut self, buf: &mut Buffer, event: termion::event::Event) -> Transition {
        match event {
            Event::Key(Key::Char('i')) => return Transition::Trans(Box::new(Insert)),
            Event::Key(Key::Char('I')) => {
                let mut i = 0;
                {
                    let line = buf.core.current_line();
                    while i < line.len() && line[i] == ' ' {
                        i += 1;
                    }
                }
                buf.core.cursor.col = i;
                return Transition::Trans(Box::new(Insert));
            }
            Event::Key(Key::Char('a')) => {
                buf.core.cursor_right();
                return Transition::Trans(Box::new(Insert));
            }
            Event::Key(Key::Char('A')) => {
                buf.core.cursor.col = buf.core.current_line().len();
                return Transition::Trans(Box::new(Insert));
            }
            Event::Key(Key::Char('r')) => return Transition::Trans(Box::new(R)),
            Event::Key(Key::Char('o')) => {
                buf.core.insert_newline();
                return Transition::Trans(Box::new(Insert));
            }
            Event::Key(Key::Char('O')) => {
                buf.core.cursor_up();
                buf.core.insert_newline();
                return Transition::Trans(Box::new(Insert));
            }
            Event::Key(Key::Char('h')) => buf.core.cursor_left(),
            Event::Key(Key::Char('j')) => buf.core.cursor_down(),
            Event::Key(Key::Char('k')) => buf.core.cursor_up(),
            Event::Key(Key::Char('l')) => buf.core.cursor_right(),
            Event::Key(Key::Char('/')) => return Transition::Trans(Box::new(Search)),
            Event::Key(Key::Char(' ')) => return Transition::Trans(Box::new(Prefix)),

            Event::Mouse(MouseEvent::Press(MouseButton::Left, x, y)) => {
                let col = x as usize - 1;
                let row = y as usize - 1;
                let cursor = Cursor { row, col };

                let mut term = draw::Term::new();
                let height = term.height;
                let width = term.width;
                buf.draw(term.view((0, 0), height, width));

                if let Some(c) = term.pos(cursor) {
                    buf.core.cursor = c;
                }
            }

            Event::Mouse(MouseEvent::Press(MouseButton::WheelUp, _, _)) => {
                if buf.core.row_offset < 3 {
                    buf.core.row_offset = 0;
                } else {
                    buf.core.row_offset -= 3;
                }
            }
            Event::Mouse(MouseEvent::Press(MouseButton::WheelDown, _, _)) => {
                buf.core.row_offset =
                    std::cmp::min(buf.core.row_offset + 3, buf.core.buffer.len() - 1);
            }
            _ => {}
        }
        Transition::Nothing
    }

    fn draw(&self, buf: &Buffer, term: &mut draw::Term) {
        let height = term.height;
        let width = term.width;
        let cursor = buf.draw(term.view((0, 0), height - 1, width));
        term.cursor = cursor
            .map(|c| draw::CursorState::Show(c, draw::CursorShape::Block))
            .unwrap_or(draw::CursorState::Hide);

        let mut footer = term.view((height - 1, 0), 1, width);
        footer.puts(
            &format!(
                "[Normal] ({} {}) [{}] {}",
                buf.core.cursor.row + 1,
                buf.core.cursor.col + 1,
                buf.path
                    .as_ref()
                    .map(|p| p.to_string_lossy())
                    .unwrap_or("*".into()),
                &self.message
            ),
            draw::CharStyle::Footer,
        );
    }
}

impl Mode for Insert {
    fn event(&mut self, buf: &mut Buffer, event: termion::event::Event) -> Transition {
        let core = &mut buf.core;
        match event {
            Event::Key(Key::Esc) => {
                return Transition::Trans(Box::new(Normal::new()));
            }
            Event::Key(Key::Backspace) => {
                core.backspase();
            }
            Event::Key(Key::Char('\t')) => {
                core.insert(' ');
                while core.cursor.col % 4 != 0 {
                    core.insert(' ');
                }
            }
            Event::Key(Key::Char(c)) => {
                core.insert(c);
            }
            _ => {}
        }
        Transition::Nothing
    }

    fn draw(&self, buf: &Buffer, term: &mut draw::Term) {
        let height = term.height;
        let width = term.width;
        let cursor = buf.draw(term.view((0, 0), height, width));
        term.cursor = cursor
            .map(|c| draw::CursorState::Show(c, draw::CursorShape::Bar))
            .unwrap_or(draw::CursorState::Hide);
    }
}

impl Mode for R {
    fn event(&mut self, buf: &mut Buffer, event: termion::event::Event) -> Transition {
        let core = &mut buf.core;
        match event {
            Event::Key(Key::Esc) => {
                return Transition::Trans(Box::new(Normal::new()));
            }
            Event::Key(Key::Char(c)) => {
                core.replace(c);
                return Transition::Trans(Box::new(Normal::new()));
            }
            _ => {}
        }
        Transition::Nothing
    }

    fn draw(&self, buf: &Buffer, term: &mut draw::Term) {
        let height = term.height;
        let width = term.width;
        let cursor = buf.draw(term.view((0, 0), height, width));
        term.cursor = cursor
            .map(|c| draw::CursorState::Show(c, draw::CursorShape::Underline))
            .unwrap_or(draw::CursorState::Hide);
    }
}

impl Mode for Search {
    fn event(&mut self, buf: &mut Buffer, event: termion::event::Event) -> Transition {
        match event {
            Event::Key(Key::Esc) => {
                return Transition::Trans(Box::new(Normal::new()));
            }
            Event::Key(Key::Backspace) => {
                buf.search.pop();
            }
            Event::Key(Key::Char(c)) => {
                if c == '\n' {
                    return Transition::Trans(Box::new(Normal::new()));
                }
                buf.search.push(c);
            }
            _ => {}
        }
        Transition::Nothing
    }

    fn draw(&self, buf: &Buffer, term: &mut draw::Term) {
        let height = term.height - 1;
        let width = term.width;
        let cursor = buf.draw(term.view((0, 0), height, width));
        term.cursor = cursor
            .map(|c| draw::CursorState::Show(c, draw::CursorShape::Block))
            .unwrap_or(draw::CursorState::Hide);

        let mut footer = term.view((height, 0), 1, width);
        footer.put('/', draw::CharStyle::Default, None);
        for &c in &buf.search {
            footer.put(c, draw::CharStyle::Default, None);
        }
    }
}

impl Mode for Save {
    fn event(&mut self, buf: &mut Buffer, event: termion::event::Event) -> Transition {
        match event {
            Event::Key(Key::Esc) => {
                return Transition::Trans(Box::new(Normal::new()));
            }
            Event::Key(Key::Backspace) => {
                self.path.pop();
            }
            Event::Key(Key::Char(c)) => {
                if c == '\n' {
                    buf.path = Some(PathBuf::from(self.path.clone()));
                    let message = if buf.save().unwrap().is_ok() {
                        format!("Saved to {}", self.path)
                    } else {
                        format!("Failed to save {}", self.path)
                    };
                    return Transition::Trans(Box::new(Normal::with_message(message)));
                }
                self.path.push(c);
            }
            _ => {}
        }
        Transition::Nothing
    }

    fn draw(&self, buf: &Buffer, term: &mut draw::Term) {
        let height = term.height - 2;
        let width = term.width;
        let cursor = buf.draw(term.view((0, 0), height, width));
        term.cursor = cursor
            .map(|c| draw::CursorState::Show(c, draw::CursorShape::Block))
            .unwrap_or(draw::CursorState::Hide);

        let mut footer = term.view((height, 0), 2, width);
        footer.puts(
            &std::env::current_dir().unwrap().to_string_lossy(),
            draw::CharStyle::UI,
        );
        footer.newline();
        footer.puts("> ", draw::CharStyle::UI);
        footer.puts(&self.path, draw::CharStyle::UI);
    }
}

impl Mode for Prefix {
    fn event(&mut self, buf: &mut Buffer, event: termion::event::Event) -> Transition {
        match event {
            Event::Key(Key::Esc) => {
                return Transition::Trans(Box::new(Normal::new()));
            }
            Event::Key(Key::Char('q')) => {
                return Transition::Exit;
            }
            Event::Key(Key::Char('s')) => {
                if let Some(ref path) = buf.path {
                    let message = if buf.save().unwrap().is_ok() {
                        format!("Saved to {}", path.to_string_lossy())
                    } else {
                        format!("Failed to save {}", path.to_string_lossy())
                    };
                    return Transition::Trans(Box::new(Normal::with_message(message)));
                } else {
                    return Transition::Trans(Box::new(Save {
                        path: String::new(),
                    }));
                }
            }
            _ => {}
        }
        Transition::Nothing
    }

    fn draw(&self, buf: &Buffer, term: &mut draw::Term) {
        let height = term.height - 1;
        let width = term.width;
        let cursor = buf.draw(term.view((0, 0), height, width));
        term.cursor = cursor
            .map(|c| draw::CursorState::Show(c, draw::CursorShape::Block))
            .unwrap_or(draw::CursorState::Hide);

        let mut footer = term.view((height, 0), 1, width);
        footer.puts("Prefix ... [q: Quit] [s: Save]", draw::CharStyle::Footer);
    }
}
