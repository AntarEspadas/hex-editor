use std::io::stdout;

use crossterm::cursor::{RestorePosition, SavePosition};
use crossterm::terminal::window_size;
use crossterm::{
    cursor, cursor::MoveTo, event::KeyCode::Char, style::Print, terminal::enable_raw_mode,
    ExecutableCommand,
};

use crossterm::event::{read, EnableMouseCapture, Event, MouseEvent, MouseEventKind};

const LINE_LENGTH: usize = 16;

enum Direction {
    Up,
    Down,
    Left,
    Right,
}

fn draw_line(blob: &[u8], pos: usize, line_length: usize, cols: usize) -> std::io::Result<()> {
    let bytes = blob
        .iter()
        .skip(pos * line_length)
        .take(line_length)
        .map(|&c| format!("{:02x}", c))
        .collect::<Vec<_>>()
        .join(" ");
    let chars = blob
        .iter()
        .skip(pos * line_length)
        .take(line_length)
        .map(|&c| {
            if c.is_ascii_alphanumeric() {
                c as char
            } else {
                '.'
            }
        })
        .collect::<String>();
    let line_num = pos * line_length;

    let line = format!(
        "{line_num:08x}: {bytes: <width$} {chars}",
        width = line_length * 3
    );

    let len = line.len();
    if len < cols {
        stdout().execute(Print(line))?;
        for _ in 0..(cols - len) {
            stdout().execute(Print(" "))?;
        }
    } else {
        let line = line[..cols].to_string();
        stdout().execute(Print(line))?;
    };
    Ok(())
}

fn move_cursor(
    start: &mut usize,
    total_lines: usize,
    direction: Direction,
) -> std::io::Result<bool> {
    let mut curos_pos = cursor::position()?;
    let win_size = window_size()?;
    let rows = win_size.rows as usize;

    let mut requires_redraw = false;

    match direction {
        Direction::Up => {
            if curos_pos.1 > 0 {
                // The cursos is anywhere but the first line
                curos_pos.1 -= 1;
            } else if *start > 0 {
                // The cursos is on the first line, but there are more lines to show
                *start -= 1;
                requires_redraw = true;
            }
        }
        Direction::Down => {
            let max_start = if total_lines <= rows {
                0
            } else {
                total_lines - rows
            };
            if curos_pos.1 < (rows - 2) as u16 {
                // The cursos is anywhere but the last line
                curos_pos.1 += 1;
            } else if *start < max_start {
                // The cursos is on the last line, but there are more lines to show
                *start += 1;
                requires_redraw = true;
            }
        }
        Direction::Left => {
            if curos_pos.0 > 10 {
                curos_pos.0 -= 1;
            }
        }
        Direction::Right => {
            if curos_pos.0 < 10 + LINE_LENGTH as u16 * 3 {
                curos_pos.0 += 1;
            }
        }
    }
    stdout().execute(MoveTo(curos_pos.0, curos_pos.1))?;
    Ok(requires_redraw)
}

fn goto_start(start: &mut usize) -> std::io::Result<bool> {
    stdout().execute(MoveTo(10, 0))?;
    if *start > 0 {
        *start = 0;
        return Ok(true);
    }
    Ok(false)
}

fn goto_end(start: &mut usize, total_lines: usize) -> std::io::Result<bool> {
    let rows = window_size()?.rows as usize;
    let max_start = if total_lines <= rows {
        0
    } else {
        total_lines - (rows - 1)
    };

    stdout().execute(MoveTo(10, rows as u16 - 2))?;
    if *start < max_start {
        *start = max_start;
        return Ok(true);
    }
    Ok(false)
}

fn draw_screen(content: &[u8], start: usize) -> std::io::Result<()> {
    let win_size = window_size()?;
    stdout().execute(SavePosition)?;
    for i in 0..(win_size.rows as usize - 1) {
        stdout().execute(MoveTo(0, i as u16))?;
        draw_line(content, i + start, 16, win_size.columns as usize)?;
    }
    let message = "Press 'q' to quit";
    let message = if message.len() < win_size.columns as usize {
        // if message is shorter than the screen width, pad it with spaces
        message.to_string()
            + " "
                .repeat(win_size.columns as usize - message.len())
                .as_str()
    } else {
        // if message is longer than the screen width, truncate it
        message[..win_size.columns as usize].to_string()
    };
    stdout()
        .execute(MoveTo(0, win_size.rows - 1))?
        .execute(Print(message))?
        .execute(RestorePosition)?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    // Get first command line argument
    let path = std::env::args().nth(1).unwrap();

    let content = std::fs::read(path)?;

    let mut start = 0usize;

    enable_raw_mode()?;

    let total_lines = (content.len() as f64 / LINE_LENGTH as f64).ceil() as usize;

    draw_screen(&content, start)?;

    stdout()
        .execute(MoveTo(10, 0))?
        .execute(EnableMouseCapture)?;

    loop {
        let requires_redraw = match read()? {
            Event::Key(event) => match event.code {
                Char('q') => break,
                Char('h') => move_cursor(&mut start, total_lines, Direction::Left)?,
                Char('j') => move_cursor(&mut start, total_lines, Direction::Down)?,
                Char('k') => move_cursor(&mut start, total_lines, Direction::Up)?,
                Char('l') => move_cursor(&mut start, total_lines, Direction::Right)?,
                Char('0') => {
                    let pos = cursor::position()?;
                    stdout().execute(MoveTo(10, pos.1))?;
                    true
                }
                Char('$') => {
                    let pos = cursor::position()?;
                    stdout().execute(MoveTo(10 + LINE_LENGTH as u16 * 3, pos.1))?;
                    false
                }
                Char('g') => goto_start(&mut start)?,
                Char('G') => goto_end(&mut start, total_lines)?,
                _ => false,
            },

            Event::Mouse(e) => match e.kind {
                MouseEventKind::ScrollUp => move_cursor(&mut start, total_lines, Direction::Up)?,
                MouseEventKind::ScrollDown => {
                    move_cursor(&mut start, total_lines, Direction::Down)?
                }
                _ => false,
            },
            Event::Resize(_, _) => true,
            _ => false,
        };

        if requires_redraw {
            draw_screen(&content, start)?;
        }
    }

    // stdout()
    //     .execute(MoveTo(5, 0))?
    //     .execute(SetForegroundColor(Color::Blue))?
    //     .execute(SetBackgroundColor(Color::Red))?
    //     .execute(Print("Styled text here."))?
    //     .execute(ResetColor)?
    //     ;

    Ok(())
}
