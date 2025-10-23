use crate::result::Result;
use std::{collections::HashMap, io::Cursor};
use tokio::io::{AsyncRead, AsyncReadExt};

#[derive(Clone, Debug, Default)]
pub struct DataPacket {
    pub fields: HashMap<String, String>,
}

// methods regarding the creation of headers on the server side
impl DataPacket {
    pub fn new() -> Self {
        Self {
            fields: HashMap::default(),
        }
    }
    pub fn insert(&mut self, key: String, value: String) {
        self.fields.insert(key, value);
    }
    pub fn remove(&mut self, key: &str) {
        self.fields.remove(key);
    }
    pub fn get(&self, key: &str) -> Result<&str> {
        // using results for easy error mapping
        match self.fields.get(key) {
            Some(v) => Ok(v),
            None => Err(format!("Can't find value of field {}.", key).into()),
        }
    }
}

// regarding the stream encoding and decoding
impl DataPacket {
    pub async fn decode(stream: &mut (impl AsyncRead + Unpin)) -> Result<Self> {
        let mut fields = HashMap::new();
        loop {
            let field_size = stream.read_u32_le().await?;
            if field_size == 0 {
                break;
            }
            println!("field size : {}", field_size);

            let mut field_raw = vec![0u8; field_size as usize];
            stream.read_exact(&mut field_raw).await?;
            let field_str = String::from_utf8(field_raw)?;
            println!("field : {}", field_str);
            match field_str.split_once(":") {
                Some((field_title, field_value)) => {
                    fields.insert(field_title.to_owned(), field_value.to_owned());
                }
                None => {
                    return Err("Invalid packet structure splitter: not found".into());
                }
            }
        }
        Ok(DataPacket { fields })
    }
    pub fn encode(&self) -> impl AsyncRead + Unpin {
        let fields = self.fields.clone();
        let mut buf = Vec::new();
        fields.iter().for_each(|(key, value)| {
            buf.extend_from_slice(&((key.len() + value.len() + 1) as u32).to_le_bytes());
            buf.extend_from_slice(key.as_bytes());
            buf.extend_from_slice(":".as_bytes());
            buf.extend_from_slice(value.as_bytes());
        });
        buf.extend_from_slice(&0_u32.to_le_bytes());
        Cursor::new(buf)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Cursor;
    #[tokio::test]
    async fn packet_reader_test() {
        let field1 = b"content_size:not a size bro";
        let field2 = b"auth_type:jwt_token";
        let mut buf = Vec::new();
        buf.extend_from_slice(&(field1.len() as u32).to_le_bytes());
        buf.extend_from_slice(field1);
        buf.extend_from_slice(&(field2.len() as u32).to_le_bytes());
        buf.extend_from_slice(field2);
        buf.extend_from_slice(&0_u32.to_le_bytes());
        let mut cursor = Cursor::new(buf);
        let fields = DataPacket::decode(&mut cursor).await.unwrap();
        let auth_type = fields.get("auth_type").unwrap();
        assert_eq!(auth_type, "jwt_token");
    }
}
