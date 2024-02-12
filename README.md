# Apocalypse: Simple Actor framework for Rust.

**Work in progress, unusable in its current state**

Acopacypse is heavily based on my experience with the actix framework, but built straight over tokio in a fully asynchronous way, and also in a very simplified way.

Here is a simple example using the framework:

```rust
use apocalypse::{Hell, Demon};

// Human demon that echoes a message with its name
struct Human {
    name: String
}

// Demon implementation for the human
impl Demon for Human {
    type Input = String;
    type Output = String;
    async fn handle(&mut self, message: Self::Input) -> Self::Output {
        format!("Hey, {} here: {}", self.name, &message)
    }
}

#[tokio::main]
async fn main() {
    // We create one demon
    let carlos = Human{
        name: "Carlos".to_string()
    };

    // We create a hell for this
    let hell = Hell::new();
    let join_handle = {
        let (gate, jh) = match hell.ignite().await {
            Ok(v) => v,
            Err(e) => panic!("Could not light up hell, {}", e)
        };
        
        // We spawn the demon in the running hell through the gate
        let location = match gate.spawn(carlos).await {
            Ok(v) => v,
            Err(e) => panic!("Could not spawn the demon, {}", e)
        };
    
        tokio::spawn(async move {
            let m1 = gate.send(&location, "hello world".to_string()).await.unwrap();
            // We print the message just for show
            println!("{}", &m1);
            // And check that it is correct
            assert_eq!("Hey, Carlos here: hello world", &m1);
        });

        jh
    };

    // We wait for all messages to be processed.
    join_handle.await.unwrap();
}
```

## Lockups

At the moment, each demon has their own thread to process requests, so the only way to cause a lockup is to create a ring of requests that starts in one demon, and ends in the same one. Avoid this situation at all costs.