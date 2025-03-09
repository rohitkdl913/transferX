use std::{fs, path::Path};

#[derive(Debug, Clone)]
pub struct FileMetaData {
    pub file_path:String,
    pub name: String,
    pub size: u64,
}

impl FileMetaData {
    pub fn get_meta_data_from_path(file_path: &Path) -> Self {
        let file_meta = fs::metadata(&file_path).unwrap();
        let file_size = file_meta.len();
        let file_name = file_path.file_name().unwrap().to_str().unwrap().to_string();
        Self {
            file_path: file_path.to_str().unwrap().to_string(),
            name: file_name,
            size: file_size,
        }
    }
}
