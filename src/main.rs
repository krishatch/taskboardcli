use std::io::{self, stdout, Stdout};
use crossterm::{ event::{self, Event, KeyCode},
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

/* The Taskboard struct represents all of the information needed to render the application
* num_lists: usize - the current number of lists
* lists: Vec<TaskList> - A vector of all List structds.
*/
#[derive(Serialize, Deserialize, Clone)]
struct TaskBoard {
    num_lists: usize,
    lists: Vec<TaskList>,
    active_list: usize,
}

/*
* The TaskList is a list of all current tasks
* id: usize - A numeric id for the list
* title: String - Name of the list, e.g., ECE 339
* tasks: Vec<Task> - A vector of all Task structs contained in this TaskList
*/
#[derive(Serialize, Deserialize, Clone)]
struct TaskList {
    id: usize,
    title: String,
    tasks: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Task {
    title: String,
    due: String,
}

#[derive(Copy, Clone, Debug)]
enum MenuItem {
    Home,
}

impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0
        }
    }
}

fn main() -> io::Result<()> {
    /*** set up terminal ***/
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    /*** initialize taskboard and home ***/
    let mut active_menu_item = MenuItem::Home;
    let mut quit = false;
    let mut active_list_state = ListState::default();
    active_list_state.select(Some(0));
    let mut taskboard = TaskBoard{
        num_lists: read_db().expect("valid read").len(), 
        lists:read_db().expect("valid read"),
        active_list: 1,
    }; // Make a function that initialized the creation of the taskboard
    
    /*** main loop ***/
    while !quit {
        match active_menu_item{
            MenuItem::Home => {let _ = ui(&mut terminal, &mut taskboard);}
        }
        quit = handle_events(&mut active_menu_item, &mut taskboard)?;
    }

    let _ = write_db(&mut taskboard);
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn ui(terminal: &mut Terminal<CrosstermBackend<Stdout>>, taskboard: &mut TaskBoard) -> Result<u32, Error> {
    /*** Set up default layout ***/
    terminal.draw(|frame| {
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

        /*** Help menu ***/
        let help_info = get_helpline();
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

        /*** Main Taskboard ***/

        let mut task_list_state = ListState::default();
        match taskboard.num_lists{
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
                let mut constraints = vec![];
                for _i in 0..taskboard.num_lists{
                    constraints.push(Constraint::Min(0));
                }
                let home = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(constraints)
                    .split(chunks[1]);

                for (i, list) in taskboard.lists.clone().into_iter().enumerate(){
                    let taskboard_list = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(3),
                            Constraint::Min(2),
                        ])
                        .split(home[i]);
                    let title = Paragraph::new(list.title.clone())
                        .style(Style::default().fg(Color::Rgb(0xFF, 0xFF, 0xFF)))
                        .alignment(Alignment::Center)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .style(Style::default().fg(Color::Rgb(0xcc, 0x55, 0x00)))
                                .title(list.title.clone())
                                .border_type(BorderType::Plain),
                        );
                    let mut color = Color::Rgb(0xcc, 0x55, 0x00);
                    if list.id == taskboard.active_list{
                        color = Color::Yellow;
                    }
                    let list_out = List::new(list.tasks)
                            .block(Block::default().fg(color).title("List").borders(Borders::ALL))
                            .style(Style::default().fg(Color::Rgb(0xff, 0xff, 0xff)))
                            .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
                            .highlight_symbol(">>")
                            .repeat_highlight_symbol(true)
                            .direction(ListDirection::TopToBottom);
                    frame.render_stateful_widget(list_out, taskboard_list[1], &mut task_list_state);
                    frame.render_widget(title, taskboard_list[0]);
                }
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
    })?;
    Ok(0)
}

fn read_db() -> Result<Vec<TaskList>, Error> {
    let db_content = fs::read_to_string(DB_PATH)?;
    let parsed: Vec<TaskList> = match serde_json::from_str(&db_content){
        Ok(parsed) => parsed,
        Err(_err) => vec![],
    };
    Ok(parsed)
}

fn create_list() -> Result<Vec<TaskList>, Error>{
    let db_content = fs::read_to_string(DB_PATH)?;
    let mut parsed: Vec<TaskList> = match serde_json::from_str(&db_content){
        Ok(parsed) => parsed,
        Err(_err) => vec![],
    };

    let new_list = TaskList {
        id: parsed.len() + 1,
        title: format!("list {}", parsed.len() + 1),
        tasks: vec![
            "task 1".to_string(),
            "task 2".to_string(),
        ]
    };

    parsed.push(new_list);
    fs::write(DB_PATH, serde_json::to_vec(&parsed)?)?;
    Ok(parsed)
}

fn write_db(taskboard: &mut TaskBoard) -> Result<Vec<TaskList>, Error>{
    let tasklists = taskboard.lists.clone();
    fs::write(DB_PATH, serde_json::to_vec(&tasklists)?)?;
    Ok(tasklists)
}

fn delete_list() -> Result<Vec<TaskList>, Error> {
    let db_content = fs::read_to_string(DB_PATH)?;
    let mut parsed: Vec<TaskList> = match serde_json::from_str(&db_content){
        Ok(parsed) => parsed,
        Err(_err) => vec![],
    };
    match parsed.len(){
        0 => {}
        _ => {parsed.pop();}
    };

    fs::write(DB_PATH, serde_json::to_vec(&parsed)?)?;
    Ok(parsed)

}

// fn add_task(taskboard: &mut TaskBoard) {
// }

fn get_helpline() -> Line<'static>{
    Line::from(vec![
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
            "ross item - ",
            Style::default()
                .fg(Color::Rgb(0xff, 0xff, 0xff))
        ),
        Span::styled(
            "Q",
            Style::default()
                .fg(Color::Rgb(0xcc, 0x55, 0x00))
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled(
            "uit",
            Style::default()
                .fg(Color::Rgb(0xff, 0xff, 0xff))
        ),
        ]
    )
}

fn handle_events(active_menu_item: &mut MenuItem, taskboard: &mut TaskBoard) -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            match active_menu_item {
                MenuItem::Home => {
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Enter {
                        *active_menu_item = MenuItem::Home;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                        return Ok(true);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('n') {
                        taskboard.lists = create_list().expect("valid list add");
                        taskboard.num_lists = taskboard.lists.len();
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('a') {
                        taskboard.lists[taskboard.active_list - 1].tasks.push("new task".to_string());
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('c') {
                        taskboard.lists[taskboard.active_list - 1].tasks.pop();
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('d') {
                        taskboard.lists = delete_list().expect("valid remove");
                        taskboard.num_lists = taskboard.lists.len();
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && (key.code == KeyCode::Char('h') || key.code == KeyCode::Left) {
                        if taskboard.active_list > 1 {
                            taskboard.active_list -= 1;
                        }
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && (key.code == KeyCode::Char('l')  || key.code == KeyCode::Right) {
                        if taskboard.active_list < taskboard.num_lists {
                            taskboard.active_list += 1;
                        }
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('0') {
                        taskboard.active_list = 0;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('1') {
                        taskboard.active_list = 1;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('2') {
                        taskboard.active_list = 2;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('3') {
                        taskboard.active_list = 3;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('4') {
                        taskboard.active_list = 4;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('5') {
                        taskboard.active_list = 5;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('6') {
                        taskboard.active_list = 6;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('7') {
                        taskboard.active_list = 7;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('8') {
                        taskboard.active_list = 8;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('9') {
                        taskboard.active_list = 9;
                        return Ok(false);
                    }
                }
                _ => {
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Enter {
                        *active_menu_item = MenuItem::Home;
                        return Ok(false);
                    }
                }

            }
        }
    }
    Ok(false)
}

