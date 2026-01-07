# QQCleaner

> [!WARNING]
    > 本项目处于开发初期，功能不完善且可能存在 bug，请谨慎使用

在 QQNT 上实现基于群聊和时间范围层面的图片清理/迁移

## Steps to Use

1. 参考 [QQDecrypt](https://docs.aaqwq.top/about/projects.html) 获取 sqlcipher key
2. 将 QQNT 的 `nt_db` 目录复制到项目根目录
3. 使用 [ntdb_unwrap](https://github.com/artiga033/ntdb_unwrap) 解密 `nt_db` 中的 `files_in_chat.db` 和 `group_info.db` 文件
4. 运行 `cargo run --release`

## Configuration

项目根目录提供 `config.toml` 用于管理常量配置，未创建时程序会使用默认值。

```toml
[paths]
qq_data_base = "Library/Containers/com.tencent.qq/Data/Library/Application Support/QQ"
nt_qq_prefix = "nt_qq_"
nt_data_subpath = "nt_data/Pic"

[database]
db_dir = "nt_db"
files_db_name = "files_in_chat.clean.db"
group_db_name = "group_info.clean.db"
```

如需自定义路径或数据库名称，只需在 `config.toml` 中调整对应项，无需改动代码。

## TODO

- [ ] 支持多账号
- [ ] 支持更多文件类型（视频、语音等）
- [ ] 适配 Windows 平台
- [ ] 支持私聊、频道等场景的图片清理
- [ ] 集成 `ntdb_unwrap` 简化操作流程

## License

本项目采用 MIT 许可证，详见 [LICENSE](LICENSE) 文件。
