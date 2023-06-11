mod window_service;
mod db_service;
mod settings_service;
use std::collections::{BTreeMap};

use chrono::{Local, Timelike, NaiveTime, NaiveDate};
use db_service::Window;
use settings_service::Settings;
use tokio::time::{self, Duration};

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

    println!(" {} | {}", "Time", "Titles");
    let mut ordered_projects = generate_project_hashmap();
    for window in windows.clone() {
        //how2format: parse_from_str("2014-5-17T12:34:56+09:30", "%Y-%m-%dT%H:%M:%S%z")
        //actual format: 2023-06-05 19:50:35.482704600 +02:00
        let start_time = NaiveTime::parse_from_str(&window.start_time, "%Y-%m-%d %H:%M:%S%.f %z").unwrap();
        let end_time = NaiveTime::parse_from_str(&window.end_time, "%Y-%m-%d %H:%M:%S%.f %z").unwrap();
        for (time, windows) in ordered_projects.iter_mut() {
            let time_from_midnight = time.num_seconds_from_midnight();
            if start_time.num_seconds_from_midnight() - 900 <= time_from_midnight && end_time.num_seconds_from_midnight() - 900 > time_from_midnight {
                windows.push(window.clone());
            }
        }
    }

    let mut previous_projects: Vec<Window> = ordered_projects.iter().next().unwrap().1.clone();
    for mut projects in ordered_projects {
        if projects.1.len() == 0 {
            // No current projects, nothing to print
            previous_projects = projects.1.clone();
            continue;
        }

        print!("{}", projects.0.format("%H:%M"));
        
        if previous_projects.len() == 0 {
            // Previous projects empty, just print current projects
            for window in projects.1.iter() {
                print!(" | {}", clean_window_title(window.title.clone()));
            }

            println!();
            previous_projects = projects.1.clone();
            continue;
        }

        // Both previous and current projects have entries, compare to keep the order
        for previous_project in previous_projects {
            if projects.1.iter().find(|w| w.title == previous_project.title).is_some() {
                print!(" | {}", clean_window_title(previous_project.title.clone()));
                projects.1.remove(projects.1.iter().position(|w| w.title == previous_project.title).unwrap());
            } else {
                print!(" | {}", clean_window_title(projects.1.remove(0).title.clone()));
            }
        }

        println!();
        previous_projects = projects.1.clone();
    }
}

fn clean_window_title(title: String) -> String {
    let cleaned_title = title.replace("|", "-").replace("｜", "-");
    // TODO: Make length configurable
    force_length(cleaned_title, 20)
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
    let settings = Settings::load_from_file();
    let mut interval = time::interval(Duration::from_secs(60));
    let mut previous_windows: Vec<Window> = Vec::new();

    loop {
        interval.tick().await;
        let now = Local::now();
        let minute = now.minute();
        if [5, 20, 35, 50].contains(&minute) {
            let windows = window_service::get_open_windows(settings.top_windows_to_save);
            
            let mut new_windows: Vec<Window> = Vec::new();
            for window in windows.clone() {
                let mut db_window = db_service::Window::new(
                    window.0.clone(),
                    now.to_string(),
                    (now + chrono::Duration::minutes(15)).to_string(),
                    None
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