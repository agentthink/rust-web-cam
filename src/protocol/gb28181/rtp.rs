use std::io::Write;

pub struct RtpAudioPacket {
    pub version: u8,
    pub padding: u8,
    pub extension: u8,
    pub csrc_count: u8,
    pub marker: u8,
    pub payload_type: u8,
    pub sequence_number: u16,
    pub timestamp: u32,
    pub ssrc: u32,
    pub payload: Vec<u8>,
}

impl RtpAudioPacket {
    pub fn new(payload_type: u8, timestamp: u32, sequence_number: u16, ssrc: u32) -> Self {
        Self {
            version: 2,
            padding: 0,
            extension: 0,
            csrc_count: 0,
            marker: 0,
            payload_type,
            sequence_number,
            timestamp,
            ssrc,
            payload: Vec::new(),
        }
    }

    pub fn with_payload(mut self, payload: Vec<u8>) -> Self {
        self.payload = payload;
        self
    }

    pub fn build(&self) -> Vec<u8> {
        let mut buffer = Vec::with_capacity(12 + self.payload.len());

        let b0 =
            (self.version << 6) | (self.padding << 5) | (self.extension << 4) | self.csrc_count;
        let b1 = (self.marker << 7) | self.payload_type;

        buffer.push(b0);
        buffer.push(b1);
        buffer
            .write_all(&self.sequence_number.to_be_bytes())
            .unwrap();
        buffer.write_all(&self.timestamp.to_be_bytes()).unwrap();
        buffer.write_all(&self.ssrc.to_be_bytes()).unwrap();
        buffer.extend_from_slice(&self.payload);

        buffer
    }

    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 12 {
            return None;
        }

        let b0 = data[0];
        let b1 = data[1];

        let version = (b0 >> 6) & 0x03;
        let padding = (b0 >> 5) & 0x01;
        let extension = (b0 >> 4) & 0x01;
        let csrc_count = b0 & 0x0F;
        let marker = (b1 >> 7) & 0x01;
        let payload_type = b1 & 0x7F;

        let sequence_number = u16::from_be_bytes([data[2], data[3]]);
        let timestamp = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let ssrc = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);

        let payload = data[12..].to_vec();

        Some(Self {
            version,
            padding,
            extension,
            csrc_count,
            marker,
            payload_type,
            sequence_number,
            timestamp,
            ssrc,
            payload,
        })
    }
}

pub struct RtpAudioBuilder {
    payload_type: u8,
    ssrc: u32,
    sequence_number: u16,
    timestamp: u32,
    timestamp_increment: u32,
}

impl RtpAudioBuilder {
    pub fn new(payload_type: u8, ssrc: u32) -> Self {
        Self {
            payload_type,
            ssrc,
            sequence_number: rand_u16(),
            timestamp: rand_u32(),
            timestamp_increment: 160,
        }
    }

    pub fn with_timestamp_increment(mut self, increment: u32) -> Self {
        self.timestamp_increment = increment;
        self
    }

    pub fn build_packet(&mut self, payload: Vec<u8>) -> Vec<u8> {
        let packet = RtpAudioPacket::new(
            self.payload_type,
            self.timestamp,
            self.sequence_number,
            self.ssrc,
        )
        .with_payload(payload);

        let bytes = packet.build();

        self.sequence_number = self.sequence_number.wrapping_add(1);
        self.timestamp = self.timestamp.wrapping_add(self.timestamp_increment);

        bytes
    }

    pub fn next_timestamp(&mut self) {
        self.timestamp = self.timestamp.wrapping_add(self.timestamp_increment);
    }
}

fn rand_u16() -> u16 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    ((now.as_nanos() & 0xFFFF) as u16).wrapping_add(1)
}

fn rand_u32() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    (now.as_nanos() as u32).wrapping_add(1)
}

pub const RTP_TICK_RATE_AUDIO: u32 = 8000;
pub const RTP_TIMESTAMP_INCREMENT_20MS: u32 = 160;
pub const RTP_TIMESTAMP_INCREMENT_30MS: u32 = 240;

pub fn build_rtp_header(
    payload_type: u8,
    sequence: u16,
    timestamp: u32,
    ssrc: u32,
    marker: u8,
) -> Vec<u8> {
    let mut header = vec![0u8; 12];
    header[0] = 0x80;
    header[1] = (marker << 7) | (payload_type & 0x7F);
    header[2..4].copy_from_slice(&sequence.to_be_bytes());
    header[4..8].copy_from_slice(&timestamp.to_be_bytes());
    header[8..12].copy_from_slice(&ssrc.to_be_bytes());
    header
}

pub fn build_audio_rtp_packet(
    payload: &[u8],
    payload_type: u8,
    timestamp: u32,
    ssrc: u32,
) -> Vec<u8> {
    build_audio_rtp_packet_with_seq(payload, payload_type, timestamp, ssrc, 0)
}

pub fn build_audio_rtp_packet_with_seq(
    payload: &[u8],
    payload_type: u8,
    timestamp: u32,
    ssrc: u32,
    sequence: u16,
) -> Vec<u8> {
    let mut packet = build_rtp_header(payload_type, sequence, timestamp, ssrc, 0);
    packet.extend_from_slice(payload);
    packet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtp_packet_build_parse() {
        let payload = vec![0x01, 0x02, 0x03, 0x04];
        let packet = RtpAudioPacket::new(8, 1000, 1, 0x12345678).with_payload(payload.clone());

        let bytes = packet.build();
        assert!(bytes.len() >= 12 + payload.len());

        let parsed = RtpAudioPacket::parse(&bytes).unwrap();
        assert_eq!(parsed.payload_type, 8);
        assert_eq!(parsed.timestamp, 1000);
        assert_eq!(parsed.sequence_number, 1);
        assert_eq!(parsed.ssrc, 0x12345678);
    }

    #[test]
    fn test_rtp_audio_builder() {
        let mut builder = RtpAudioBuilder::new(8, 0x12345678).with_timestamp_increment(160);

        let payload1 = vec![0x01, 0x02, 0x03];
        let packet1 = builder.build_packet(payload1.clone());
        assert!(packet1.len() >= 15);

        let payload2 = vec![0x05, 0x06, 0x07];
        let packet2 = builder.build_packet(payload2);
        assert!(packet2.len() >= 15);
    }
}
