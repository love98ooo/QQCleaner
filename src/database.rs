use anyhow::{Context, Result};
use rusqlite::{Connection};
use std::collections::HashMap;
use std::path::Path;

use crate::models::{FileInfo, GroupInfo};

pub struct Database {
    files_conn: Connection,
    group_conn: Connection,
}

impl Database {
    pub fn new<P: AsRef<Path>>(files_db: P, group_db: P) -> Result<Self> {
        let files_conn = Connection::open(files_db)
            .context("无法打开 files_in_chat.clean.db")?;
        let group_conn = Connection::open(group_db)
            .context("无法打开 group_info.clean.db")?;

        Ok(Database {
            files_conn,
            group_conn,
        })
    }

    pub fn get_all_files(&self) -> Result<Vec<FileInfo>> {
        let mut stmt = self.files_conn.prepare(
            "SELECT `45001`, `82300`, `40001`, `45403`, `45404`, `40020`, `40021`,
                    `40010`, `45002`, `45003`, `45402`, `45405`, `40050`, `82302`
             FROM files_in_chat_table"
        )?;

        let files = stmt.query_map([], |row| {
            Ok(FileInfo {
                client_seq: row.get(0)?,
                msg_random: row.get(1)?,
                msg_id: row.get(2)?,
                filepath: row.get(3).unwrap_or_default(),
                thumbpath: row.get(4).unwrap_or_default(),
                nt_uid: row.get(5).unwrap_or_default(),
                peer_uid: row.get(6).unwrap_or_default(),
                chat_type: row.get(7)?,
                element_type: row.get(8)?,
                sub_element_type: row.get(9)?,
                file_name: row.get(10).unwrap_or_default(),
                file_size: row.get(11).unwrap_or(0),
                msg_time: row.get(12)?,
                original: row.get(13).unwrap_or(0),
                actual_size: None,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(files)
    }

    pub fn get_all_groups(&self) -> Result<HashMap<String, GroupInfo>> {
        let mut stmt = self.group_conn.prepare(
            "SELECT `60001`, `60007`, `60026`, `60002`, `60004`, `60005`, `60006`, `60340`
             FROM group_detail_info_ver1"
        )?;

        let mut groups = HashMap::new();
        let group_iter = stmt.query_map([], |row| {
            let group_id: i64 = row.get(0)?;
            Ok(GroupInfo {
                group_id: group_id.to_string(),
                group_name: row.get(1).unwrap_or_else(|_| format!("群 {}", group_id)),
                group_remark: row.get(2).ok(),
                owner_uid: row.get(3).unwrap_or_default(),
                create_time: row.get(4).unwrap_or(0),
                max_member: row.get(5).unwrap_or(0),
                member_count: row.get(6).unwrap_or(0),
                quit_flag: row.get(7).unwrap_or(0),
            })
        })?;

        for group_result in group_iter {
            if let Ok(group) = group_result {
                groups.insert(group.group_id.clone(), group);
            }
        }

        Ok(groups)
    }

    pub fn group_files_by_peer(&self) -> Result<HashMap<String, Vec<FileInfo>>> {
        let files = self.get_all_files()?;
        let mut grouped: HashMap<String, Vec<FileInfo>> = HashMap::new();

        for file in files {
            if file.chat_type == 2 {
                grouped.entry(file.peer_uid.clone())
                    .or_insert_with(Vec::new)
                    .push(file);
            }
        }

        Ok(grouped)
    }
}
