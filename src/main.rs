use std::io::{self, stdout, Stdout};
use std::env;
use std::path::PathBuf;
use crossterm::{ event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{prelude::*, widgets::*};

/*** Taskboard specific includes ***/
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use thiserror::Error;

const DEBUG: bool = true;
const COLOR1: Color = Color::White;
const COLOR2: Color = Color::Rgb(0xff, 0xff, 0xff);
const COLOR3: Color = Color::Yellow;
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
    debug_str: String,
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
    tasks: Vec<Task>,
    selected: usize,
}

#[derive(Serialize, Deserialize, Clone)]
struct Task {
    title: String,
    date_string: String,
    due: NaiveDate,
}

impl From<Task> for Text<'static> {
    fn from(task: Task) -> Self {
        Text::raw(format!("{} - {}", task.title, task.date_string))
    }
}

#[derive(Copy, Clone, Debug)]
enum MenuItem {
    Home,
    AddingList,
    AddingTaskTitle,
    AddingTaskDate,
}

impl From<MenuItem> for usize {
    fn from(input: MenuItem) -> usize {
        match input {
            MenuItem::Home => 0,
            MenuItem::AddingList => 1,
            MenuItem::AddingTaskTitle => 2,
            MenuItem::AddingTaskDate => 3,
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
        debug_str: String::from(""),
    }; // Make a function that initialized the creation of the taskboard
    
    /*** main loop ***/
    while !quit {
        let _ = ui(&mut terminal, &mut taskboard, &mut active_menu_item);
        quit = handle_events(&mut active_menu_item, &mut taskboard)?; 
        update_dates(&mut taskboard);
    }

    let _ = write_db(&mut taskboard);
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn ui(terminal: &mut Terminal<CrosstermBackend<Stdout>>, taskboard: &mut TaskBoard, active_menu_item: &mut MenuItem) -> Result<u32, Error> {
    /*** Set up default layout ***/
    terminal.draw(|frame| {
        let size = frame.size();
        let mut constraints = vec![
                Constraint::Length(3),
                Constraint::Min(2),
        ];
        if DEBUG {
            constraints.push(Constraint::Length(3));
        }
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints(&constraints[..])
            .split(size);

        /*** Help menu ***/
        let help_info = get_helpline();
        let help = Paragraph::new(help_info.clone()) .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(COLOR1))
                    .title("Commands")
                    .border_type(BorderType::Plain),
                );
        frame.render_widget(help, chunks[0]);

        /*** Main Taskboard ***/
        match taskboard.num_lists{
            0 => {
                let taskboard = Paragraph::new("No Lists")
                    .style(Style::default().fg(COLOR2))
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .style(Style::default().fg(COLOR1))
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

                let mut task_list_state = ListState::default().with_selected(Some(taskboard.lists[taskboard.active_list - 1].selected));
                for (i, list) in taskboard.lists.clone().into_iter().enumerate(){
                    let taskboard_list = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(3),
                            Constraint::Min(2),
                        ])
                        .split(home[i]);
                    let color = match active_menu_item {
                        MenuItem::AddingList => {
                            if list.id == taskboard.active_list{
                                COLOR3
                            } else {
                                COLOR1
                            }
                        }
                        _ => COLOR1,
                    };
                    let title = Paragraph::new(list.title.clone())
                        .style(Style::default().fg(COLOR2))
                        .alignment(Alignment::Center)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .style(Style::default().fg(color))
                                .title("List ".to_owned() + &(i + 1).to_string())
                                .border_type(BorderType::Plain),
                        );

                    let color = match active_menu_item {
                        MenuItem::Home => {
                            if list.id == taskboard.active_list {
                                COLOR3
                            } else {
                                COLOR1
                            }
                        },
                        MenuItem::AddingList => COLOR1,
                        _ => 
                            if list.id == taskboard.active_list {
                                COLOR3
                            } else {
                                COLOR1
                            }
                    };
                    let empty = list.tasks.is_empty();
                    let list_out = List::new(list.tasks)
                            .block(Block::default().fg(color).title("List").borders(Borders::ALL))
                            .style(Style::default().fg(COLOR2))
                            .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
                            .highlight_symbol(">>")
                            .repeat_highlight_symbol(true)
                            .direction(ListDirection::TopToBottom);
                    if list.id == taskboard.active_list && !empty{
                        frame.render_stateful_widget(list_out, taskboard_list[1], &mut task_list_state);
                    } else {
                        frame.render_widget(list_out, taskboard_list[1]);
                    }
                    frame.render_widget(title, taskboard_list[0]);
                }
            }
        }

        /*** Debug ***/
        let copyright = Paragraph::new(taskboard.debug_str.clone())
            .style(Style::default().fg(COLOR2))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(COLOR1))
                    .title("DEBUG")
                    .border_type(BorderType::Plain),
            );

        /*** Render widgets ***/
        frame.render_widget(copyright, chunks[2]);
    })?;
    Ok(0)
}

fn get_db_path() -> Result<PathBuf, Error> {
    let bin_path = env::current_exe().unwrap();
    let mut db_path = bin_path.clone();
    db_path.pop();
    db_path.pop();
    db_path.pop();
    db_path.push("data/");
    if !db_path.exists(){
        let _ = fs::create_dir(db_path.clone());
        db_path.push("lists.json");
        let _ = OpenOptions::new().truncate(true).create(true).write(true).open(db_path.clone());
    } else {
        db_path.push("lists.json");
    }
    Ok(db_path)
}

fn read_db() -> Result<Vec<TaskList>, Error> {
    let db_path = get_db_path()?;
    print!("{}", db_path.display());
    let db_content = fs::read_to_string(db_path)?;
    let parsed: Vec<TaskList> = match serde_json::from_str(&db_content){
        Ok(parsed) => parsed,
        Err(_err) => vec![],
    };
    Ok(parsed)
}

fn create_list(taskboard: &mut TaskBoard) {
    let new_list = TaskList {
        id:  taskboard.num_lists + 1,
        title: String::from("|"),
        tasks: vec![],
        selected: 0,
    };

    taskboard.lists.push(new_list);
    taskboard.num_lists += 1;
}

fn write_db(taskboard: &mut TaskBoard) -> Result<Vec<TaskList>, Error>{
    let tasklists = taskboard.lists.clone();
    let db_path = get_db_path()?;
    fs::write(db_path, serde_json::to_vec(&tasklists)?)?;
    Ok(tasklists)
}

fn delete_list(taskboard: &mut TaskBoard) {
    match taskboard.num_lists {
        0 => {},
        _ => {
            taskboard.lists.remove(taskboard.active_list - 1);
            taskboard.num_lists -= 1;
        }
    }
}

fn update_dates(taskboard: &mut TaskBoard){
    // Update strings 
    for list in taskboard.lists.iter_mut(){
        for task in list.tasks.iter_mut() {
            let due_diff = NaiveDateTime::new(task.due, NaiveTime::from_hms_opt(0, 0, 0).unwrap()) - NaiveDateTime::new(Local::now().naive_local().date(), NaiveTime::from_hms_opt(0, 0, 0).unwrap());
            match due_diff.num_days() {
                0 => task.date_string = "Today".to_string(),
                1 => task.date_string= "Tomorrow".to_string(),
                2.. => {},
                _ => task.date_string = "Overdue".to_string(),
            }
            
        }
    }

    // Sort tasks by due date
    for list in taskboard.lists.iter_mut() {
        list.tasks.sort_by(|a, b| a.due.cmp(&b.due));
    }
}
fn get_helpline() -> Line<'static>{
    Line::from(vec![
        Span::styled(
            "<num>",
            Style::default()
                .fg(COLOR1)
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled(
            " Select List - ",
            Style::default()
                .fg(COLOR2)
        ),
        Span::styled(
            "N",
            Style::default()
                .fg(COLOR1)
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled(
            "ew List - ",
            Style::default()
                .fg(COLOR2)
        ),
        Span::styled(
            "D",
            Style::default()
                .fg(COLOR1)
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled(
            "elete List - ",
            Style::default()
                .fg(COLOR2)
        ),
        Span::styled(
            "A",
            Style::default()
                .fg(COLOR1)
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled(
            "dd item - ",
            Style::default()
                .fg(COLOR2)
        ),
        Span::styled(
            "d",
            Style::default()
                .fg(COLOR1)
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled(
            "elete item - ",
            Style::default()
                .fg(COLOR2)
        ),
        Span::styled(
            "Q",
            Style::default()
                .fg(COLOR1)
                .add_modifier(Modifier::UNDERLINED),
        ),
        Span::styled(
            "uit",
            Style::default()
                .fg(COLOR2)
        ),
        ]
    )
}

/*** Key input handling ***/
fn handle_events(active_menu_item: &mut MenuItem, taskboard: &mut TaskBoard) -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            match active_menu_item {

                /*** Adding task date ***/
                MenuItem::AddingTaskDate => {
                    // make inputs change list name
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Enter {
                        *active_menu_item = MenuItem::Home;
                        let tasks = &mut taskboard.lists[taskboard.active_list - 1].tasks;
                        let last_task_index = tasks.len() - 1;
                        if let Some(last_task) = tasks.get_mut(last_task_index) {
                            let mut new_task = last_task.clone();
                            new_task.date_string.pop();
                            if let Ok(due_date) = NaiveDate::parse_from_str(&new_task.date_string, "%Y/%m/%d") {
                                new_task.due = due_date;
                            } else {
                                taskboard.debug_str = format!("Failed to parse date: {}", new_task.date_string);
                            }
                            *last_task = new_task;
                            return Ok(false);
                        }
                    }
                    if let KeyCode::Char(c) = key.code {
                        let tasks = &mut taskboard.lists[taskboard.active_list - 1].tasks;
                        let last_task_index = tasks.len() - 1;
                        if let Some(last_task) = tasks.get_mut(last_task_index) {
                            let mut new_task = last_task.clone();
                            new_task.date_string.insert(new_task.date_string.len() - 1 ,c);
                            *last_task = new_task;
                            return Ok(false);
                        }
                    }
                    if key.code == KeyCode::Backspace{
                        let tasks = &mut taskboard.lists[taskboard.active_list - 1].tasks;
                        let last_task_index = tasks.len() - 1;
                        if let Some(last_task) = tasks.get_mut(last_task_index) {
                            let mut new_task = last_task.clone();
                            if new_task.date_string.len() != 1{
                                new_task.date_string.remove(new_task.date_string.len() - 2);
                            }
                            *last_task = new_task;
                            return Ok(false);
                        }
                    }
                    if key.code == KeyCode::Esc{
                        taskboard.lists[taskboard.active_list - 1].tasks.pop();
                        taskboard.lists[taskboard.active_list - 1].selected = match taskboard.lists[taskboard.active_list - 1].tasks.len(){
                            0 => 0,
                            _=> taskboard.lists[taskboard.active_list - 1].tasks.len() - 1,
                        };
                        *active_menu_item = MenuItem::Home;
                    }
                }

                /*** Adding Task ***/
                MenuItem::AddingTaskTitle => {
                    // make inputs change list name
                    if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Enter {
                        *active_menu_item = MenuItem::AddingTaskDate;
                        let tasks = &mut taskboard.lists[taskboard.active_list - 1].tasks;
                        let last_task_index = tasks.len() - 1;
                        if let Some(last_task) = tasks.get_mut(last_task_index) {
                            let mut new_task = last_task.clone();
                            new_task.title.pop();
                            *last_task = new_task;
                            return Ok(false);
                        }
                    }
                    if let KeyCode::Char(c) = key.code {
                        let tasks = &mut taskboard.lists[taskboard.active_list - 1].tasks;
                        let last_task_index = tasks.len() - 1;
                        if let Some(last_task) = tasks.get_mut(last_task_index) {
                            let mut new_task = last_task.clone();
                            new_task.title.insert(new_task.title.len() - 1 ,c);
                            *last_task = new_task;
                            return Ok(false);
                        }
                    }
                    if key.code == KeyCode::Backspace{
                        let tasks = &mut taskboard.lists[taskboard.active_list - 1].tasks;
                        let last_task_index = tasks.len() - 1;
                        if let Some(last_task) = tasks.get_mut(last_task_index) {
                            let mut new_task = last_task.clone();
                            if new_task.title.len() != 1{
                                new_task.title.remove(new_task.title.len() - 2);
                            }
                            *last_task = new_task;
                            return Ok(false);
                        }
                    }
                    if key.code == KeyCode::Esc{
                        taskboard.lists[taskboard.active_list - 1].tasks.pop();
                        taskboard.lists[taskboard.active_list - 1].selected = match taskboard.lists[taskboard.active_list - 1].tasks.len(){
                            0 => 0,
                            _=> taskboard.lists[taskboard.active_list - 1].tasks.len() - 1,
                        };
                        *active_menu_item = MenuItem::Home;
                    }
                }

                /*** Adding List ***/
                MenuItem::AddingList => {
                    if let KeyCode::Char(c) = key.code {
                        let title = &mut taskboard.lists[taskboard.num_lists - 1].title;
                        title.insert(title.len() - 1, c); 
                        return Ok(false);
                    } else if key.code == KeyCode::Enter {
                        let title = &mut taskboard.lists[taskboard.num_lists - 1].title;
                        title.pop();
                        *active_menu_item = MenuItem::Home;
                        return Ok(false);
                    } else if key.code == KeyCode::Backspace{
                        let title = &mut taskboard.lists[taskboard.num_lists - 1].title;
                        if title.len() != 1{
                            title.remove(title.len() - 2);
                        }
                    }else if key.code == KeyCode::Esc {
                        taskboard.lists.pop();
                        taskboard.num_lists -= 1;
                        taskboard.active_list -= 1;
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
                                taskboard.active_list = taskboard.lists.len();
                                *active_menu_item = MenuItem::AddingList;
                                return Ok(false);
                            }
                            'a' => {
                                if taskboard.num_lists > 0 {
                                    taskboard.lists[taskboard.active_list - 1].tasks.push(Task{title: String::from("|"), due: NaiveDate::from_ymd_opt(2102, 12, 1).unwrap(), date_string: String::from("|")});
                                    taskboard.lists[taskboard.active_list - 1].selected = taskboard.lists[taskboard.active_list - 1].tasks.len() - 1;
                                    *active_menu_item = MenuItem::AddingTaskTitle;
                                }
                                return Ok(false);
                            }
                            'd' => {
                                let active_list_index = taskboard.active_list - 1;
                                let active_list = &mut taskboard.lists[active_list_index];
                                if active_list.tasks.is_empty(){
                                    return Ok(false);
                                }
                                let selected_task_index = active_list.selected;
                                active_list.tasks.remove(selected_task_index);
                                let new_selected = match selected_task_index {
                                    0 => 0,
                                    len if len == active_list.tasks.len() => len - 1,
                                    other => other,
                                };
                                active_list.selected = new_selected;
                                return Ok(false);
                            }
                            'D' => {
                                delete_list(taskboard);
                                taskboard.num_lists = taskboard.lists.len();
                                let new_active_list = match taskboard.active_list {
                                    1 => 1,
                                    _=> taskboard.active_list - 1,
                                };
                                for (i, list) in taskboard.lists.iter_mut().enumerate() {
                                    list.id = i + 1;
                                }
                                taskboard.active_list = new_active_list;
                                return Ok(false);
                            }
                            'j' => if taskboard.lists[taskboard.active_list - 1].selected + 1< taskboard.lists[taskboard.active_list - 1].tasks.len(){
                                taskboard.lists[taskboard.active_list - 1].selected += 1;
                            }
                            'k' => if taskboard.lists[taskboard.active_list - 1].selected > 0{
                                taskboard.lists[taskboard.active_list - 1].selected -= 1;
                            }
                            'h' | 'l' | '0'..='9' => {
                                let new_active_list = match c {
                                    'h' if taskboard.active_list > 1 => taskboard.active_list - 1,
                                    'l' if taskboard.active_list < taskboard.num_lists => taskboard.active_list + 1,
                                    '0'..='9' => c.to_digit(10).map(|n| n as usize).unwrap_or(0),
                                    _ => taskboard.active_list,
                                };
                                taskboard.active_list = new_active_list;
                                taskboard.lists[taskboard.active_list - 1].selected = 0;
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
