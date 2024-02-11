use criterion::*;
use apocalypse::{Hell, Demon};

// Human demon that echoes a message with its name
struct Human {}

// Demon implementation for the human
impl Demon for Human {
    type Input = ();
    type Output = ();
    async fn handle(&mut self, _message: Self::Input) -> Self::Output {
        ()
    }
}

fn bench(c: &mut Criterion) {
    for number in [1, 2, 4, 8, 16, 32] {
        c.bench_function(&format!("{} Actor(s), Empty Ping Pong", number), |b| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let handle = rt.handle();
            let (gate, locations) = handle.block_on(async {
                // We create a hell for this
                let hell = Hell::new();
                let (gate, _) = hell.fire().await.unwrap();
    
                // We spawn the demon in the running hell through the gate
                let mut locations = Vec::new();
                for _ in 0..number {
                    locations.push(gate.spawn(Human{}).await.unwrap());
                }
                (gate, locations)
            });
    
            b.to_async(rt).iter(|| async {
                let futs = locations.iter().map(|location| gate.send(&location, ()));
    
                let res = futures::future::join_all(futs).await;
                for val in res {
                    val.unwrap();
                };
            });
        });
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);