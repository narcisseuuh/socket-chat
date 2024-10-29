use rusqlite::Connection;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use std::env;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::BufReader;

mod login;
mod messaging;

async fn read_string(
    src: &[u8],
    reader: &mut BufReader<tokio::net::TcpStream>,
    buf: &mut Vec<u8>
) -> Result<String, Box<dyn Error + Send + Sync>> {
    reader.get_mut().write_all(src).await?;
    let n = reader.read_until(b'\n', buf).await?;
    Ok(String::from_utf8_lossy(&buf[..n]).trim().to_string())
}

async fn get_login(
    reader: &mut BufReader<tokio::net::TcpStream>, 
    conn: Arc<Mutex<Connection>>
) -> Result<i32, Box<dyn Error + Send + Sync>> {
    let mut buf = vec![0; 1024];

    let username =
        read_string(b"Enter username: \n>> ", reader, &mut buf).await?;

    let password =
        read_string(b"Enter password: \n>> ", reader, &mut buf).await?;

    if let Ok(id) =
        login::log(conn, &username, &password).await {
        reader.get_mut().write_all(b"Login successful\n").await?;
        return Ok(id)
    } else {
        reader.get_mut().write_all(b"Login failed\n").await?;
        return Err("Login failed".into())
    }
}

async fn register_account(
    reader: &mut BufReader<tokio::net::TcpStream>, 
    conn: Arc<Mutex<Connection>>,
    id: i32
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut buf = vec![0; 1024];

    let username =
        read_string(b"Enter username: \n>> ", reader, &mut buf).await?;

    let password =
        read_string(b"Enter password: \n>> ", reader, &mut buf).await?;

    if login::user_exists(&conn, &username).await? {
        reader.get_mut().write_all(b"Username already exists\n").await?;
        return Err("Username already exists".into());
    }

    login::register(&conn, &username, &password, id).await?;
    reader.get_mut().write_all(b"Registration successful\n").await?;
    Ok(())
}

async fn menu(
    reader: &mut BufReader<tokio::net::TcpStream>,
    id: i32,
    chat: Arc<Mutex<messaging::Chat>>,
    conn: Arc<Mutex<Connection>>
) -> Result<(), Box<dyn Error>> {
    let mut buf = vec![0; 1024];

    let value = login::Person::new(id, conn).await?;
    loop {
        reader.get_mut().write_all(b"1. Send message\n").await?;
        reader.get_mut().write_all(b"2. Show messages\n").await?;
        reader.get_mut().write_all(b"3. Exit\n").await?;

        let n = reader.read(&mut buf).await?;
        let choice_str = String::from_utf8_lossy(&buf[..n]);
        let choice = choice_str.trim();

        match choice {
            "1" => {
                reader.get_mut().write_all(b"Enter recipient id: \n>> ").await?;
                let n = reader.read(&mut buf).await?;
                let recipient: i32 =
                    String::from_utf8_lossy(&buf[..n]).trim().parse()?;
                reader.get_mut().write_all(b"Enter message: \n>> ").await?;
                let n = reader.read(&mut buf).await?;
                let message =
                    String::from_utf8_lossy(&buf[..n]).trim().to_string();
                let mut chat = chat.lock().await;
                let person = value.clone();
                chat.send(
                    person, 
                    recipient, 
                    message, 
                    reader.get_mut()
                ).await?;
            }
            "2" => {
                let chat = chat.lock().await;
                chat.show_messages(id, reader.get_mut()).await?;
            }
            "3" => {
                return Ok(());
            }
            _ => {
                reader.get_mut().write_all(b"Invalid choice\n").await?;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let listener = TcpListener::bind(&addr).await?;
    let conn =
        Arc::new(Mutex::new(login::initialize_database().unwrap()));
    let chat =
        Arc::new(Mutex::new(messaging::Chat::new()));
    println!("Listening on: {}", addr);

    loop {
        let (socket, _) = listener.accept().await?;
        let reader = BufReader::new(socket);
        let conn = Arc::clone(&conn);
        let chat = Arc::clone(&chat);

        tokio::spawn(async move {
            let mut reader = reader;
            loop {
                let mut buf = vec![0; 1];
                let choice =
                {
                    reader.get_mut().write_all(b"1. Login\n").await.unwrap();
                    reader.get_mut().write_all(b"2. Register\n").await.unwrap();
                    reader.get_mut().write_all(b"3. Exit\n").await.unwrap();

                    let n = reader.read(&mut buf).await.unwrap();
                    let choice_str = String::from_utf8_lossy(&buf[..n]);
                    choice_str.trim().to_string()
                };

                match choice.as_str() {
                    "1" => {
                        println!("User logging in...");
                        match get_login(&mut reader, conn.clone()).await {
                            Ok(id) => {
                                println!("User {} entering the main menu!", id);
                                if let Err(e) =
                                menu(&mut reader, id, chat.clone(), conn.clone()).await {
                                    eprintln!("Error in menu: {}", e);
                                }
                                println!("End of user session!");
                            }
                            Err(e) => {
                                eprintln!("Login failed: {}", e);
                            }
                        }
                    }
                    "2" => {
                        println!("User registering...");
                        // Generate a new unique id for each user
                        let id = login::generate_unique_id(&conn).await.unwrap();
                        if let Err(e) =
                        register_account(&mut reader, conn.clone(), id).await {
                            eprintln!("Registration failed: {}", e);
                        }
                        println!("User registered!");
                    }
                    "3" => {
                        break;
                    }
                    _ => {
                        reader.get_mut().write_all(b"Invalid choice\n").await.unwrap();
                    }
                }
            }
        });
    }
}
