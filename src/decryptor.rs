use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::fs;
use ntdb_unwrap::db::{register_offset_vfs, try_decrypt_db, export_to_plain, OFFSET_VFS_NAME};
use ntdb_unwrap::ntqq::DBDecryptInfo;
use rusqlite::Connection;

pub struct Decryptor {
    key_path: PathBuf,
}

impl Decryptor {
    pub fn new() -> Result<Self> {
        let key_path = Self::find_key_file()
            .context("未找到 sqlcipher.key 文件")?;
        
        register_offset_vfs()
            .map_err(|e| anyhow::anyhow!("注册 offset VFS 失败: {:?}", e))?;
        
        Ok(Decryptor { key_path })
    }

    fn find_key_file() -> Option<PathBuf> {
        let current_dir_key = std::env::current_dir()
            .ok()
            .map(|p| p.join("sqlcipher.key"));

        let user_config_key = dirs::config_dir()
            .map(|p| p.join("qqcleaner").join("sqlcipher.key"));

        [current_dir_key, user_config_key]
            .into_iter()
            .flatten()
            .find(|p| p.exists())
    }

    fn read_key(&self) -> Result<String> {
        let key = fs::read_to_string(&self.key_path)
            .with_context(|| format!("无法读取密钥文件: {:?}", self.key_path))?;
        
        Ok(key.trim().to_string())
    }

    pub fn decrypt_database<P: AsRef<Path>>(&self, encrypted_db: P, output_db: P) -> Result<()> {
        let encrypted_path = encrypted_db.as_ref();
        let output_path = output_db.as_ref();

        if !encrypted_path.exists() {
            bail!("加密数据库不存在: {:?}", encrypted_path);
        }

        let key = self.read_key()?;

        let conn = Connection::open(format!("file:{}?vfs={}", 
            encrypted_path.display(), 
            OFFSET_VFS_NAME
        ))
        .with_context(|| format!("打开加密数据库失败: {:?}", encrypted_path))?;

        let decrypt_info = DBDecryptInfo {
            key,
            cipher_hmac_algorithm: None,
        };
        
        try_decrypt_db(&conn, decrypt_info)
            .map_err(|e| anyhow::anyhow!("解密失败: {:?}", e))?;

        export_to_plain(&conn, output_path)
            .map_err(|e| anyhow::anyhow!("导出普通数据库失败: {:?}", e))?;

        Ok(())
    }

    pub fn decrypt_databases<P: AsRef<Path>>(
        &self,
        nt_db_dir: P,
        output_dir: P,
        db_names: &[&str],
    ) -> Result<()> {
        let nt_db_path = nt_db_dir.as_ref();
        let output_path = output_dir.as_ref();

        if !nt_db_path.exists() {
            bail!("nt_db 目录不存在: {:?}", nt_db_path);
        }

        if !output_path.exists() {
            fs::create_dir_all(output_path)
                .with_context(|| format!("创建输出目录失败: {:?}", output_path))?;
        }

        for db_name in db_names {
            let encrypted_db = nt_db_path.join(db_name);
            let output_db = output_path.join(format!("{}.clean.db", 
                db_name.trim_end_matches(".db")));

            if encrypted_db.exists() {
                println!("正在解密: {}", db_name);
                self.decrypt_database(&encrypted_db, &output_db)
                    .with_context(|| format!("解密 {} 失败", db_name))?;
                println!("✓ 解密成功: {}", db_name);
            } else {
                println!("跳过不存在的数据库: {}", db_name);
            }
        }

        Ok(())
    }

    pub fn get_key_path(&self) -> &Path {
        &self.key_path
    }
}
