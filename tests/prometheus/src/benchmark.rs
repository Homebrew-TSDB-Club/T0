use clap::Parser;
use proto::prometheus::{Label, Sample, TimeSeries, WriteRequest};
use std::iter::repeat_with;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

#[derive(Parser, Debug)]
#[clap(
    name = "benchmark",
    about = "Prometheus remote write performance benchmark"
)]
struct Args {
    /// Remote write server address.
    #[clap(short, long)]
    addr: String,
    // Client thread num.
    #[clap(short, long)]
    threads: usize,
    // each client request round.
    #[clap(short, long)]
    round: usize,
    // each client request round.
    #[clap(short, long)]
    batch: usize,
    // each client request round.
    #[clap(short, long)]
    channels: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Args = clap::Parser::parse();
    fastrand::seed(42);
    let label_keys = Arc::new(generate_strings(7, 10));
    let label_values = Arc::new(generate_strings(7, 10));
    let addr = Arc::new(args.addr);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(args.threads)
        .enable_all()
        .build()?;

    runtime.block_on(async move {
        let mut joins = Vec::new();
        for _ in 0..args.channels {
            let addr = Arc::clone(&addr);
            let label_keys = Arc::clone(&label_keys);
            let label_values = Arc::clone(&label_values);
            joins.push(tokio::spawn(async move {
                let mut stream = TcpStream::connect(addr.as_ref()).await.unwrap();
                for _ in 0..args.round {
                    let request = generate_request(args.batch, &label_keys, &label_values);
                    let mut buf = Vec::new();
                    buf.reserve(prost::Message::encoded_len(&request));
                    prost::Message::encode(&request, &mut buf).unwrap();
                    stream.write_u64(buf.len() as u64).await.unwrap();
                    stream.write_all(&buf).await.unwrap();
                }
            }));
        }

        let start = SystemTime::now();
        for join in joins {
            join.await.unwrap();
        }
        let since = SystemTime::now().duration_since(start).unwrap();
        println!(
            "QPS: {}",
            (args.channels * args.round * args.batch) as f64 / since.as_secs_f64()
        );
    });
    Ok(())
}

fn generate_timeseries(keys: &[String], values: &[String]) -> TimeSeries {
    let mut labels = vec![Label {
        name: String::from("__name__"),
        value: String::from("test_benchmark"),
    }];
    for key in keys {
        labels.push(Label {
            name: key.clone(),
            value: values[fastrand::usize(0..7)].clone(),
        });
    }
    TimeSeries {
        labels,
        samples: vec![Sample {
            value: 1.0,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
        }],
        exemplars: vec![],
    }
}

fn generate_request(batch: usize, keys: &[String], values: &[String]) -> WriteRequest {
    WriteRequest {
        timeseries: (0..batch)
            .into_iter()
            .map(|_| generate_timeseries(keys, values))
            .collect(),
        metadata: vec![],
    }
}

fn generate_strings(len: usize, size: usize) -> Vec<String> {
    (0..len)
        .into_iter()
        .map(|_| repeat_with(fastrand::alphanumeric).take(size).collect())
        .collect()
}
