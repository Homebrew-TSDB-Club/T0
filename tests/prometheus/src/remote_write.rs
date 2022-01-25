use clap::Parser;
use proto::prometheus::remote_client::RemoteClient;
use proto::prometheus::{Label, Sample, TimeSeries, WriteRequest};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Parser, Debug)]
#[clap(name = "client", about = "Prometheus remote write test client")]
struct Args {
    /// Remote write server address.
    #[clap(short, long)]
    addr: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Args = clap::Parser::parse();
    let mut client = RemoteClient::connect(args.addr).await?;

    let timeseries = TimeSeries {
        labels: vec![
            Label {
                name: String::from("__name__"),
                value: String::from("test_test"),
            },
            Label {
                name: String::from("label1"),
                value: String::from("test"),
            },
        ],
        samples: vec![Sample {
            value: 1.0,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
        }],
        exemplars: vec![],
    };

    let request = tonic::Request::new(WriteRequest {
        timeseries: vec![timeseries],
        metadata: vec![],
    });

    let response = client.write(request).await?;

    println!("RESPONSE={:?}", response);

    Ok(())
}
