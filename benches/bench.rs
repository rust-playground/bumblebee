#[macro_use]
extern crate criterion;

use bumblebee::transformer::TransformerBuilder;
use criterion::{Benchmark, Criterion, Throughput};

fn criterion_benchmark(c: &mut Criterion) {
    let trans = TransformerBuilder::default()
        .add_direct("top", "new")
        .unwrap()
        .build()
        .unwrap();
    let input = r#"
    {
        "top": "value"
    }"#;

    c.bench(
        "top_level",
        Benchmark::new("1_top_level", move |b| {
            b.iter(|| trans.apply_from_str(input))
        })
        .throughput(Throughput::Bytes(input.as_bytes().len() as u32)),
    );

    let trans = TransformerBuilder::default()
        .add_direct("top1", "new1")
        .unwrap()
        .add_direct("top2", "new2")
        .unwrap()
        .add_direct("top3", "new3")
        .unwrap()
        .add_direct("top4", "new4")
        .unwrap()
        .add_direct("top5", "new5")
        .unwrap()
        .add_direct("top6", "new6")
        .unwrap()
        .add_direct("top7", "new7")
        .unwrap()
        .add_direct("top8", "new8")
        .unwrap()
        .add_direct("top9", "new9")
        .unwrap()
        .add_direct("top10", "new10")
        .unwrap()
        .build()
        .unwrap();
    let input = r#"
    {
        "top1": "value",
        "top2": "value",
        "top3": "value",
        "top4": "value",
        "top5": "value",
        "top6": "value",
        "top7": "value",
        "top8": "value",
        "top9": "value",
        "top10": "value"
    }"#;

    c.bench(
        "top_level",
        Benchmark::new("10_top_level", move |b| {
            b.iter(|| trans.apply_from_str(input))
        })
        .throughput(Throughput::Bytes(input.as_bytes().len() as u32)),
    );

    let trans = TransformerBuilder::default()
        .add_constant("value", "new")
        .unwrap()
        .build()
        .unwrap();
    let input = r#"
    {
        "top": "value"
    }"#;

    c.bench(
        "constant",
        Benchmark::new("contant", move |b| b.iter(|| trans.apply_from_str(input)))
            .throughput(Throughput::Bytes(input.as_bytes().len() as u32)),
    );

    let trans = TransformerBuilder::default()
        .add_direct("top1", "new1")
        .unwrap()
        .add_direct("top2", "new2")
        .unwrap()
        .add_direct("top3", "new3")
        .unwrap()
        .add_direct("top4", "new4")
        .unwrap()
        .add_direct("top5", "new5")
        .unwrap()
        .add_direct("top6", "new6")
        .unwrap()
        .add_direct("top7", "new7")
        .unwrap()
        .add_direct("top8", "new8")
        .unwrap()
        .add_direct("top9", "new9")
        .unwrap()
        .add_direct("top10", "new10")
        .unwrap()
        .build()
        .unwrap();
    let input = r#"[
        {"top1": "value"},
        {"top2": "value"},
        {"top3": "value"},
        {"top4": "value"},
        {"top5": "value"},
        {"top6": "value"},
        {"top7": "value"},
        {"top8": "value"},
        {"top9": "value"},
        {"top10": "value"}
    ]"#;

    c.bench(
        "many_2_many",
        Benchmark::new("10_top_level_many_2_many", move |b| {
            b.iter(|| trans.apply_from_str(input))
        })
        .throughput(Throughput::Bytes(input.as_bytes().len() as u32)),
    );

    let trans = TransformerBuilder::default()
        .add_flatten("nested", "", Some("new"), Some("_"), false)
        .unwrap()
        .build()
        .unwrap();
    let input = r#"{"nested":{
        "top1": "value1",
        "top2": "value2",
        "top3": "value3",
        "top4": "value4",
        "top5": "value5",
        "top6": "value6",
        "top7": "value7",
        "top8": "value8",
        "top9": "value9",
        "top10": "value10"}
    }"#;

    c.bench(
        "flatten",
        Benchmark::new("10_flatten_direct", move |b| {
            b.iter(|| trans.apply_from_str(input))
        })
        .throughput(Throughput::Bytes(input.as_bytes().len() as u32)),
    );

    let trans = TransformerBuilder::default()
        .add_flatten("nested", "", None, Some("_"), false)
        .unwrap()
        .build()
        .unwrap();
    let input = r#"{"nested":[
        "value1",
        "value2",
        "value3",
        "value4",
        "value5",
        "value6",
        "value7",
        "value8",
        "value9",
        "value10"]
    }"#;

    c.bench(
        "flatten",
        Benchmark::new("10_flatten_array", move |b| {
            b.iter(|| trans.apply_from_str(input))
        })
        .throughput(Throughput::Bytes(input.as_bytes().len() as u32)),
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
