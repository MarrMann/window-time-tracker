mod window_service;
mod db_service;
mod settings_service;
use std::collections::{BTreeMap};

use chrono::{Local, Timelike, NaiveTime};
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
                query_today()
            } 
            _ => {
                println!("Incorrect argument, assuming --query");
                query_today()
            }
        }
    }
    else {
        println!("No argument, assuming --query");
        query_today()
    }
}

fn query_today() {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    print!("Querying {}", today);
    let windows = db_service::get_entries_on_date(today).unwrap();
    println!("Found {} entries", windows.len());

    let mut ordered_projects = generate_project_hashmap();
    for window in windows.clone() {
        let start_time_vec = &window.start_time.split_whitespace().collect::<Vec<&str>>();
        let start_time = NaiveTime::parse_from_str(start_time_vec[1], "%H:%M:%S%.f").unwrap();
        let end_time_vec = &window.end_time.split_whitespace().collect::<Vec<&str>>();
        let end_time = NaiveTime::parse_from_str(end_time_vec[1], "%H:%M:%S%.f").unwrap();
        for (time, windows) in ordered_projects.iter_mut() {
          println!("Checking {}", time);
          if start_time >= *time && end_time < *time {
              windows.push(window.clone());
          }
        }
    }

    for projects in ordered_projects {
        print!("{}: {}", projects.0, projects.1.len());
        for window in projects.1 {
            print!(" | {}", window.title);
        }
        println!();
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