use tokio::io::{AsyncRead, AsyncReadExt, BufReader};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum ProtocolMessage {
    RequestMetadata,
    Metadata {
        file_name: String,
        file_size: u64,
        chunk_size: u64,
    },
    RequestChunk {
        chunk_index: u64,
    },
    FileChunk {
        chunk_index: u64,
        data: Vec<u8>,
    },
    TransferComplete,
}

impl ProtocolMessage {
    pub fn framed_serialize(&self) -> Vec<u8> {
        let serialized = bincode::serialize(&self).unwrap();

        let mut buffer = Vec::with_capacity(4 + serialized.len());
        buffer.extend_from_slice(&(serialized.len() as u32).to_be_bytes());
        buffer.extend_from_slice(&serialized);

        return buffer;
    }

    pub async fn framed_deserialize<T>(mut reader: BufReader<T>) -> Result<Self, bincode::Error>
    where
        T: AsyncRead + Unpin,
    {
        let mut length_buf = [0u8; 4];

        reader.read_exact(&mut length_buf).await?;
        let length = u32::from_be_bytes(length_buf) as usize;

        // Read the full message
        let mut buffer = vec![0u8; length];
        reader.read_exact(&mut buffer).await?;

        // Deserialize the ProtocolMessage
        match bincode::deserialize::<ProtocolMessage>(&buffer) {
            Ok(message) => {
                log::info!("Received: {:?}", message);

                Ok(message)
            }
            Err(e) => {
                log::error!("Failed to deserialize message: {:?}", e);
                Err(e)
            }
        }
    }
}
