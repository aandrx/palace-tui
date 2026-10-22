use crate::tui::App;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
};
use std::io::{self, Write};
use std::panic;

pub fn run(notes_path: std::path::PathBuf) -> Result<()> {
    eprintln!("[DEBUG] Starting Palace TUI");

    let db_path = notes_path.join(".palace-tui").join("database.db");

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    eprintln!("[DEBUG] Opening database at {}", db_path.display());
    let db = crate::storage::Database::new(&db_path)?;

    let existing_notes = db.count_notes().unwrap_or(0);
    eprintln!(
        "[DEBUG] Found {} existing notes in database",
        existing_notes
    );

    if existing_notes == 0 {
        eprintln!("[DEBUG] Indexing notes...");
        let ingester = crate::storage::Ingester::new(notes_path.clone(), db_path)?;
        let stats = ingester.index_all()?;
        println!(
            "Indexed {} notes with {} links",
            stats.total_notes, stats.total_links
        );
    } else {
        eprintln!("[DEBUG] Skipping indexing, using existing data");
        let links = db.get_all_note_links().unwrap_or_default();
        println!("Loaded {} notes with {} links", existing_notes, links.len());
    }

    eprintln!("[DEBUG] Building app...");
    let mut app = App::new(&db)?;

    let mut stdout = io::stdout();
    eprintln!("[DEBUG] Entering alternate screen...");
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    stdout.flush()?;
    enable_raw_mode()?;

    let original_panic = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), LeaveAlternateScreen);
        original_panic(info);
    }));

    eprintln!("[DEBUG] Starting event loop");

    loop {
        if app.should_quit {
            eprintln!("[DEBUG] Quit requested");
            break;
        }

        if let Err(e) = render_screen(&mut app, &mut stdout) {
            eprintln!("[ERROR] render_screen failed: {}", e);
            break;
        }
        let _ = stdout.flush();

        match event::poll(std::time::Duration::from_millis(100)) {
            Ok(true) => match event::read() {
                Ok(Event::Key(key)) => {
                    if key.code == KeyCode::Char('c')
                        && key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL)
                    {
                        eprintln!("[DEBUG] Ctrl+C pressed");
                        break;
                    }
                    app.handle_key_event(Event::Key(key));
                }
                Ok(ev) => {
                    eprintln!("[DEBUG] Non-key event: {:?}", ev);
                }
                Err(e) => {
                    eprintln!("[ERROR] event::read failed: {}", e);
                }
            },
            Ok(false) => {}
            Err(e) => {
                eprintln!("[ERROR] event::poll failed: {}", e);
            }
        }
    }

    let _ = disable_raw_mode();
    let _ = crossterm::execute!(stdout, LeaveAlternateScreen);
    eprintln!("[DEBUG] Exited Palace TUI");

    Ok(())
}

fn render_screen<W: Write>(app: &mut App, stdout: &mut W) -> Result<()> {
    let (width, height) = size()?;
    let width = width as usize;
    let height = height as usize;

    write!(
        stdout,
        "{}",
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )?;

    match app.mode {
        crate::tui::AppMode::Help => {
            let help = app.get_help_text();
            for (i, line) in help.iter().take(height).enumerate() {
                write!(stdout, "{}", crossterm::cursor::MoveTo(1, (i + 1) as u16))?;
                write!(stdout, "{}", line)?;
            }
        }
        _ => {
            let tree_height = height.saturating_sub(2);
            let tree_lines = app.render(width, tree_height);

            for (i, line) in tree_lines.iter().enumerate() {
                if i >= tree_height {
                    break;
                }
                write!(stdout, "{}", crossterm::cursor::MoveTo(1, (i + 1) as u16))?;
                write!(stdout, "{}", line)?;
            }

            let status = app.get_status_bar();
            write!(
                stdout,
                "{}",
                crossterm::cursor::MoveTo(0, tree_height as u16)
            )?;
            write!(stdout, "{}", status)?;

            write!(
                stdout,
                "{}",
                crossterm::cursor::MoveTo(0, (tree_height + 1) as u16)
            )?;
            for _ in 0..width.min(200) {
                write!(stdout, "─")?;
            }

            let hint = " ? Help  / Search  q Quit";
            write!(stdout, "{}", crossterm::cursor::MoveTo(1, height as u16))?;
            write!(stdout, "{}", hint)?;
        }
    }

    Ok(())
}
