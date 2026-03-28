# Security Policy

## 支持的版本

| 版本 | 支持状态 |
|------|----------|
| 0.1.x | :white_check_mark: 当前支持 |

## 报告安全漏洞

RustViking 团队非常重视安全问题。如果您发现了安全漏洞，请通过以下方式 responsibly 地报告：

### 通过 GitHub Security Advisories 报告（推荐）

1. 访问 [GitHub Security Advisories](https://github.com/SpellingDragon/rustviking/security/advisories)
2. 点击 "New draft security advisory"
3. 填写漏洞详细信息：
   - **Title**: 漏洞的简要描述
   - **Description**: 详细的技术描述，包括：
     - 漏洞类型（如：缓冲区溢出、SQL 注入等）
     - 受影响的版本
     - 复现步骤
     - 可能的影响
     - 建议的修复方案（如有）
4. 提交后，维护团队会在 48 小时内确认收到报告

### 通过邮件报告（备用方式）

如果无法使用 GitHub Security Advisories，请发送邮件至：

📧 **security@rustviking.dev** (待设置)

邮件主题格式：`[SECURITY] 简要描述 - RustViking`

邮件内容请包含：
- 漏洞详细描述
- 复现步骤
- 受影响的版本
- 您的联系方式（可选）

## 漏洞处理流程

1. **确认收到** (24-48 小时内): 维护团队确认收到报告并分配跟踪编号
2. **评估分析** (1-2 周): 评估漏洞严重性和影响范围
3. **修复开发** (时间视复杂性而定): 开发修复补丁
4. **预发布通知** (修复发布前 24-48 小时): 向报告者通知修复即将发布
5. **公开发布**: 发布安全更新和 CVE（如适用）

## 安全更新通知

- 安全更新将通过 [GitHub Releases](https://github.com/SpellingDragon/rustviking/releases) 发布
- 建议订阅仓库的 "Watch" -> "Custom" -> "Security alerts" 以获取通知
- 严重漏洞将通过仓库的 Security Advisory 页面公开披露

## 安全最佳实践

使用 RustViking 时，建议遵循以下安全实践：

1. **保持更新**: 及时升级到最新版本
2. **配置文件权限**: 确保 `config.toml` 和数据目录的权限设置正确（建议 600/700）
3. **输入验证**: 对通过 CLI 传入的路径和参数进行验证
4. **网络安全**: 如果启用 gRPC/HTTP 服务，建议使用 TLS 加密
5. **数据备份**: 定期备份 RocksDB 数据目录

## 已知安全限制

- CLI 命令以当前用户权限执行，请确保运行环境的权限配置正确
- 数据文件存储在本地文件系统，请确保物理存储安全

## 致谢

感谢所有负责任地报告安全问题的研究人员和用户。您的贡献使 RustViking 更加安全。

---

*最后更新: 2026-03-29*
