use crate::kafka;
use crate::model::OffsetAndMetadata::OffsetKey;
use crate::model::{OffsetAndMetadata, OffsetValue};
use futures::StreamExt;
use log::warn;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::Message;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::{Arc, Mutex};

pub struct OffsetsConsumer {
    pub consumer: StreamConsumer,
    pub offsets: Arc<Mutex<HashMap<OffsetAndMetadata, OffsetValue>>>,
}

impl OffsetsConsumer {
    pub async fn get_consumer_group_offsets(
        brokers: Arc<String>,
        offsets: Arc<Mutex<HashMap<OffsetAndMetadata, OffsetValue>>>,
    ) {
        let consumer = kafka::build_offset_consumer(brokers.as_str());
        consumer
            .subscribe(&vec!["__consumer_offsets"])
            .expect("Can't subscribe to specified topics");

        let mut message_stream = consumer.start();

        while let Some(message) = message_stream.next().await {
            match message {
                Err(_e) => (), //TODO push to hashmap only after the first EOF event?
                Ok(m) => {
                    let key = match m.key_view::<[u8]>() {
                        None => &[],
                        Some(Ok(key)) => key,
                        Some(Err(e)) => {
                            warn!("Error while deserializing message key: {:?}", e);
                            &[]
                        }
                    };

                    let payload = match m.payload_view::<[u8]>() {
                        Some(Ok(s)) => s,
                        None => &[],
                        Some(Err(e)) => {
                            warn!("Error while deserializing message payload: {:?}", e);
                            &[]
                        }
                    };

                    let mut t = offsets.lock().unwrap();
                    if let (Ok(key), Ok(value)) = (
                        OffsetAndMetadata::try_from(key),
                        OffsetValue::try_from(payload),
                    ) {
                        if let OffsetKey { .. } = key {
                            t.insert(key, value);
                        }
                    }

                    // self.offset_consumer.commit_message(&m, CommitMode::Async).unwrap();
                }
            };
        }
    }
}
