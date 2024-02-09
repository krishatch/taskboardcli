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

fn create_list() -> Result<Vec<TaskList>, Error>{
    let db_content = fs::read_to_string(DB_PATH)?;
    let mut parsed: Vec<TaskList> = match serde_json::from_str(&db_content){
        Ok(parsed) => parsed,
        Err(_err) => vec![],
    };

    let new_list = TaskList {
        id: parsed.len() + 1,
        title: "".to_string(),
        tasks: vec![],
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
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('A'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone() + "A";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('a'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone() + "a";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('B'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "B";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('b'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "b";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('C'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "C";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('c'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "c";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('D'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "D";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('d'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "d";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('E'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "E";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('e'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "e";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('F'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "F";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('f'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "f";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('G'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "G";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('g'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "g";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('H'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "H";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('h'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "h";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('I'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "I";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('i'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "i";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('J'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "J";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('j'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "j";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('K'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "K";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('k'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "k";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('L'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "L";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('l'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "l";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('M'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "M";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('m'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "m";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('N'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "N";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('n'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "n";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('O'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "O";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('o'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "o";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('P'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "P";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('p'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "p";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('Q'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "Q";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "q";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('R'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "R";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('r'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "r";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('S'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "S";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('s'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "s";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('T'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "T";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('t'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "t";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('U'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "U";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('u'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "u";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('V'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "V";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('v'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "v";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('W'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "W";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('w'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "w";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('X'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "X";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('x'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "x";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('Y'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "Y";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('y'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "y";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('Z'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "Z";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('z'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "z";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('0'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "0";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('1'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "1";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('2'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "2";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('3'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "3";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('4'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "4";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('5'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "5";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('6'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "6";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('7'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "7";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('8'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "8";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('9'){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ "9";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char(' '){
                        let tasks_size = taskboard.lists[taskboard.active_list - 1].tasks.len();
                        taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1] = taskboard.lists[taskboard.active_list - 1].tasks[tasks_size - 1].clone()+ " ";
                        return Ok(false);
                    }
                }

                /*** Adding List ***/
                MenuItem::AddingList => {
                    // make inputs change list name
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Enter {
                        *active_menu_item = MenuItem::Home;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('A'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "A";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('a'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "a";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('B'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "B";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('b'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "b";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('C'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "C";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('c'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "c";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('D'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "D";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('d'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "d";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('E'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "E";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('e'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "e";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('F'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "F";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('f'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "f";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('G'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "G";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('g'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "g";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('H'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "H";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('h'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "h";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('I'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "I";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('i'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "i";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('J'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "J";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('j'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "j";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('K'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "K";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('k'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "k";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('L'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "L";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('l'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "l";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('M'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "M";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('m'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "m";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('N'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "N";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('n'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "n";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('O'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "O";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('o'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "o";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('P'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "P";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('p'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "p";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('Q'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "Q";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "q";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('R'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "R";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('r'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "r";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('S'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "S";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('s'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "s";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('T'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "T";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('t'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "t";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('U'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "U";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('u'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "u";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('V'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "V";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('v'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "v";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('W'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "W";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('w'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "w";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('X'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "X";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('x'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "x";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('Y'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "Y";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('y'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "y";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('Z'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "Z";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('z'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "z";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('0'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "0";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('1'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "1";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('2'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "2";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('3'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "3";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('4'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "4";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('5'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "5";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('6'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "6";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('7'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "7";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('8'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "8";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('9'){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + "9";
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char(' '){
                        taskboard.lists[taskboard.num_lists - 1].title = taskboard.lists[taskboard.num_lists - 1].title.clone() + " ";
                        return Ok(false);
                    }
                }

                /*** Home ***/
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
                        *active_menu_item = MenuItem::AddingList;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('a') {
                        match taskboard.num_lists {
                            0 => {return Ok(false)},
                            _ => {
                                taskboard.lists[taskboard.active_list - 1].tasks.push("".to_string());
                                *active_menu_item = MenuItem::AddingTask;
                                return Ok(false);
                            }
                        }

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
                        if taskboard.active_list  > 1 {
                            taskboard.active_list  -= 1;
                        }
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && (key.code == KeyCode::Char('l')  || key.code == KeyCode::Right) {
                        if taskboard.active_list  < taskboard.num_lists {
                            taskboard.active_list  += 1;
                        }
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('0') {
                        taskboard.active_list  = 0;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('1') {
                        taskboard.active_list  = 1;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('2') {
                        taskboard.active_list  = 2;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('3') {
                        taskboard.active_list  = 3;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('4') {
                        taskboard.active_list  = 4;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('5') {
                        taskboard.active_list  = 5;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('6') {
                        taskboard.active_list  = 6;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('7') {
                        taskboard.active_list  = 7;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('8') {
                        taskboard.active_list  = 8;
                        return Ok(false);
                    }
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('9') {
                        taskboard.active_list  = 9;
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
