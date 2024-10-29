use rusqlite::Connection;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use std::env;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;

mod login;
mod messaging;

async fn get_login(
    socket: &Arc<Mutex<tokio::net::TcpStream>>, 
    conn: Arc<Mutex<Connection>>
) -> Result<i32, Box<dyn Error + Send + Sync>> {
    let mut buf = vec![0; 1024];
    let mut socket = socket.lock().await;

    socket.write_all(b"Enter username: \n>> ").await?;
    let n = socket.read(&mut buf).await?;
    let username =
        String::from_utf8_lossy(&buf[..n]).trim().to_string();

    socket.write_all(b"Enter password: \n>> ").await?;
    let n = socket.read(&mut buf).await?;
    let password =
        String::from_utf8_lossy(&buf[..n]).trim().to_string();

    if let Ok(id) =
        login::log(conn, &username, &password).await {
        socket.write_all(b"Login successful\n").await?;
        return Ok(id)
    } else {
        socket.write_all(b"Login failed\n").await?;
        return Err("Login failed".into())
    }
}

async fn register_account(
    socket: &Arc<Mutex<tokio::net::TcpStream>>, 
    conn: Arc<Mutex<Connection>>,
    id: i32
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut buf = vec![0; 1024];
    let mut socket = socket.lock().await;

    socket.write_all(b"Enter new username: \n>> ").await?;
    let n = socket.read(&mut buf).await?;
    let username =
        String::from_utf8_lossy(&buf[..n]).trim().to_string();

    socket.write_all(b"Enter new password: \n>> ").await?;
    let n = socket.read(&mut buf).await?;
    let password =
        String::from_utf8_lossy(&buf[..n]).trim().to_string();

    if login::user_exists(&conn, &username).await? {
        socket.write_all(b"Username already exists\n").await?;
        return Err("Username already exists".into());
    }

    login::register(&conn, &username, &password, id).await?;
    socket.write_all(b"Registration successful\n").await?;
    Ok(())
}

async fn menu(
    socket: &Arc<Mutex<tokio::net::TcpStream>>,
    id: i32,
    chat: Arc<Mutex<messaging::Chat>>,
    conn: Arc<Mutex<Connection>>
) -> Result<(), Box<dyn Error>> {
    let mut buf = vec![0; 1024];
    let mut socket = socket.lock().await;

    let value = login::Person::new(id, conn).await?;
    loop {
        socket.write_all(b"1. Send message\n").await?;
        socket.write_all(b"2. Show messages\n").await?;
        socket.write_all(b"3. Exit\n").await?;

        let n = socket.read(&mut buf).await?;
        let choice_str = String::from_utf8_lossy(&buf[..n]);
        let choice = choice_str.trim();

        match choice {
            "1" => {
                socket.write_all(b"Enter recipient id: \n>> ").await?;
                let n = socket.read(&mut buf).await?;
                let recipient: i32 =
                    String::from_utf8_lossy(&buf[..n]).trim().parse()?;
                socket.write_all(b"Enter message: \n>> ").await?;
                let n = socket.read(&mut buf).await?;
                let message =
                    String::from_utf8_lossy(&buf[..n]).trim().to_string();
                let mut chat = chat.lock().await;
                let person = value.clone();
                chat.send(
                    person, 
                    recipient, 
                    message, 
                    &mut socket
                ).await?;
            }
            "2" => {
                let chat = chat.lock().await;
                chat.show_messages(id, &mut socket).await?;
            }
            "3" => {
                return Ok(());
            }
            _ => {
                socket.write_all(b"Invalid choice\n").await?;
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
        let socket_locker = Arc::new(Mutex::new(socket));
        let conn = Arc::clone(&conn);
        let chat = Arc::clone(&chat);

        tokio::spawn(async move {
            loop {
                let mut buf = vec![0; 1];
                let choice =
                {
                    let mut socket_borrowing =
                        socket_locker.lock().await;

                    socket_borrowing.write_all(b"1. Login\n").await.unwrap();
                    socket_borrowing.write_all(b"2. Register\n").await.unwrap();
                    socket_borrowing.write_all(b"3. Exit\n").await.unwrap();

                    let n = socket_borrowing.read(&mut buf).await.unwrap();
                    let choice_str = String::from_utf8_lossy(&buf[..n]);
                    choice_str.trim().to_string()
                };

                match choice.as_str() {
                    "1" => {
                        println!("User logging in...");
                        match get_login(&socket_locker, conn.clone()).await {
                            Ok(id) => {
                                println!("User {} entering the main menu!", id);
                                if let Err(e) =
                                menu(&socket_locker, id, chat.clone(), conn.clone()).await {
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
                        register_account(&socket_locker, conn.clone(), id).await {
                            eprintln!("Registration failed: {}", e);
                        }
                        println!("User registered!");
                    }
                    "3" => {
                        break;
                    }
                    _ => {
                        let mut socket_borrowing =
                            socket_locker.lock().await;
                        socket_borrowing
                            .write_all(b"Invalid choice\n").await.unwrap();
                    }
                }
            }
        });
    }
}
