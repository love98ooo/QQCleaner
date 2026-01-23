# QQCleaner

> [!WARNING]
> 本项目处于开发初期，功能不完善且可能存在 bug，请谨慎使用
>
> **免责声明**: 本项目仅为教育和研究目的而创建。使用本项目前，用户必须：
>
> 1. **检查当地法律** - 确认相关操作在您的司法管辖区合法
> 2. **遵守服务条款** - 确保使用方式符合相关应用的用户协议
> 3. **仅用于个人用途** - 本项目仅应用于对自己拥有的设备上的数据进行合法操作
> 4. **自行承担风险** - 用户对使用本项目的所有后果自行负责

QQCleaner(sQl based Quick file Cleaner) 是一个针对偏移1024字节的 SQLCipher 数据库的访问工具，并提供了媒体文件管理能力

## Steps to Use

### 1. 准备密钥文件

将密钥保存为 `sqlcipher.key`，放在项目根目录或 `~/.config/qqcleaner/` 目录。

### 2. 准备数据库文件（重要）

**法律声明**：本程序**不会自动访问或复制**任何应用的数据。因此所有数据库文件必须由用户手动复制 `files_in_chat.db` 和 `group_info.db` 到本应用的目录下：

- macOS: `~/Library/Application Support/qqcleaner/nt_db/`
- Windows: `C:\Users\<用户名>\AppData\Roaming\qqcleaner\nt_db\`
- Linux: `~/.local/share/qqcleaner/nt_db/`

### 3. 运行程序

运行 `cargo run --release`

程序会自动解密数据库并启动 TUI 界面。

**首次运行时**：

- 程序会显示实际使用的工作目录路径
- 如果未找到数据库文件，程序会自动打开两个目录：
  - QQ 数据库源目录（从这里复制文件）
  - 工作目录（复制到这里，根据平台不同位置不同）
- 请手动复制数据库文件到工作目录
- 复制完成后重新运行程序
- 程序会自动解密数据库（需要密钥文件）

## 配置

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

如需自定义路径或数据库名称，只需在 `config.toml` 中调整对应项

## TODO

- [ ] 支持多账号
- [ ] 支持更多文件类型（视频、语音等）
- [ ] 适配 Windows 平台
- [ ] 支持私聊、频道等场景的媒体管理
- [x] 集成数据库工具简化操作流程

## License

本项目采用 MIT 许可证，详见 [LICENSE](LICENSE) 文件。
