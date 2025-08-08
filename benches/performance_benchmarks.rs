use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_janus::prelude::*;
use std::collections::HashMap;

fn benchmark_message_serialization(c: &mut Criterion) {
    let request = JanusRequest::new(
        "test_channel".to_string(),
        "test_request".to_string(),
        Some({
            let mut args = HashMap::new();
            args.insert("key".to_string(), serde_json::json!("value"));
            args
        }),
        Some(5.0),
    );

    c.bench_function("serialize_socket_request", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(&request).unwrap());
        })
    });

    let serialized = serde_json::to_string(&request).unwrap();
    c.bench_function("deserialize_socket_request", |b| {
        b.iter(|| {
            black_box(serde_json::from_str::<JanusRequest>(&serialized).unwrap());
        })
    });
}

criterion_group!(benches, benchmark_message_serialization);
criterion_main!(benches);