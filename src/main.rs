use std::io::{self, stdout};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};

/*** Taskboard specific includes ***/
use chrono::prelude::*;
use rand::{distributions::Alphanumeric, prelude::*};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;

const DB_PATH: &str = "./data/lists.json";
/*** Error handling for db reading ***/
#[derive(Error, Debug)]
pub enum Error {
    #[error("error reading the DB file: {0}")]
    ReadDBError(#[from] io::Error),
    #[error("error parsing the DB file: {0}")]
    ParseDBError(#[from] serde_json::Error),
}

#[derive(Serialize, Deserialize, Clone)]
struct Task {
    title: String,
    due: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct TaskList {
    size: usize,
    name: String,
    tasks: Vec<Task>,
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut should_quit = false;
    while !should_quit {
        terminal.draw(ui)?;
        should_quit = handle_events()?;
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn handle_events() -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn ui(frame: &mut Frame) {
    /*** Set up default layout ***/
    let size = frame.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(2),
            Constraint::Length(3),
        ].as_ref(),
        )
        .split(size);

    /*** Main Taskboard ***/
    //let tasklists = read_db().expect("can fetch tasklist list");
    let listlen = 2;


    let help_info = Line::from(vec![
        Span::styled(
            "<num>",
            Style::default()
                .fg(Color::Rgb(0xcc, 0x55, 0x00))
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled(
            " Select List - ",
            Style::default()
                .fg(Color::Rgb(0xff, 0xff, 0xff))
        ),
        Span::styled(
            "N",
            Style::default()
                .fg(Color::Rgb(0xcc, 0x55, 0x00))
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled(
            "ew List - ",
            Style::default()
                .fg(Color::Rgb(0xff, 0xff, 0xff))
        ),
        Span::styled(
            "D",
            Style::default()
                .fg(Color::Rgb(0xcc, 0x55, 0x00))
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled(
            "elete List - ",
            Style::default()
                .fg(Color::Rgb(0xff, 0xff, 0xff))
        ),
        Span::styled(
            "A",
            Style::default()
                .fg(Color::Rgb(0xcc, 0x55, 0x00))
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled(
            "dd item - ",
            Style::default()
                .fg(Color::Rgb(0xff, 0xff, 0xff))
        ),
        Span::styled(
            "C",
            Style::default()
                .fg(Color::Rgb(0xcc, 0x55, 0x00))
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled(
            "ross item",
            Style::default()
                .fg(Color::Rgb(0xff, 0xff, 0xff))
        ),
        ]
    );

    let help = Paragraph::new(help_info.clone())
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Rgb(0xcc, 0x55, 0x00)))
                .title("Commands")
                .border_type(BorderType::Plain),
            );
    frame.render_widget(help, chunks[0]);
    match listlen{
        0 => {
            let taskboard = Paragraph::new("No Lists")
                .style(Style::default().fg(Color::Rgb(0xFF, 0xFF, 0xFF)))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::Rgb(0xcc, 0x55, 0x00)))
                        .title("Taskboard")
                        .border_type(BorderType::Plain),
                );
            frame.render_widget(taskboard, chunks[1]);
        }
        _=>{
            let mut lists = vec![];
            for _i in 0..2{
                lists.push(Constraint::Min(0));
            }
            let taskboard = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(lists)
                .split(chunks[1]);

            let left = Paragraph::new("List 1")
                .style(Style::default().fg(Color::Rgb(0xFF, 0xFF, 0xFF)))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::Rgb(0xcc, 0x55, 0x00)))
                        .title("Taskboard")
                        .border_type(BorderType::Plain),
                );

            let right = Paragraph::new("List 2")
                .style(Style::default().fg(Color::Rgb(0xFF, 0xFF, 0xFF)))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::Rgb(0xcc, 0x55, 0x00)))
                        .title("Taskboard")
                        .border_type(BorderType::Plain),
                );
            frame.render_widget(left, taskboard[0]);
            frame.render_widget(right, taskboard[1]);
        }
    }
    let copyright = Paragraph::new("taskboardcli 2024 - all rights reserved")
        .style(Style::default().fg(Color::Rgb(0xFF, 0xFF, 0xFF)))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Rgb(0xcc, 0x55, 0x00)))
                .title("Copyright")
                .border_type(BorderType::Plain),
        );

    /*** Render widgets ***/
    frame.render_widget(copyright, chunks[2]);
}

// fn render_taskboard<'a>(tasklist_list_state: &ListState) -> Paragraph<'a>{
//
// }

fn read_db() -> Result<Vec<TaskList>, Error> {
    let db_content = fs::read_to_string(DB_PATH)?;
    let parsed: Vec<TaskList> = serde_json::from_str(&db_content)?;
    Ok(parsed)
}
