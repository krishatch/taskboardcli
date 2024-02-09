use std::io::{self, stdout, Stdout};
use crossterm::{ event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};

/*** Taskboard specific includes ***/
// use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
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
    AddingList,
    AddingTask,
}

impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0,
            MenuItem::AddingList => 1,
            MenuItem::AddingTask => 2,
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
        let _ = ui(&mut terminal, &mut taskboard);
        quit = handle_events(&mut active_menu_item, &mut taskboard)?; }

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
                                .title("List ".to_owned() + &i.to_string())
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

fn create_list(taskboard: &mut TaskBoard) {
    let new_list = TaskList {
        id:  taskboard.num_lists + 1,
        title: "".to_string(),
        tasks: vec![],
    };

    taskboard.lists.push(new_list);
    taskboard.num_lists += 1;
}

fn write_db(taskboard: &mut TaskBoard) -> Result<Vec<TaskList>, Error>{
    let tasklists = taskboard.lists.clone();
    fs::write(DB_PATH, serde_json::to_vec(&tasklists)?)?;
    Ok(tasklists)
}

fn delete_list(taskboard: &mut TaskBoard) {
    match taskboard.num_lists {
        0 => {},
        _ => {
            taskboard.lists.pop();
            taskboard.num_lists -= 1;
        }
    }
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

/*** Key input handling ***/
fn handle_events(active_menu_item: &mut MenuItem, taskboard: &mut TaskBoard) -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            match active_menu_item {
                MenuItem::AddingTask => {
                    // make inputs change list name
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Enter {
                        *active_menu_item = MenuItem::Home;
                        return Ok(false);
                    }
                    if let KeyCode::Char(c) = key.code {
                        let tasks = &mut taskboard.lists[taskboard.active_list - 1].tasks;
                        let last_task_index = tasks.len() - 1;
                        if let Some(last_task) = tasks.get_mut(last_task_index) {
                            let mut new_task = last_task.clone();
                            new_task.push(c);
                            *last_task = new_task;
                            return Ok(false);
                        }
                    }
                }

                /*** Adding List ***/
                MenuItem::AddingList => {
                    if let KeyCode::Char(c) = key.code {
                        let title = &mut taskboard.lists[taskboard.num_lists - 1].title;
                        title.push(c); // Convert character to uppercase
                        return Ok(false);
                    } else if key.code == KeyCode::Enter {
                        *active_menu_item = MenuItem::Home;
                        return Ok(false);
                    }
                }
                /*** Home ***/
                MenuItem::Home => {
                    if let KeyCode::Char(c) = key.code {
                        match c {
                            'q' => return Ok(true),
                            'n' => {
                                create_list(taskboard);
                                *active_menu_item = MenuItem::AddingList;
                                return Ok(false);
                            }
                            'a' => {
                                if taskboard.num_lists > 0 {
                                    taskboard.lists[taskboard.active_list - 1].tasks.push("".to_string());
                                    *active_menu_item = MenuItem::AddingTask;
                                }
                                return Ok(false);
                            }
                            'c' => {
                                taskboard.lists[taskboard.active_list - 1].tasks.pop();
                                return Ok(false);
                            }
                            'd' => {
                                delete_list(taskboard);
                                taskboard.num_lists = taskboard.lists.len();
                                return Ok(false);
                            }
                            'h' | 'l' | '0'..='9' => {
                                let new_active_list = match c {
                                    'h' if taskboard.active_list > 1 => taskboard.active_list - 1,
                                    'l' if taskboard.active_list < taskboard.num_lists => taskboard.active_list + 1,
                                    '0'..='9' => c.to_digit(10).map(|n| n as usize).unwrap_or(0),
                                    _ => taskboard.active_list,
                                };
                                taskboard.active_list = new_active_list;
                                return Ok(false);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    Ok(false)
}
