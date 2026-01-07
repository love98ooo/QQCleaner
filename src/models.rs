/// 文件信息
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FileInfo {
    pub client_seq: i64,        // 45001
    pub msg_random: i64,        // 82300
    pub msg_id: i64,            // 40001
    pub filepath: String,       // 45403
    pub thumbpath: String,      // 45404
    pub nt_uid: String,         // 40020
    pub peer_uid: String,       // 40021
    pub chat_type: i64,         // 40010
    pub element_type: i64,      // 45002
    pub sub_element_type: i64,  // 45003
    pub file_name: String,      // 45402
    pub file_size: i64,         // 45405
    pub msg_time: i64,          // 40050
    pub original: i64,          // 82302
    pub actual_size: Option<u64>, // 文件系统实际大小（如果文件存在）
}

/// 群组详细信息
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GroupInfo {
    pub group_id: String,       // 60001 (群号)
    pub group_name: String,     // 60007
    pub group_remark: Option<String>, // 60026
    pub owner_uid: String,      // 60002
    pub create_time: i64,       // 60004
    pub max_member: i64,        // 60005
    pub member_count: i64,      // 60006
    pub quit_flag: i64,         // 60340 (0为群成员，1为已不是群成员)
}

/// 群组统计信息
#[derive(Debug)]
#[allow(dead_code)]
pub struct GroupStats {
    pub group_id: String,
    pub group_name: String,
    pub total_size: u64,        // 总大小（字节）
    pub file_count: usize,      // 文件数量
    pub exist_count: usize,     // 存在的文件数量
    pub missing_count: usize,   // 缺失的文件数量
    pub files: Vec<FileInfo>,
}

impl GroupStats {
    pub fn format_size(&self) -> String {
        format_bytes(self.total_size)
    }
}

/// 格式化字节大小
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
