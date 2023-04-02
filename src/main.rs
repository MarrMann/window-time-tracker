mod window_service;
mod db_service;
use chrono::{Local, Timelike};
use tokio::time::{self, Duration};

#[tokio::main]
async fn main() {
    let mut interval = time::interval(Duration::from_secs(60));
    match db_service::create_database() {
        Ok(_) => println!("Databases available"),
        Err(e) => println!("Error creating database: {}", e)
    }

    loop {
        interval.tick().await;
        let now = Local::now();
        let minute = now.minute();
        if [5, 20, 35, 50].contains(&minute) {
            let windows = window_service::get_open_windows();
        
            for window in windows {
                let db_window = db_service::Window::new(
                    window.0.clone(),
                    now.to_string(),
                    (now + chrono::Duration::minutes(15)).to_string()
                );
                match db_service::create_or_update_entry(db_window) {
                    Ok(_) => println!("Entry created"),
                    Err(e) => println!("Error creating entry with title {}", e)
                }

                println!("Window title: {}", window.0);
            }
        }
    }
}