use local_ip_address::local_ip;
use std::path::{Path, PathBuf};
use tokio::{
    io::{AsyncSeekExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

use crate::{file_metadata::FileMetaData, protocol::ProtocolMessage};

const CHUNK_SIZE: u64 = 1024 * 1024;

pub struct Sender {
    listener: TcpListener,
    file_metadata: FileMetaData,
}

impl Sender {
    pub fn new(file_path: PathBuf) -> Sender {
        let file_path = file_path.to_str().unwrap().to_string();

        let file_metadata = FileMetaData::get_meta_data_from_path(&Path::new(&file_path));

        log::info!("Initializing sender... file path:{}", file_path);

        let local_address = local_ip().unwrap();
        let listener =
            std::net::TcpListener::bind(format!("{}:0", local_address.to_string())).unwrap();
        let _ = listener.set_nonblocking(true);

        println!(
            "Server is hosted on {}:{}",
            listener.local_addr().unwrap().ip(),
            listener.local_addr().unwrap().port()
        );

        Self {
            file_metadata: file_metadata,
            listener: TcpListener::from_std(listener).unwrap(),
        }
    }

    async fn handle_message_packet(
        message: ProtocolMessage,
        file_metadata: &FileMetaData,
    ) -> Option<ProtocolMessage> {
        match message {
            ProtocolMessage::RequestMetadata => {
                return Some(ProtocolMessage::Metadata {
                    file_name: file_metadata.name.clone(),
                    file_size: file_metadata.size,
                    chunk_size: CHUNK_SIZE,
                })
            }

            ProtocolMessage::RequestChunk { chunk_index } => {
                return Some(
                    Sender::send_chunk(&file_metadata.file_path, chunk_index, file_metadata.size)
                        .await,
                );
            }
            _ => {
                log::warn!("Unknown message packet received");
                return None;
            }
        }
    }

    async fn send_chunk(file_path: &str, chunk_index: u64, file_size: u64) -> ProtocolMessage {
        let start_pos = chunk_index * CHUNK_SIZE;

        if start_pos >= file_size {
            return ProtocolMessage::TransferComplete;
        }

        let mut file = tokio::fs::File::open(file_path).await.unwrap();

        file.seek(std::io::SeekFrom::Start(start_pos))
            .await
            .unwrap();

        let remaining = file_size.saturating_sub(start_pos);
        let actual_read = std::cmp::min(CHUNK_SIZE, remaining) as usize;

        let mut chunk_data = vec![0u8; actual_read];
        let read_result = tokio::io::AsyncReadExt::read_exact(&mut file, &mut chunk_data).await;

        match read_result {
            Ok(_) => ProtocolMessage::FileChunk {
                chunk_index,
                data: chunk_data,
            },
            Err(e) => {
                log::error!("Error reading chunk {}: {}", chunk_index, e);
                ProtocolMessage::TransferComplete
            }
        }
    }

    async fn process_socket(mut socket: TcpStream, file_metadata: FileMetaData) {
        let file_metadata = file_metadata;

        loop {
            let message =
                match ProtocolMessage::framed_deserialize(BufReader::new(&mut socket)).await {
                    Ok(msg) => msg,
                    Err(_) => break, // Connection closed or error, exit loop
                };

            if let Some(response) = Sender::handle_message_packet(message, &file_metadata).await {
                let is_transfer_complete = matches!(response, ProtocolMessage::TransferComplete);
                let framed_response = response.framed_serialize();
                if let Err(e) = socket.write_all(&framed_response).await {
                    log::error!("Failed to send response: {}", e);
                    break;
                }

                if is_transfer_complete {
                    break;
                }
            }
        }
    }

    pub async fn run(self) {
        let listener = self.listener;
        let file_metadata = self.file_metadata;
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let metadata = file_metadata.clone();
            tokio::spawn(async move {
                Sender::process_socket(socket, metadata).await;
            });
        }
    }
}
