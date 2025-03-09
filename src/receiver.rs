use crate::{protocol::ProtocolMessage, utils::write_at};
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::mpsc,
};

use indicatif::{ProgressBar, ProgressStyle};

const CONCURRENT_THRESHOLD: u64 = 100 * 1024 * 1024; // 100MB
const NUM_WORKERS: usize = 5;

pub struct Receiver {
    sender_address: String,
}

impl Receiver {
    pub fn new(sender_address: String) -> Receiver {
        log::info!("Receiver created");
        Self { sender_address }
    }

    pub async fn run(&self) {
        log::info!("Receiver running");

        let mut stream = TcpStream::connect(&self.sender_address).await.unwrap();

        // Request metadata first
        let metadata_msg = ProtocolMessage::RequestMetadata;
        stream
            .write_all(&metadata_msg.framed_serialize())
            .await
            .unwrap();

        if let Ok(ProtocolMessage::Metadata {
            file_name,
            file_size,
            chunk_size,
        }) = ProtocolMessage::framed_deserialize(BufReader::new(&mut stream)).await
        {
            if file_size > CONCURRENT_THRESHOLD {
                log::info!("Using concurrent download for large file");
                self.download_concurrent(file_name, file_size, chunk_size)
                    .await;
            } else {
                log::info!("Using sequential download for small file");
                self.download_sequential(stream, file_name, file_size, chunk_size)
                    .await;
            }
        }
    }

    async fn download_sequential(
        &self,
        mut stream: TcpStream,
        file_name: String,
        file_size: u64,
        chunk_size: u64,
    ) {
        let chunk_count = (file_size + chunk_size - 1) / chunk_size;

        let mut file = tokio::fs::File::create(&file_name)
            .await
            .expect("Failed to create file");
        file.set_len(file_size)
            .await
            .expect("Failed to pre-allocate file");

        let progress_bar = ProgressBar::new(chunk_count);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg} {wide_bar} {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("##-"),
        );
        progress_bar.set_message(format!("Downloading {}", file_name));

        for chunk_index in 0..chunk_count {
            let request = ProtocolMessage::RequestChunk { chunk_index };
            stream.write_all(&request.framed_serialize()).await.unwrap();

            match ProtocolMessage::framed_deserialize(BufReader::new(&mut stream)).await {
                Ok(ProtocolMessage::FileChunk {
                    chunk_index: _,
                    data,
                }) => {
                    let offset = chunk_index * chunk_size;
                    write_at(&mut file, offset, data).await;

                    progress_bar.inc(1);
                }
                Ok(ProtocolMessage::TransferComplete) => break,
                _ => log::error!("Unexpected response"),
            }
        }
        progress_bar.finish_with_message("Download completed");
    }

    async fn download_concurrent(&self, file_name: String, file_size: u64, chunk_size: u64) {
        let chunk_count = (file_size + chunk_size - 1) / chunk_size;
        let (tx, mut rx) = mpsc::channel(100);

        let mut file = tokio::fs::File::create(&file_name)
            .await
            .expect("Failed to create file");
        file.set_len(file_size)
            .await
            .expect("Failed to pre-allocate file");

        let progress_bar = ProgressBar::new(chunk_count);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg} {wide_bar} {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("##-"),
        );
        progress_bar.set_message(format!("Downloading {}", file_name));

        let mut workers = Vec::with_capacity(NUM_WORKERS);
        let chunks_per_worker = chunk_count as usize / NUM_WORKERS;
        let remainder = chunk_count as usize % NUM_WORKERS;

        for worker_id in 0..NUM_WORKERS {
            let tx = tx.clone();
            let sender_address = self.sender_address.clone();
            let start = worker_id * chunks_per_worker + worker_id.min(remainder);
            let end = start + chunks_per_worker + if worker_id < remainder { 1 } else { 0 };

            workers.push(tokio::spawn(async move {
                let mut stream = TcpStream::connect(&sender_address)
                    .await
                    .expect("Worker failed to connect");

                // Worker needs to request metadata first to establish protocol
                let metadata_msg = ProtocolMessage::RequestMetadata;
                stream
                    .write_all(&metadata_msg.framed_serialize())
                    .await
                    .unwrap();

                for chunk_index in start..end {
                    if chunk_index as u64 >= chunk_count {
                        break;
                    }

                    let request = ProtocolMessage::RequestChunk {
                        chunk_index: chunk_index as u64,
                    };
                    stream.write_all(&request.framed_serialize()).await.unwrap();

                    match ProtocolMessage::framed_deserialize(BufReader::new(&mut stream)).await {
                        Ok(ProtocolMessage::FileChunk {
                            chunk_index: idx,
                            data,
                        }) => {
                            tx.send((idx, data)).await.expect("Channel closed");
                        }
                        Ok(ProtocolMessage::TransferComplete) => break,
                        _ => log::error!("Worker {} received invalid response", worker_id),
                    }
                }
            }));
        }

        // Collect chunks
        let mut received_chunks = 0;
        while let Some((index, data)) = rx.recv().await {
            let offset = index * chunk_size;
            write_at(&mut file, offset, data).await;

            progress_bar.inc(1);

            received_chunks += 1;
            if received_chunks >= chunk_count {
                break;
            }
        }

        // Wait for all workers to finish
        for worker in workers {
            worker.await.expect("Worker task failed");
        }
        progress_bar.finish_with_message("Download completed");
    }

    
}
