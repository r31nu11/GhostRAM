//! Deterministic GGUF fixture construction for tests only.

pub(crate) struct GgufBuilder {
    version: u32,
    metadata: Vec<u8>,
    metadata_count: u64,
    tensors: Vec<u8>,
    tensor_count: u64,
}

impl GgufBuilder {
    pub(crate) fn new(version: u32) -> Self {
        Self {
            version,
            metadata: Vec::new(),
            metadata_count: 0,
            tensors: Vec::new(),
            tensor_count: 0,
        }
    }

    pub(crate) fn raw_entry(&mut self, key: &str, value_type: u32, value: &[u8]) {
        encode_string(&mut self.metadata, key);
        self.metadata.extend_from_slice(&value_type.to_le_bytes());
        self.metadata.extend_from_slice(value);
        self.metadata_count += 1;
    }

    pub(crate) fn u8(&mut self, key: &str, value: u8) {
        self.raw_entry(key, 0, &[value]);
    }

    pub(crate) fn i8(&mut self, key: &str, value: i8) {
        self.raw_entry(key, 1, &[value as u8]);
    }

    pub(crate) fn u16(&mut self, key: &str, value: u16) {
        self.raw_entry(key, 2, &value.to_le_bytes());
    }

    pub(crate) fn i16(&mut self, key: &str, value: i16) {
        self.raw_entry(key, 3, &value.to_le_bytes());
    }

    pub(crate) fn u32(&mut self, key: &str, value: u32) {
        self.raw_entry(key, 4, &value.to_le_bytes());
    }

    pub(crate) fn i32(&mut self, key: &str, value: i32) {
        self.raw_entry(key, 5, &value.to_le_bytes());
    }

    pub(crate) fn f32_bits(&mut self, key: &str, value: u32) {
        self.raw_entry(key, 6, &value.to_le_bytes());
    }

    pub(crate) fn boolean(&mut self, key: &str, value: bool) {
        self.raw_entry(key, 7, &[u8::from(value)]);
    }

    pub(crate) fn string(&mut self, key: &str, value: &str) {
        let mut encoded = Vec::new();
        encode_string(&mut encoded, value);
        self.raw_entry(key, 8, &encoded);
    }

    pub(crate) fn u64(&mut self, key: &str, value: u64) {
        self.raw_entry(key, 10, &value.to_le_bytes());
    }

    pub(crate) fn i64(&mut self, key: &str, value: i64) {
        self.raw_entry(key, 11, &value.to_le_bytes());
    }

    pub(crate) fn f64_bits(&mut self, key: &str, value: u64) {
        self.raw_entry(key, 12, &value.to_le_bytes());
    }

    pub(crate) fn array(&mut self, key: &str, element_type: u32, count: u64, payload: &[u8]) {
        let mut encoded = Vec::new();
        encoded.extend_from_slice(&element_type.to_le_bytes());
        encoded.extend_from_slice(&count.to_le_bytes());
        encoded.extend_from_slice(payload);
        self.raw_entry(key, 9, &encoded);
    }

    pub(crate) fn string_array(&mut self, key: &str, values: &[&str]) {
        let mut payload = Vec::new();
        for value in values {
            encode_string(&mut payload, value);
        }
        self.array(
            key,
            8,
            u64::try_from(values.len()).expect("fixture length fits in u64"),
            &payload,
        );
    }

    pub(crate) fn tensor(&mut self, name: &str, dimensions: &[u64], type_id: u32, offset: u64) {
        encode_string(&mut self.tensors, name);
        self.tensors.extend_from_slice(
            &u32::try_from(dimensions.len())
                .expect("fixture dimensions fit in u32")
                .to_le_bytes(),
        );
        for dimension in dimensions {
            self.tensors.extend_from_slice(&dimension.to_le_bytes());
        }
        self.tensors.extend_from_slice(&type_id.to_le_bytes());
        self.tensors.extend_from_slice(&offset.to_le_bytes());
        self.tensor_count += 1;
    }

    pub(crate) fn finish(&self, alignment: u32) -> Vec<u8> {
        self.finish_with_payload(alignment, &[])
    }

    pub(crate) fn finish_with_payload(&self, alignment: u32, payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"GGUF");
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&self.tensor_count.to_le_bytes());
        bytes.extend_from_slice(&self.metadata_count.to_le_bytes());
        bytes.extend_from_slice(&self.metadata);
        bytes.extend_from_slice(&self.tensors);
        if self.tensor_count > 0 {
            let alignment = usize::try_from(alignment).expect("fixture alignment fits usize");
            let padded = align_up(bytes.len(), alignment);
            bytes.resize(padded, 0);
        }
        bytes.extend_from_slice(payload);
        bytes
    }
}

pub(crate) fn align_up(value: usize, alignment: usize) -> usize {
    let remainder = value % alignment;
    if remainder == 0 {
        value
    } else {
        value + (alignment - remainder)
    }
}

fn encode_string(output: &mut Vec<u8>, value: &str) {
    output.extend_from_slice(
        &u64::try_from(value.len())
            .expect("fixture string length fits in u64")
            .to_le_bytes(),
    );
    output.extend_from_slice(value.as_bytes());
}
