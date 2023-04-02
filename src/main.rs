mod window_service;
mod db_service;
use chrono::{Local, Timelike};
use db_service::Window;
use tokio::time::{self, Duration};

#[tokio::main]
async fn main() {
    let mut interval = time::interval(Duration::from_secs(60));
    match db_service::create_database() {
        Ok(_) => println!("Databases available"),
        Err(e) => println!("Error creating database: {}", e)
    }

    let mut previous_windows: Vec<Window> = Vec::new();

    loop {
        interval.tick().await;
        let now = Local::now();
        let minute = now.minute();
        if [5, 20, 35, 50].contains(&minute) {
            let windows = window_service::get_open_windows();
            
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