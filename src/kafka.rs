use crate::model::{
    ClusterInfo, GroupInfo, GroupMember, MemberAssignment, PartitionOffsets, TopicDetail, TopicInfo,
};

use crate::utils::read_str;
use byteorder::{BigEndian, ReadBytesExt};

use log::trace;
use rdkafka::config::RDKafkaLogLevel;
use rdkafka::consumer::{stream_consumer::StreamConsumer, BaseConsumer, Consumer};
use rdkafka::metadata::Metadata;
use rdkafka::ClientConfig;
use std::io::Cursor;
use std::io::Error;
use std::str;
use std::time::Duration;

pub struct KafkaWrapper {
    consumer: BaseConsumer,
}

impl KafkaWrapper {
    pub fn new(brokers: &str) -> KafkaWrapper {
        KafkaWrapper {
            consumer: build_consumer(brokers),
        }
    }

    fn get_metadata(&self) -> Metadata {
        let timeout: u64 = 3000;
        self.consumer
            .fetch_metadata(None, Duration::from_millis(timeout))
            .expect("Failed to fetch metadata")
    }

    pub fn get_cluster_infos(&self) -> ClusterInfo {
        let metadata = self.get_metadata();

        ClusterInfo {
            broker_count: metadata.brokers().len(),
            topic_count: metadata.topics().len(),
            broker_name: metadata.orig_broker_name().to_string(),
            broker_id: metadata.orig_broker_id(),
        }
    }

    pub fn get_topic_infos(&self) -> Vec<TopicInfo> {
        trace!("get topics infos");
        let metadata = self.get_metadata();

        let mut topic_infos = Vec::new();

        for topic in metadata.topics() {
            let name = topic.name().to_owned();
            let nb_partitions = topic.partitions().len();
            topic_infos.push(TopicInfo {
                name,
                nb_partitions,
            })
        }
        topic_infos
    }

    pub fn get_group_infos(&self) -> Vec<GroupInfo> {
        trace!("get group infos");
        let mut group_infos: Vec<GroupInfo> = vec![];
        let group_list = self
            .consumer
            .fetch_group_list(None, Duration::from_millis(60000))
            .expect("Failed to fetch group list");

        for group in group_list.groups() {
            trace!("group {}", group.name());
            let name = group.name().to_string();
            let state = group.state().to_string();
            let mut members: Vec<GroupMember> = vec![];

            if group.protocol_type() == "consumer" {
                for member in group.members() {
                    let member_id = member.id().to_string();
                    let client_host = member.client_host().to_string();
                    let client_id = member.client_id().to_string();
                    let mut assignments: Vec<MemberAssignment> = vec![];
                    if let Some(ass) = member.assignment() {
                        let mut cursor = Cursor::new(ass);
                        assignments =
                            parse_member_assignment(&mut cursor).expect("can't read assignments");
                    }
                    let group_member = GroupMember {
                        id: member_id,
                        client_id,
                        client_host,
                        assignments,
                    };
                    members.push(group_member);
                }
            }
            let group_info = GroupInfo {
                name,
                state,
                members,
            };
            group_infos.push(group_info)
        }
        group_infos
    }

    pub fn get_topic_detail(&self, topic_name: &str) -> Option<TopicDetail> {
        let metadata = self.get_metadata();

        let found_topic = metadata.topics().iter().find(|mt| mt.name() == topic_name);

        if let Some(topic) = found_topic {
            let mut message_count = 0;
            let info = TopicInfo {
                name: topic.name().to_owned(),
                nb_partitions: topic.partitions().len(),
            };
            let mut offsets: Vec<PartitionOffsets> = vec![];
            for partition in topic.partitions() {
                let id = partition.id();
                let leader = partition.leader();
                let _isr = partition.isr();
                let (low, high) = self
                    .consumer
                    .fetch_watermarks(topic.name(), partition.id(), Duration::from_secs(1))
                    .unwrap_or((-1, -1));
                let count = high - low;
                message_count += count;
                let partition_offsets = PartitionOffsets {
                    leader,
                    high,
                    count,
                    low,
                    id,
                };
                offsets.push(partition_offsets);
            }
            return Some(TopicDetail {
                info,
                message_count,
                offsets,
            });
        }
        None
    }
}

pub fn build_offset_consumer(brokers: &str) -> StreamConsumer {
    let offset_consumer = ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set(
            "ssl.key.location",
            "/home/fblarel/workspaces/kafkahq/certs/dev/aiven-service.key",
        )
        .set(
            "ssl.certificate.location",
            "/home/fblarel/workspaces/kafkahq/certs/dev/aiven-service.cert",
        )
        .set(
            "ssl.ca.location",
            "/home/fblarel/workspaces/kafkahq/certs/dev/aiven-ca.pem",
        )
        .set("security.protocol", "ssl")
        .set("enable.partition.eof", "true")
        .set("session.timeout.ms", "30000")
        .set("enable.auto.commit", "false")
        .set("group.id", "test-offset-reader-group")
        .set("auto.offset.reset", "smallest")
        .set_log_level(RDKafkaLogLevel::Debug)
        .create::<StreamConsumer>()
        .expect("failed to create offset consumer");

    offset_consumer
}

fn build_consumer(brokers: &str) -> BaseConsumer {
    let consumer: BaseConsumer = ClientConfig::new()
        .set("bootstrap.servers", brokers)
        .set(
            "ssl.key.location",
            "/home/fblarel/workspaces/kafkahq/certs/dev/aiven-service.key",
        )
        .set(
            "ssl.certificate.location",
            "/home/fblarel/workspaces/kafkahq/certs/dev/aiven-service.cert",
        )
        .set(
            "ssl.ca.location",
            "/home/fblarel/workspaces/kafkahq/certs/dev/aiven-ca.pem",
        )
        .set("security.protocol", "ssl")
        .create()
        .expect("Consumer creation failed");
    consumer
}

// pub fn print_metadata(brokers: &str) {
//
//     // to check
//     let admin = AdminClient::from_config(ClientConfig::new().set("bootstrap.servers", brokers))
//         .expect("cannot create admin");
//     let _aruc = admin.describe_configs(
//         &[ResourceSpecifier::Broker(1)],
//         AdminOptions::new().borrow(),
//     );
// }

// crash si on consomme plusieurs topics?
fn parse_member_assignment(
    payload_rdr: &mut Cursor<&[u8]>,
) -> Result<Vec<MemberAssignment>, Error> {
    let _version = payload_rdr.read_i16::<BigEndian>()?;
    let assign_len = payload_rdr.read_i32::<BigEndian>()?;
    let mut assigns = Vec::with_capacity(assign_len as usize);
    for _ in 0..assign_len {
        let topic = read_str(payload_rdr)?.to_owned();
        let partition_len = payload_rdr.read_i32::<BigEndian>()?;
        let mut partitions = Vec::with_capacity(partition_len as usize);
        for _ in 0..partition_len {
            let partition = payload_rdr.read_i32::<BigEndian>()?;
            partitions.push(partition);
        }
        assigns.push(MemberAssignment { topic, partitions })
    }
    Ok(assigns)
}

// pub fn read_str<'a>(rdr: &'a mut Cursor<&[u8]>) -> Result<&'a str, Error> {
//     let len = (rdr.read_i16::<BigEndian>())? as usize;
//     let pos = rdr.position() as usize;
//     let slice = str::from_utf8(&rdr.get_ref()[pos..(pos + len)]).expect("kaboom");
//     rdr.consume(len);
//     Ok(slice)
// }
