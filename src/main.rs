use std::{collections::HashMap, time::Duration};

use clap::Parser;
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::CommitMode;
use rdkafka::Message;
use rdkafka::{
    config::RDKafkaLogLevel,
    consumer::Consumer,
    error::KafkaError,
    producer::{FutureProducer, FutureRecord},
    ClientConfig,
};
use scylla::transport::errors::NewSessionError;
use scylla::{Session, SessionBuilder};
use serde::Serialize;

trait Metadata {
    fn broker(&self) -> &str;
    fn topic(&self) -> &str;
}

#[derive(Parser, Debug)]
#[clap(version, author, about)]
pub enum Opts {
    AddMutant(AddMutant),
    ReadMutant(ReadMutant),
}

#[derive(Parser, Debug)]
pub struct AddMutant {
    #[arg(short, long)]
    pub name: String,
    #[arg(short, long)]
    pub last_name: String,
    #[arg(short, long)]
    pub address: String,
}

impl Metadata for AddMutant {
    fn broker(&self) -> &str {
        "localhost:9092"
    }

    fn topic(&self) -> &str {
        "mutants"
    }
}

#[derive(Parser, Debug)]
pub struct ReadMutant {
    #[arg(short, long)]
    pub group_id: String,
}

impl Metadata for ReadMutant {
    fn broker(&self) -> &str {
        "localhost:9092"
    }

    fn topic(&self) -> &str {
        "mutants"
    }
}

#[derive(Debug, Serialize)]
pub struct Payload {
    pub key: String,
    pub payload: String,
}

pub async fn produce(broker: &str, topic: &str, payload: &Payload) {
    let producer: &Result<FutureProducer, KafkaError> = &ClientConfig::new()
        .set("bootstrap.servers", broker)
        .set("message.timeout.ms", "5000")
        .create::<FutureProducer<_>>();

    // Very bad way but hey, is just for learning purposes :)

    match producer {
        Ok(producer) => {
            let status = producer
                .send(
                    FutureRecord::to(topic)
                        .payload(&payload.payload)
                        .key(&payload.key),
                    Duration::from_secs(0),
                )
                .await;

            println!("Status: {:?}", status);
        }
        Err(e) => {
            println!("Error while creating producer: {}", e);
        }
    }
}

pub async fn consume(broker: &str, topic: &str, group_id: &str) {
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", broker)
        .set("group.id", group_id)
        .set("enable.partition.eof", "false")
        .set("enable.auto.commit", "true")
        .set_log_level(RDKafkaLogLevel::Debug)
        .create()
        .expect("Consumer creation failed");

    consumer
        .subscribe(&[topic])
        .expect("Can't subscribe to specified topic");

    loop {
        let message = consumer.recv().await;

        match message {
            Ok(message) => {
                let payload = match message.payload_view::<str>() {
                    None => "",
                    Some(Ok(s)) => s,
                    Some(Err(e)) => {
                        println!("Error while deserializing message payload: {:?}", e);
                        ""
                    }
                };
                let result = consumer.commit_message(&message, CommitMode::Async);
                println!(
                    "Received message: {:?}. Commit result: {:?}",
                    payload, result
                );
            }
            Err(e) => {
                println!("Error while receiving message: {}", e);
            }
        }
    }
}

async fn insert_mutant_to_scylla(session: &Session, name: &str, last_name: &str, address: &str) {
    match session
        .query(
            "INSERT INTO catalog.mutants (first_name, last_name, address, picture_location) VALUES (?, ?, ?, ?)",
            (name, last_name, address, "https://www.google.com"),
        )
        .await
        .map_err(|e| {
            println!("Error while inserting mutant: {}", e);
        })
        .is_err()
    {
        true => (),
        false => println!("Mutant inserted"),
    }
}

#[tokio::main]
async fn main() {
    let opts: Opts = Opts::parse();

    match opts {
        Opts::AddMutant(add_mutant) => {
            let session: Result<Session, NewSessionError> = SessionBuilder::new()
                .known_node("localhost:9042")
                .build()
                .await;

            match session {
                Ok(session) => {
                    println!("Session created, inserting mutant");
                    insert_mutant_to_scylla(
                        &session,
                        add_mutant.name.as_str(),
                        add_mutant.last_name.as_str(),
                        add_mutant.address.as_str(),
                    )
                    .await;

                }
                Err(e) => {
                    println!("Error while creating session: {}", e);
                }
            }

            let payload = Payload {
                key: add_mutant.name.to_owned(),
                payload: serde_json::to_string(&HashMap::from([
                    ("name", add_mutant.name.as_str()),
                    ("last_name", add_mutant.last_name.as_str()),
                    ("address", add_mutant.address.as_str()),
                ]))
                .expect("Error while serializing payload"),
            };

            produce(add_mutant.broker(), add_mutant.topic(), &payload).await;
        }
        Opts::ReadMutant(read_mutant) => {
            consume(
                read_mutant.broker(),
                read_mutant.topic(),
                read_mutant.group_id.as_str(),
            )
            .await;
        }
    };
}
