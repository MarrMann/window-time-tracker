mod window_service;
mod db_service;
mod settings_service;
use std::collections::{BTreeMap};

use chrono::{Local, Timelike, NaiveTime, NaiveDate};
use db_service::Window;
use settings_service::Settings;
use tokio::time::{self, Duration};

pub mod colors{
  pub const RED: &str = "\x1b[31m";
  pub const GREEN: &str = "\x1b[32m";
  pub const YELLOW: &str = "\x1b[33m";
  pub const BLUE: &str = "\x1b[34m";
  pub const MAGENTA: &str = "\x1b[35m";
  pub const CYAN: &str = "\x1b[36m";
  pub const WHITE: &str = "\x1b[37m";
}

#[tokio::main]
async fn main() {
    match db_service::create_database() {
        Ok(_) => println!("Database available"),
        Err(e) => println!("Error creating database: {}", e)
    }

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "-r" | "--run" => {
                println!("Running window loop");
                tokio::spawn(get_windows_loop()).await.unwrap()
            }
            "-q" | "--query" => {
                if args.len() > 2 && args[2].len() == 10 {
                    let date = NaiveDate::parse_from_str(&args[2], "%Y-%m-%d").unwrap_or_else(|_|{
                        println!("Incorrect date format, assuming today");
                        Local::now().date_naive()
                    });
                    query_date(Some(date))
                }
                else {
                  query_date(None)
                }
            } 
            _ => {
                println!("Incorrect argument, assuming --run");
                println!("Running window loop");
                tokio::spawn(get_windows_loop()).await.unwrap()
            }
        }
    }
    else {
        println!("No argument, assuming --run");
        println!("Running window loop");
        tokio::spawn(get_windows_loop()).await.unwrap()
    }
}

fn query_date(date: Option<NaiveDate>) {
    let date_string = match date {
        Some(d) => d.format("%Y-%m-%d").to_string(),
        None => Local::now().format("%Y-%m-%d").to_string()
    };
    println!("Querying date {}", date_string);
    let windows = db_service::get_entries_on_date(date_string).unwrap();
    println!("Found {} entries\n", windows.len());
    if windows.len() == 0 {
        return;
    }

    // Projects ordered by time
    let mut ordered_projects = generate_project_hashmap();
    for window in windows.clone() {
        //how2format: parse_from_str("2014-5-17T12:34:56+09:30", "%Y-%m-%dT%H:%M:%S%z")
        //actual format: 2023-06-05 19:50:35.482704600 +02:00
        let start_time = NaiveTime::parse_from_str(&window.start_time, "%Y-%m-%d %H:%M:%S%.f %z").unwrap();
        let end_time = NaiveTime::parse_from_str(&window.end_time, "%Y-%m-%d %H:%M:%S%.f %z").unwrap();
        for (time, ordered_windows) in ordered_projects.iter_mut() {
            let time_from_midnight = time.num_seconds_from_midnight();
            if start_time.num_seconds_from_midnight() - 900 <= time_from_midnight && end_time.num_seconds_from_midnight() - 900 > time_from_midnight {
                ordered_windows.push(window.clone());
            }
        }
    }
    
    println!(" {} | {}", "Time", "Titles");
    let mut previous_projects: Vec<Window> = ordered_projects.iter().next().unwrap().1.clone();
    for projects in ordered_projects {
        if projects.1.len() == 0 {
            // No current projects, nothing to print
            previous_projects = projects.1.clone();
            continue;
        }

        print!("{}", projects.0.format("%H:%M"));
        
        let mut new_previous_projects: Vec<Window> = Vec::new();
        let mut removable_projects = projects.1.clone();
        // Both previous and current projects could have entries, compare to keep the order
        for previous_project in previous_projects {
            let printed_project: Window = if removable_projects.iter().find(|w| w.title == previous_project.title).is_some() {
                removable_projects.remove(removable_projects.iter().position(|w| w.title == previous_project.title)
                    .expect("There was a match for the previous project, so there should be here too since the match is the same"));
                previous_project.clone()
            } else {
                if removable_projects.len() == 0 {
                    break;
                }
                removable_projects.remove(0)
            };
            new_previous_projects.push(printed_project.clone());
            print!(" | {}", format_window_title(printed_project.title.clone(), printed_project.category));
        }

        // Print the rest, or if there were no previous projects, print current ones
        for window in removable_projects.iter() {
            new_previous_projects.push(window.clone());
            print!(" | {}", format_window_title(window.title.clone(), window.category));
        }

        println!();
        previous_projects = new_previous_projects;
    }
}

fn format_window_title(title: String, category: Option<u8>) -> String {
    let settings = Settings::load_from_file();
    let mut cleaned_title = title.replace("|", "-").replace("｜", "-");
    cleaned_title = force_length(cleaned_title, settings.window_title_length.try_into().expect("u32 should always fit usize "));

    // TODO: Change this to a lookup table
    match category {
        Some(c) => {
            let color = match c {
                0 => colors::RED,
                1 => colors::GREEN,
                2 => colors::YELLOW,
                3 => colors::BLUE,
                4 => colors::MAGENTA,
                5 => colors::CYAN,
                6 => colors::WHITE,
                _ => colors::WHITE
            };
            format!("{}{}\x1b[0m", color, cleaned_title)
        },
        None => cleaned_title
    }
}

fn force_length(string: String, length: usize) -> String {
    if string.chars().count() > length {
        string.chars().take(length).collect::<String>()
    } 
    else if string.chars().count() < length {
        let mut new_string = string.clone();
        for _ in 0..length - string.chars().count() {
            new_string.push(' ');
        }
        new_string
    }
    else {
        string
    }
}

fn generate_project_hashmap() -> BTreeMap<NaiveTime, Vec<Window>> {
    let settings = Settings::load_from_file();
    let mut ordered_projects: BTreeMap<NaiveTime, Vec<Window>> = BTreeMap::new();
    for hour in 0..24 {
        for minute in settings.minutes_to_save.clone() {
            let time = NaiveTime::from_hms_opt(hour, minute, 0).unwrap();
            ordered_projects.insert(time, Vec::new());
        }
    }

    ordered_projects
}

async fn get_windows_loop() {
    let mut interval = time::interval(Duration::from_secs(60));
    let mut previous_windows: Vec<Window> = Vec::new();

    loop {
        interval.tick().await;
        let now = Local::now();
        let minute = now.minute();
        if [5, 20, 35, 50].contains(&minute) {
            let settings = Settings::load_from_file(); // Reload settings every loop in case they have changed
            let windows = window_service::get_open_windows(settings.top_windows_to_save);
            
            let mut new_windows: Vec<Window> = Vec::new();
            for window in windows.clone() {
                let mut category: Option<u8> = None;
                settings.projects.iter().find(|p| p.keywords.iter().any(|k| window.0.to_lowercase().contains(&k.to_lowercase()))).map(|p| {
                    category = Some(p.category.clone());
                });

                let mut db_window = db_service::Window::new(
                    window.0.clone(),
                    now.to_string(),
                    (now + chrono::Duration::minutes(15)).to_string(),
                    category
                );
                previous_windows.iter().find(|w| w.title == window.0).map(|w| {
                    db_window.start_time = w.start_time.clone();
                });

                match db_service::create_or_update_entry(db_window.clone()) {
                    Ok(_) => {
                        println!("Entry created");
                        new_windows.push(db_window);
                    },
                    Err(e) => println!("Error creating entry with title {}", e)
                }
                
                println!("Window title: {}", window.0);
            }

            previous_windows = new_windows;
        }
    }
}