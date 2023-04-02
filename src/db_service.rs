use rusqlite::{Connection, Result};

pub struct Window {
    pub id: i32,
    pub title: String,
    pub start_time: String,
    pub end_time: String
}

impl Window {
    pub fn new(title: String, start_time: String, end_time: String) -> Window {
        Window {
            id: 0,
            title,
            start_time,
            end_time
        }
    }
}

pub fn create_database() -> Result<()> {
    let conn = Connection::open("mydatabase.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS mytable (
            id              INTEGER PRIMARY KEY,
            title           TEXT NOT NULL,
            start_time      TEXT NOT NULL,
            end_time        TEXT NOT NULL,
            category        INTEGER
            )",
        [],
    )?;
    Ok(())
}

pub fn create_or_update_entry(window: Window) -> Result<()> {
    let conn = Connection::open("mydatabase.db")?;
    let mut statement = conn.prepare("SELECT * FROM mytable WHERE title = ? AND start_time = ?")?;
    let windows_iter = statement.query_map(&[&window.title, &window.start_time], |row| {
        Ok(Window {
            id: row.get(0)?,
            title: row.get(1)?,
            start_time: row.get(2)?,
            end_time: row.get(3)?,
        })
    })?;

    let mut windows: Vec<Window> = Vec::new();
    for window in windows_iter {
        windows.push(window.unwrap());
    }

    if windows.len() == 0 {
        conn.execute(
            "INSERT INTO mytable (title, start_time, end_time) VALUES (?1, ?2, ?3)",
            &[&window.title, &window.start_time, &window.end_time],
        )?;
    } else {
        conn.execute(
            "UPDATE mytable SET end_time = ?1 WHERE title = ?2",
            &[&window.end_time, &window.title],
        )?;
    }
    Ok(())
}

pub fn get_entries_on_date(date: String) -> Result<Vec<Window>> {
    let conn = Connection::open("mydatabase.db")?;
    let mut statement = conn.prepare("SELECT * FROM mytable WHERE start_time LIKE ?")?;
    let windows_iter = statement.query_map(&[&date], |row| {
        Ok(Window {
            id: row.get(0)?,
            title: row.get(1)?,
            start_time: row.get(2)?,
            end_time: row.get(3)?,
        })
    })?;

    let mut windows: Vec<Window> = Vec::new();
    for window in windows_iter {
        windows.push(window.unwrap());
    }
    Ok(windows)
}