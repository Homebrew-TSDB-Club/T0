use clap::Parser;
use proto::prometheus::{Label, Sample, TimeSeries, WriteRequest};
use proto::Message;
use std::iter::repeat_with;
use std::time::{SystemTime, UNIX_EPOCH};

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
    #[clap(short, long, default_value_t = 32)]
    threads: usize,
    // gRPC client num.
    #[clap(short, long, default_value_t = 1024)]
    workers: usize,
    // each client request round.
    #[clap(short, long, default_value_t = 8)]
    round: usize,
    // each client request round.
    #[clap(short, long, default_value_t = 128)]
    batch: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Args = clap::Parser::parse();
    fastrand::seed(42);
    let label_keys = generate_strings(7, 10);
    let label_values = generate_strings(7, 10);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(args.threads)
        .enable_all()
        .build()?;

    runtime.block_on(async move {
        let mut joins = Vec::new();
        for _ in 0..args.threads {
            for _ in 0..args.workers {
                let label_keys = label_keys.clone();
                let label_values = label_values.clone();
                let addr = args.addr.clone();
                joins.push(tokio::spawn(async move {
                    let client = hyper::Client::new();
                    for _ in 0..args.round {
                        let request = http::Request::builder()
                            .version(http::Version::HTTP_11)
                            .method(http::Method::POST)
                            .uri(&addr)
                            .body(hyper::Body::from(snappy::compress(
                                encode_body(generate_request(
                                    args.batch,
                                    &label_keys,
                                    &label_values,
                                ))
                                .as_ref(),
                            )))
                            .unwrap();
                        client.request(request).await.unwrap();
                    }
                }));
            }
        }

        let start = SystemTime::now();
        for join in joins {
            join.await.unwrap();
        }
        let since = SystemTime::now().duration_since(start).unwrap();
        println!(
            "QPS: {}",
            (args.threads * args.workers * args.round * args.batch) as f64 / since.as_secs_f64()
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

fn encode_body<T>(msg: T) -> Vec<u8>
where
    T: Message,
{
    let mut buf = Vec::<u8>::new();

    // write the message
    msg.encode(&mut buf).unwrap();
    buf
}
