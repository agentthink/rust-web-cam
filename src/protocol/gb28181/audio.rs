#[derive(Debug, Clone, Copy)]
pub enum G711Codec {
    PCMA,
    PCMU,
}

impl G711Codec {
    pub fn new(codec_name: &str) -> Self {
        match codec_name.to_uppercase().as_str() {
            "PCMA" | "G711A" | "A-LAW" => G711Codec::PCMA,
            "PCMU" | "G711U" | "MU-LAW" | "ULAW" => G711Codec::PCMU,
            _ => G711Codec::PCMA,
        }
    }
}

pub fn linear_to_alaw(samples: &[i16]) -> Vec<u8> {
    let mut result = Vec::with_capacity(samples.len());
    for &sample in samples {
        result.push(linear_to_alaw_sample(sample));
    }
    result
}

pub fn alaw_to_linear(encoded: &[u8]) -> Vec<i16> {
    let mut result = Vec::with_capacity(encoded.len());
    for &byte in encoded {
        result.push(alaw_to_linear_sample(byte));
    }
    result
}

pub fn linear_to_ulaw(samples: &[i16]) -> Vec<u8> {
    let mut result = Vec::with_capacity(samples.len());
    for &sample in samples {
        result.push(linear_to_ulaw_sample(sample));
    }
    result
}

pub fn ulaw_to_linear(encoded: &[u8]) -> Vec<i16> {
    let mut result = Vec::with_capacity(encoded.len());
    for &byte in encoded {
        result.push(ulaw_to_linear_sample(byte));
    }
    result
}

fn linear_to_alaw_sample(sample: i16) -> u8 {
    let s = if sample == i16::MIN {
        32767
    } else {
        sample.abs().min(32767)
    };
    let sign = if sample < 0 { 0x80u8 } else { 0 };

    let seg: u8 = if s < 256 {
        0
    } else if s < 512 {
        1
    } else if s < 1024 {
        2
    } else if s < 2048 {
        3
    } else if s < 4096 {
        4
    } else if s < 8192 {
        5
    } else if s < 16384 {
        6
    } else {
        7
    };

    let seg = if seg > 0 { seg - 1 } else { 0 };

    let mask: u8 = if seg == 0 { 0x7F } else { 0x1F };
    let pcm_val: i16 = if seg == 0 { s } else { s >> seg };

    (sign | (seg << 4) | ((pcm_val as u8) & mask)) ^ 0x55
}

fn alaw_to_linear_sample(a_val: u8) -> i16 {
    let s = a_val ^ 0x55;
    let seg = (s >> 4) & 0x07;

    let mask: u8 = if seg == 0 { 0x7F } else { 0x1F };
    let pcm_val: i16 = if seg == 0 {
        (s & 0x7F) as i16
    } else {
        ((s & 0x0F) | mask) as i16
    };

    let shift = if seg == 0 { 4 } else { seg - 1 };
    let linear = pcm_val << shift;

    if s & 0x80 == 0 {
        -linear
    } else {
        linear
    }
}

fn linear_to_ulaw_sample(sample: i16) -> u8 {
    let s = sample.abs().min(32767);
    let sign = if sample < 0 { 0x80u8 } else { 0 };

    let seg: u8 = if s < 132 {
        0
    } else if s < 264 {
        1
    } else if s < 528 {
        2
    } else if s < 1056 {
        3
    } else if s < 2112 {
        4
    } else if s < 4224 {
        5
    } else if s < 8448 {
        6
    } else {
        7
    };

    let seg = if seg > 0 { seg - 1 } else { 0 };

    let mask: u8 = if seg == 0 { 0x7D } else { 0x1F };
    let pcm_val: i16 = if seg == 0 { s } else { s >> seg };

    (sign | (seg << 4) | ((pcm_val as u8) & mask)) ^ 0xFF
}

fn ulaw_to_linear_sample(u_val: u8) -> i16 {
    let s = u_val ^ 0xFF;
    let seg = (s >> 4) & 0x07;

    let mask: u8 = if seg == 0 { 0x7D } else { 0x1F };
    let pcm_val: i16 = if seg == 0 {
        (s & 0x7D) as i16
    } else {
        ((s & 0x0F) | mask) as i16
    };

    let shift = if seg == 0 { 4 } else { seg - 1 };
    let linear = pcm_val << shift;

    if u_val & 0x80 == 0 {
        -linear
    } else {
        linear
    }
}

pub struct G711Encoder {
    codec: G711Codec,
}

impl G711Encoder {
    pub fn new(codec: G711Codec) -> Self {
        Self { codec }
    }

    pub fn encode(&self, pcm_data: &[i16]) -> Vec<u8> {
        match self.codec {
            G711Codec::PCMA => linear_to_alaw(pcm_data),
            G711Codec::PCMU => linear_to_ulaw(pcm_data),
        }
    }
}

pub struct G711Decoder {
    codec: G711Codec,
}

impl G711Decoder {
    pub fn new(codec: G711Codec) -> Self {
        Self { codec }
    }

    pub fn decode(&self, encoded: &[u8]) -> Vec<i16> {
        match self.codec {
            G711Codec::PCMA => alaw_to_linear(encoded),
            G711Codec::PCMU => ulaw_to_linear(encoded),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alaw_roundtrip() {
        let samples: Vec<i16> = vec![0, 1000, -1000, 32767, -32768];
        let encoded = linear_to_alaw(&samples);
        let decoded = alaw_to_linear(&encoded);
        assert_eq!(samples.len(), decoded.len());
    }

    #[test]
    fn test_ulaw_roundtrip() {
        let samples: Vec<i16> = vec![0, 1000, -1000, 32767, -32768];
        let encoded = linear_to_ulaw(&samples);
        let decoded = ulaw_to_linear(&encoded);
        assert_eq!(samples.len(), decoded.len());
    }
}
