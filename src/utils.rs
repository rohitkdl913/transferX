use std::io::SeekFrom;

use tokio::{
    fs::File,
    io::{AsyncSeekExt, AsyncWriteExt},
};

pub async fn write_at(file: &mut File, pos: u64, data: Vec<u8>) {
    file.seek(SeekFrom::Start(pos)).await.expect("Seek failed");
    file.write_all(&data).await.expect("Write failed");
}

async fn assemble_and_save(file_name: &str, mut chunks: Vec<(u64, Vec<u8>)>) {
    // Sort chunks by index
    chunks.sort_unstable_by_key(|(idx, _)| *idx);

    // Create file and write chunks
    let mut file = tokio::fs::File::create(file_name)
        .await
        .expect("Failed to create file");

    for (_, data) in chunks {
        file.write_all(&data).await.expect("Failed to write chunk");
    }

    log::info!("File {} saved successfully", file_name);
}
