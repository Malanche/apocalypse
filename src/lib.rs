//! # Apocalypse: Simple Actor framework for Rust.
//!
//! Apocalypse is a simple actor framework inspired by actix. The goal is to have a fully asynchronous framework with few disadvantages. A simple working example is the following
//!
//! ```rust
//! use apocalypse::{Hell, Demon};
//! 
//! // Human demon that echoes a message with its name
//! struct Human {
//!     name: String
//! }
//! 
//! // Demon implementation for the human
//! #[async_trait::async_trait]
//! impl Demon for Human {
//!     type Input = String;
//!     type Output = String;
//!     async fn handle(&mut self, message: Self::Input) -> Self::Output {
//!         format!("Hey, {} here: {}", self.name, &message)
//!     }
//! }
//! 
//! #[tokio::main]
//! async fn main() {
//!     // We create one demon
//!     let carlos = Human{
//!         name: "Carlos".to_string()
//!     };
//! 
//!     // We create hell for this
//!     let hell = Hell::new();
//!     let (portal, jh) = match hell.fire().await {
//!         Ok(v) => v,
//!         Err(e) => panic!("Could not light up hell, {}", e)
//!     };
//!     
//!     // We spawn the demon in the running hell through the portal
//!     let location = match portal.spawn(carlos).await {
//!         Ok(v) => v,
//!         Err(e) => panic!("Could not spawn the demon, {}", e)
//!     };
//! 
//!     tokio::spawn(async move {
//!         // We send a message to the demon, and await the response.
//!         let m1 = portal.send(&location, "hello world".to_string()).await.unwrap();
//!         // We print the message just for show
//!         println!("{}", &m1);
//!         // And check that it is correct
//!         assert_eq!("Hey, Carlos here: hello world", &m1);
//!     });
//! 
//!     // We wait for all messages to be processed.
//!     jh.await.unwrap();
//! }
//! ```
//!
//! As you can see, you will need to use [async_trait](https://docs.rs/async_trait) in order to implement the [Demon](crate::Demon) trait in your code.

pub use self::demon::{Demon, Location};
pub use self::hell::Hell;
pub use self::gate::{Gate};
pub use self::error::Error;

mod demon;
mod hell;
mod gate;
mod error;