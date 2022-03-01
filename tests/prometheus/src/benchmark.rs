use clap::Parser;
use flat::write::{
    Label, LabelArgs, Sample, Timeseries, TimeseriesArgs, WriteRequest, WriteRequestArgs,
};
use flatbuffers::{FlatBufferBuilder, WIPOffset};
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
                stream.write_u64(0x9d2bd00b191c59e9).await.unwrap();
                for _ in 0..args.round {
                    let mut builder = FlatBufferBuilder::with_capacity(512);
                    generate_request(&mut builder, args.batch, &label_keys, &label_values);
                    let buf = builder.finished_data();
                    stream.write_u64(buf.len() as u64).await.unwrap();
                    stream.write_all(buf).await.unwrap();
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

fn generate_timeseries<'a, 'b: 'a>(
    builder: &'a mut FlatBufferBuilder<'b>,
    keys: &'a [String],
    values: &'a [String],
) -> WIPOffset<Timeseries<'b>> {
    let args = LabelArgs {
        name: Some(builder.create_string("__name__")),
        value: Some(builder.create_string("test_benchmark")),
    };
    let mut labels = vec![Label::create(builder, &args)];

    for key in keys {
        let args = LabelArgs {
            name: Some(builder.create_string(key.as_ref())),
            value: Some(builder.create_string(values[fastrand::usize(0..7)].as_ref())),
        };
        labels.push(Label::create(builder, &args));
    }

    let sample = vec![Sample::new(
        1.0,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
    )];

    let args = TimeseriesArgs {
        labels: Some(builder.create_vector(&labels)),
        samples: Some(builder.create_vector(&sample)),
    };

    Timeseries::create(builder, &args)
}

fn generate_request<'a, 'b: 'a>(
    builder: &'a mut FlatBufferBuilder<'b>,
    batch: usize,
    keys: &'a [String],
    values: &'a [String],
) {
    let mut ts = Vec::new();
    for _ in 0..batch {
        ts.push(generate_timeseries(builder, keys, values));
    }
    let args = WriteRequestArgs {
        timeseries: Some(builder.create_vector(&ts)),
    };
    let request = WriteRequest::create(builder, &args);
    builder.finish(request, None);
}

fn generate_strings(len: usize, size: usize) -> Vec<String> {
    (0..len)
        .into_iter()
        .map(|_| repeat_with(fastrand::alphanumeric).take(size).collect())
        .collect()
}
