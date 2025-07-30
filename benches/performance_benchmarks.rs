use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_janus::prelude::*;
use std::collections::HashMap;

fn benchmark_message_serialization(c: &mut Criterion) {
    let command = SocketCommand::new(
        "test_channel".to_string(),
        "test_command".to_string(),
        Some({
            let mut args = HashMap::new();
            args.insert("key".to_string(), serde_json::json!("value"));
            args
        }),
        Some(5.0),
    );

    c.bench_function("serialize_socket_command", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(&command).unwrap());
        })
    });

    let serialized = serde_json::to_string(&command).unwrap();
    c.bench_function("deserialize_socket_command", |b| {
        b.iter(|| {
            black_box(serde_json::from_str::<SocketCommand>(&serialized).unwrap());
        })
    });
}

criterion_group!(benches, benchmark_message_serialization);
criterion_main!(benches);