use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use crate::login;

struct Message {
    sender: login::Person,
    recipient: i32,
    message: String,
}

pub struct Chat {
    messages: Vec<Message>,
}

impl Chat {
    pub fn new() -> Chat {
        Chat {
            messages: Vec::new(),
        }
    }

    pub async fn send(
        &mut self,
        sender: login::Person,
        recipient: i32,
        message: String,
        socket: &mut TcpStream
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.messages.push(Message {
            sender,
            recipient,
            message: message.clone(),
        });

        let mut reader = BufReader::new(socket);
        let notification = format!("Your message '{}' was sent!\n", message);
        reader.get_mut().write_all(notification.as_bytes()).await?;

        Ok(())
    }
    
    pub async fn show_messages(
        &self,
        id: i32,
        socket: &mut TcpStream
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut reader = BufReader::new(socket);
        for message in &self.messages {
            if id == 0 || message.recipient == id {
                let msg =
                    format!("From {}: {}\n", message.sender.get_name(), message.message);
                reader.get_mut().write_all(msg.as_bytes()).await?;
            }
        }

        Ok(())
    }
}
