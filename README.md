# Socket Chat

## Purpose
The purpose of this project is to learn Rust and gain experience with socket programming.

## Building the Project
To build the project, you can use Docker for a consistent development environment. Follow these steps:

1. Clone the repository:
```sh
git clone https://github.com/yourusername/socket-chat.git
cd socket-chat
```

2. Build the Docker image:
```sh
docker build -t socket-chat .
```

3. Run the Docker container:
```sh
docker run -it -p 8080:8080 socket-chat
```

## Requirements

See `Cargo.toml` :
```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
rusqlite = "0.32.1"
sha1 = "0.10.5"
```
