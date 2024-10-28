use std::sync::Arc;
use tokio::sync::Mutex;

use rusqlite::{Connection, Result};
use sha1::{Sha1, Digest};

#[derive(Debug)]
#[derive(Clone)]
pub struct Person {
    id: i32,
    name: String,
    is_admin: bool,
    password_hash: String,
}

impl Person {
    pub async fn new(id: i32, conn: Arc<Mutex<Connection>>) -> Result<Person, rusqlite::Error> {
        let conn = conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT name, is_admin, password FROM person WHERE id = ?1"
        )?;
        let mut rows = stmt.query(&[&id])?;

        if let Some(row) = rows.next()? {
            let name: String = row.get(0)?;
            let is_admin: bool = row.get(1)?;
            let password_hash: String = row.get(2)?;

            Ok(Person {
                id,
                name,
                is_admin,
                password_hash,
            })
        } else {
            Err(rusqlite::Error::QueryReturnedNoRows)
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}

pub fn initialize_database() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;

    conn.execute(
        "CREATE TABLE person (
            id   INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            is_admin BOOLEAN NOT NULL,
            password TEXT NOT NULL
        )",
        (),
    )?;

    let mut hasher = Sha1::new();
    hasher.update("admin".as_bytes());
    let password_hash = format!("{:x}", hasher.finalize());
    let admin = Person {
        id: 0,
        name: "admin".to_string(),
        is_admin: true,
        password_hash
    };
    conn.execute(
        "INSERT INTO person (id, name, is_admin, password) 
            VALUES (?1, ?2, ?3, ?4)",
        (&admin.id, &admin.name, 
            &admin.is_admin, &admin.password_hash),
    )?;

    Ok(conn)
}

pub async fn log(
    conn: Arc<Mutex<Connection>>, 
    username: &String, 
    password: &String
) -> Result<i32, rusqlite::Error> {
    let mut hasher = Sha1::new();
    hasher.update(password.as_bytes());
    let password_hash = format!("{:x}", hasher.finalize());
    let conn = conn.lock().await;
    let mut stmt = conn.prepare(
        "SELECT id FROM person WHERE name = ?1 AND password = ?2")?;
    let mut rows = stmt.query(&[username, &password_hash])?;

    if let Some(row) = rows.next()? {
        let id: i32 = row.get(0)?;
        Ok(id)
    } else {
        Err(rusqlite::Error::QueryReturnedNoRows)
    }
}

pub async fn user_exists(
    conn: &Arc<Mutex<Connection>>, 
    username: &str
) -> Result<bool> {
    let conn = conn.lock().await;
    let mut stmt =
        conn.prepare("SELECT COUNT(*) FROM person WHERE name = ?1")?;
    let count: i64 =
        stmt.query_row([username], |row| row.get(0))?;
    
    Ok(count > 0)
}

pub async fn register(
    conn: &Arc<Mutex<Connection>>, 
    username: &String, 
    password: &String,
    id : i32
) -> Result<()> {
    let mut hasher = Sha1::new();
    hasher.update(password.as_bytes());
    let password_hash = format!("{:x}", hasher.finalize());
    let person = Person {
        id: id,
        name: username.to_string(),
        is_admin: false,
        password_hash: password_hash,
    };
    let conn = conn.lock().await;
    conn.execute(
        "INSERT INTO person (id, name, is_admin, password) 
            VALUES (?1, ?2, ?3, ?4)",
        (&person.id, &person.name, 
            &person.is_admin, &person.password_hash),
    )?;
    
    Ok(())
}

pub async fn generate_unique_id(conn: &Arc<Mutex<Connection>>) -> rusqlite::Result<i32> {

    let conn = conn.lock().await;

    let mut stmt = conn.prepare("SELECT MAX(id) FROM person")?;

    let id: Option<i32> = stmt.query_row([], |row| row.get(0))?;

    Ok(id.unwrap_or(0) + 1)

}
