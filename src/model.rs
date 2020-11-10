use crate::utils::read_str;
use byteorder::{BigEndian, ReadBytesExt};
use serde::export::TryFrom;
use std::hash::Hash;
use std::io::{Cursor, Error};

pub enum Event<K> {
    Tick,
    Input(K),
}

pub struct MemberAssignment {
    pub topic: String,
    pub partitions: Vec<i32>,
}

pub struct GroupMember {
    pub id: String,
    pub client_id: String,
    pub client_host: String,
    pub assignments: Vec<MemberAssignment>,
}

impl GroupMember {
    fn consume_topic(&self, topic: &str) -> bool {
        self.assignments.iter().any(|ass| ass.topic == topic)
    }
}

pub struct GroupInfo {
    pub name: String,
    pub state: String,
    pub members: Vec<GroupMember>,
}

impl GroupInfo {
    pub fn consume_topic(&self, topic: &str) -> bool {
        self.members.iter().any(|m| m.consume_topic(topic))
    }
}

pub struct PartitionOffsets {
    pub low: i64,
    pub high: i64,
    pub count: i64,
    pub id: i32,
    pub leader: i32,
    // isr: [i32],
}
pub struct TopicInfo {
    pub name: String,
    pub nb_partitions: usize,
}

pub struct TopicDetail {
    pub info: TopicInfo,
    pub message_count: i64,
    pub offsets: Vec<PartitionOffsets>,
}

pub struct ClusterInfo {
    pub broker_count: usize,
    pub topic_count: usize,
    pub broker_name: String,
    pub broker_id: i32,
}

pub struct OffsetValue {
    pub offset: i64,
}

impl TryFrom<&[u8]> for OffsetValue {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let mut val_rdr = Cursor::new(bytes);
        val_rdr.read_i16::<BigEndian>()?;
        let offset = val_rdr.read_i64::<BigEndian>()?.to_owned();
        // let metadata = read_str(&mut val_rdr)?.to_owned();
        Ok(OffsetValue { offset })
    }
}

#[derive(PartialEq, Eq, Hash)]
pub enum OffsetAndMetadata {
    OffsetKey {
        group: String,
        topic: String,
        partition: i32,
    },
    GroupMetadataKey {
        group: String,
    },
}
impl TryFrom<&[u8]> for OffsetAndMetadata {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let mut key_rdr = Cursor::new(bytes);
        let version = key_rdr.read_i16::<BigEndian>()?;

        let group = read_str(&mut key_rdr)?.to_owned();
        if version > 1 {
            return Ok(OffsetAndMetadata::GroupMetadataKey { group });
        }
        let topic = read_str(&mut key_rdr)?.to_owned();
        let partition = key_rdr.read_i32::<BigEndian>()?;
        Ok(OffsetAndMetadata::OffsetKey {
            group,
            topic,
            partition,
        })
    }
}
