//! This example is by no-means a full websockets example
//! Please do not use it for anything serious, as it skipps the
//! websockets handshake.
use apocalypse::{Hell, Demon};
use cataclysm_ws::{WebSocketReader, Frame, Message};
use tokio::net::{TcpStream, TcpListener};
use tokio::io::AsyncWriteExt;

use misc::SimpleLogger;
mod misc;

// Human demon that echoes a message with its name
struct Human {
    name: String
}

// Demon implementation for the human
#[async_trait::async_trait]
impl Demon for Human {
    type Input = String;
    type Output = String;
    async fn handle(&mut self, message: Self::Input) -> Self::Output {
        format!("Hey, {} here: {}", self.name, &message)
    }
}

#[async_trait::async_trait]
impl WebSocketReader for Human {
    async fn on_message(&mut self, message: Message) {
        match message {
            Message::Text(text) => {
                println!("Hey, {} here from WebSocket: {}", self.name, text)
            },
            _ => ()
        }
    }
}

#[tokio::main]
async fn main() {
    SimpleLogger::new().with_level(log::LevelFilter::Debug).init().unwrap();
    // We create hell here
    let hell = Hell::new();
    let (gate, jh) = match hell.fire().await {
        Ok(v) => v,
        Err(e) => panic!("Could not light up hell, {}", e)
    };

    tokio::spawn(async move {
        // Now, we create a listener
        let listener = TcpListener::bind("127.0.0.1:8000").await.unwrap();
        let mut counter: usize = 0;

        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let (read_stream, _write_stream) = socket.into_split();

            // We create one demon
            let carlos = Human {
                name: format!("Human-{}", counter)
            };

            // We spawn the demon in the running hell through the gate
            let location = match gate.spawn_ws(carlos, read_stream).await {
                Ok(v) => v,
                Err(e) => panic!("Could not spawn the demon, {}", e)
            };

            // We send a message to see the actor behaviour
            let m1 = gate.send(&location, "hello world".to_string()).await.unwrap();
            // We print the message just for show
            println!("As actor: {}", &m1);
            assert_eq!(format!("Hey, Human-{} here: hello world", counter), m1);

            counter += 1;

            gate.vanquish(&location).await.unwrap();
        }
    });

    tokio::spawn(async move {
        // Connect to a peer
        let mut stream = TcpStream::connect("127.0.0.1:8000").await.unwrap();
        // Write some data.
        stream.write_all(&Frame::text("hello world").bytes()).await.unwrap();
        // When we drop the stream, the actor will get killed, so we wait at least 100ms to receive the other message
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    });

    // We wait for all messages to be processed.
    jh.await.unwrap();
}