# ClipRTC 同步内核重构设计

> 状态：已确认，进入实施阶段  
> 日期：2026-11-07  
> 范围：Tauri v2 + React 桌面端，LAN-only，Windows/macOS/Linux  
> 目标：把现有“玩具级”实现重构为可长期维护、可测试、可演进的工程级同步内核。  
> 实施进度：Phase 0（身份/信任/协议基础 + CI）已完成；最小 QUIC 双向认证文本传输已完成并通过集成测试；尚未接入 Tauri 运行时。

## 1. 结论

不做整仓库推倒重写。采用**保留外壳 + 重写同步内核**的绞杀式重构：

- 保留：Tauri、React、Tailwind、shadcn、tray、i18n、构建发布。
- 保留底层库：`quinn`（QUIC）、`mdns-sd`、`clipboard-rs`。
- 重写：身份/信任、传输会话、同步协议、文件传输、发现。
- 重构：剪贴板适配、持久化、生命周期、前端状态与 IPC。
- 新建：测试、CI、可观测性、迁移流程。

核心原则：**`cliprtc-core` 不依赖 Tauri**，可以脱离窗口和 WebView 做双端测试。

## 2. 目标与非目标

### 目标

- 设备身份可验证，房间成员不能冒充另一台已有设备。
- 多设备并发、重启、重连、回放、回声抑制行为正确。
- 连接复用，有背压、超时、取消、重试，不静默失败。
- 文件/目录传输可校验、可取消、可清理，路径安全。
- 启动/关闭可预测，无全局 `Lazy` / `OnceCell` 业务状态，无运行时 panic。
- 有单元测试、集成测试、双端 harness、CI。
- 保持“输入同一个房间密钥即可使用”的无感体验。

### 非目标

- 不做跨网/NAT 穿透/relay。
- 不做移动端。
- 不做账号体系、云同步、遥测。
- 不做端到端加密的额外应用层加密（依赖 QUIC/TLS）。
- 不重写 UI 技术栈。

## 3. 已确认决策

| 主题 | 决策 |
|---|---|
| 身份模型 | 房间密钥经 KDF 派生私有 CA；每设备独立密钥；CA 签发设备证书；本地名册绑定 `uuid -> 设备公钥` |
| 新设备 | 有房间密钥即可自动加入；本地 blocklist 撤销；强撤销靠轮换房间密钥 |
| 架构 | 新增 `cliprtc-core` crate；Tauri 只做薄适配 |
| 传输 | 每 peer 一条长期 QUIC 连接；控制流 + 业务流；有界队列；状态机 + 重试 |
| 协议 | `event_id`、`boot_id`、`seq`、Lamport、内容哈希、ACK、LWW |
| 文件 | manifest + 每文件流；路径校验；临时目录 + 原子切换；SHA-256；进度/取消；预留续传 |
| 发现 | mDNS 仅作提示；稳定实例名；按 `uuid` 发事件；地址可达性筛选；保留 daemon 句柄 |
| 剪贴板 | `ClipboardBackend` trait + `ClipboardActor` 串行化；内容哈希回声抑制 |
| 持久化 | Rust 单一数据源；schema + migration；密钥进 keychain |
| 前端 | typed IPC；事件驱动状态；配对/信任/进度/错误 UI |
| 工程 | PR CI、fmt/clippy/test、集成 harness、fuzz/benchmark |

## 4. 威胁模型

### 防护目标

- 被动局域网抓包：QUIC/TLS 加密，看不到剪贴板内容。
- 没有房间密钥的设备：无法完成证书验证，无法加入。
- 房间成员冒充另一台**已有设备**：名册 `uuid -> 设备公钥` 不匹配，拒绝。
- 重放/重复包：`event_id`、`boot_id`、`seq`、Lamport、ACK 去重。
- 路径穿越/符号链接逃逸：接收端全量校验路径，拒绝异常条目。
- 恶意大包/无限流：帧上限、流上限、队列上限、超时。

### 明确不防护

- 拿到房间密钥的人作为**新设备**加入并读取剪贴板。这是共享密钥模型的固有限制。
- 弱房间密钥被离线爆破。默认生成高熵密钥；弱 key 警告。
- 设备被物理攻破、keychain 被读取。
- 局域网内主动 MITM 配合弱 key 的离线猜测。使用强 key 降低风险。
- 用户主动把敏感内容复制到剪贴板。

### 密钥与证书

- 房间密钥：用户共享；默认生成 128-bit 随机值，允许自定义。
- KDF：`HKDF-SHA256` 派生 CA 种子；后续可升级为 `Argon2id`，协议版本中记录 KDF。
- CA：每台设备都能从房间密钥派生同一 CA 私钥；CA 只证明“房间成员资格”，不证明“设备身份”。
- 设备密钥：每设备独立生成，私钥只存本机 keychain；公钥写入证书。
- 名册：首次见到 `uuid` 时记录设备公钥；后续必须一致。
- 证书：CA 签发，SAN 包含 `cliprtc://device/<uuid>`；TLS 双向验证。
- mDNS 中的证书指纹是公开提示，不作为信任根；不再用房间密钥加密指纹。

## 5. 总体架构

```text
┌─────────────────────────────────────────────┐
│ React UI (src/)                             │
│  只通过 typed invoke / event 与后端交互       │
└────────────────────┬────────────────────────┘
                     │ Tauri commands / events
┌────────────────────▼────────────────────────┐
│ Tauri shell (src-tauri/)                    │
│  windows / tray / keychain / paths / logging│
│  薄适配层：命令 -> core；core 事件 -> UI      │
└────────────────────┬────────────────────────┘
                     │
┌────────────────────▼────────────────────────┐
│ cliprtc-core（新 crate）                     │
│                                             │
│  identity    discovery    transport         │
│  protocol    transfer     clipboard         │
│  store       events       error             │
└─────────────────────────────────────────────┘
```

### 模块职责

| 模块 | 职责 | 明确不做什么 |
|---|---|---|
| `identity` | 房间密钥、KDF、CA、设备密钥、证书、名册、信任/撤销 | 不处理网络连接和 UI |
| `discovery` | mDNS 注册/浏览、对端事件、地址筛选、版本信息 | 不做认证，不信任 mDNS 内容 |
| `transport` | QUIC endpoint、每 peer 一个 session、连接复用、超时、重试、背压 | 不解析剪贴板语义 |
| `protocol` | 消息信封、编码/解码、协议版本、seq、boot id、去重、回放窗口 | 不发文件、不碰剪贴板 |
| `transfer` | 文件 manifest、分块、校验、断点、原子落盘、路径安全、进度 | 不决定“什么时候同步” |
| `clipboard` | `ClipboardBackend` trait、采集、应用、去抖、回声抑制、大小策略 | 不做网络传输 |
| `store` | 设置、名册、历史、schema、migration、密钥引用 | 不直接暴露给 UI |
| `core` | 组装模块、事件总线、生命周期、配置、错误上报 | 不写平台 UI 细节 |

### 数据流

```text
本地复制：
  ClipboardActor
      -> LocalClipboardEvent
      -> protocol::encode
      -> SessionManager
      -> transport::send
      -> peers

远端同步：
  transport::recv
      -> protocol::decode
      -> dedup / echo 检查
      -> clipboard::apply
      -> emit UiEvent

设备发现：
  discovery
      -> PeerEvent { found / updated / removed }
      -> SessionManager
      -> emit UiEvent
```

### 硬约束

- 不允许用全局 `Lazy` / `OnceCell` 存业务状态；由 `Arc<Core>` 注入。
- 运行时路径不允许 `unwrap()` / `expect()`；启动失败返回 `Result`，UI 显示错误。
- UI 不直接碰网络、文件、剪贴板；只通过 typed commands/events。
- 每个 peer 一个长期 session，连接复用。
- 所有网络/文件操作都有超时、取消、错误分类。
- 模块之间通过 trait 和事件通信，不直接读写别人的全局 map。
- 不在 `DashMap` guard 上 `.await`。

## 6. 身份与信任

### 首次运行

```text
输入房间密钥 R
    |
    v
KDF(R) -> 房间 CA 私钥/公钥
    |
    v
生成本机设备密钥 D（私钥存 keychain）
    |
    v
CA 给 (uuid, D.pub) 签发设备证书 C
    |
    v
启动 QUIC 服务，mDNS 广播 uuid / port / C 指纹
```

### 连接认证

```text
A <========== QUIC + TLS 双向证书 ==========> B
     |                                          |
     | 验证 C 由房间 CA 签发                     | 验证 C 由房间 CA 签发
     | 提取 C 中的 uuid + 设备公钥               | 提取 C 中的 uuid + 设备公钥
     | 查名册 uuid -> 公钥                       | 查名册 uuid -> 公钥
     |   - 无记录：首次登记，自动加入            |   - 无记录：首次登记，自动加入
     |   - 不一致：拒绝                          |   - 不一致：拒绝
     |                                          |
     | 强制消息 origin == C.uuid                 | 强制消息 origin == C.uuid
```

### 撤销与轮换

- 本地 blocklist：按 `uuid` / 设备公钥拒绝。
- 强撤销：轮换房间密钥，重新派生 CA；设备密钥和 `uuid` 不变，名册不变，不需要重新配对。
- 拿到旧 key 的设备无法再连接。
- 被撤销设备若换新 `uuid` 重新加入，只能靠轮换 key 阻止。

### 密钥存储

- 设备私钥、房间密钥：OS keychain（Windows Credential Manager / macOS Keychain / Linux Secret Service）。
- keychain 不可用时：加密文件 + 明确警告；不允许明文 JSON。
- 旧版本 `key` 字段迁移：保留用户 key；默认 `"cliprtc"` 视为弱 key，UI 提示更换。

## 7. 发现

- 服务类型：`_cliprtc._quic._udp.local.`。
- 实例名：`cliprtc-<uuid>`，稳定唯一；主机名放 TXT，不再用 hostname 当实例名。
- TXT：`v=2`、`uuid`、`port`、`name`、`caps`（压缩/续传/IPv6）、`cert`（证书指纹，公开）。
- 不把房间密钥、设备私钥写入 TXT；不依赖“加密指纹”做认证。
- 保留 `ServiceDaemon` 句柄，支持 shutdown / 重新注册。
- 按 `uuid` 合并多个服务实例；忽略 `uuid == local_uuid`。
- `ServiceResolved`：更新地址/TXT/last_seen；**不重置协议状态**。
- `ServiceRemoved`：必须发 `PeerRemoved`（或标记 offline + grace），修掉 UI 残留。
- 地址选择：
  - 收集 A/AAAA，过滤 loopback/无效地址。
  - 优先同子网，再其他；支持 IPv6。
  - 对多个地址做带超时的连接竞速，选第一个成功的，处理 VPN/Docker/多网卡。
- 版本检查：`v=2` 才连接；旧版显示“需要升级”，不猜测兼容。
- 可选 fallback：手动输入 `IP:port` 直连，解决 mDNS 失败。
- 网络切换/休眠：重置 session，重新注册/浏览；先做 periodical health check，后续接 OS 网络事件。

## 8. 传输与会话

### 状态机

```text
Disconnected
    |
    | discovery 发现 peer 在线
    v
Connecting（5s 超时）
    |
    v
TLS 握手 + 房间 CA 验证
    |
    v
名册检查：uuid -> 设备公钥
    |
    v
Ready  <---- keepalive / 自动重连 ---->  Draining
    |                                      |
    | peer 离线 / 错误 / 退出               v
    +----------------------------------> Closed
```

### 流布局

```text
每个 peer 的 QUIC 连接：

  Control stream（双向，长期）
    Hello / 版本 / 能力 / Ping / Bye / Applied / TransferControl

  Clipboard stream（单向，每个事件一条）
    Text / Image / File

  Transfer streams（单向，每个传输一条或每文件一条）
    Manifest + Chunks + End
```

- 小事件：一个事件一条单向流，天然隔离。
- 大文件：manifest + 每文件流；后续可加 resume。
- 控制流负责握手、能力、ACK、keepalive、关闭；不混业务数据。

### 连接参数

```text
connect_timeout          = 5s
handshake_timeout        = 5s
keep_alive_interval      = 10s
max_idle_timeout         = 30s
max_concurrent_uni       = 16
max_concurrent_bidi      = 4
stream_receive_window    = 4 MiB
receive_window           = 32 MiB
per-peer outbound queue  = 16 个事件 / 64 MiB
```

后续用 benchmark 调整；默认值先保证稳定。

### 背压与重试

- 每 peer 有界队列 + 信号量。
- 文本/图片队列满：保留最新，旧事件合并丢弃。
- 文件队列满：拒绝并通知 UI。
- 连接失败：指数退避（1s, 2s, 4s, ... 最大 30s）。
- 证书/名册/协议版本错误：不重试，UI 报错。
- 网络切换/休眠：立即重置 session 并重连。
- 同时拨号：保留一条，关闭另一条；按 `initiator_uuid < responder_uuid` 决定赢家。

## 9. 同步协议

### 事件信封

```text
ClipboardEvent {
  protocol_version: u16,
  event_id: Uuid,
  origin: Uuid,          // 必须 == TLS 证书 uuid
  boot_id: Uuid,         // 每次启动随机
  seq: u64,              // 本次启动内单调递增
  lamport: u64,          // 逻辑时钟
  kind: Text | Image | File,
  content_hash: [u8; 32],
  size: u64,
}
```

| 字段 | 解决什么 |
|---|---|
| `event_id` | 重复包、重试包去重 |
| `origin` | 防伪造来源 |
| `boot_id` | 设备重启 seq 归零 |
| `seq` | 同一 boot 内快速判断新旧 |
| `lamport` | 不依赖系统时钟的排序 |
| `content_hash` | 回声抑制、内容去重、校验 |

### Lamport

```text
本地复制：
  counter = counter + 1

收到事件：
  counter = max(local, remote) + 1
  如果 (remote.lamport, remote.origin) > (last_applied.lamport, last_applied.origin)
    -> 应用
  否则
    -> 只 ack，不应用
```

### 去重状态

```text
PeerSyncState {
  boot_id,
  last_seq,
  seen_event_ids: LRU,
  last_applied: (lamport, origin)
}
```

- 新 `boot_id`：重置该 peer 的 seq 窗口，信任关系不变。
- 重复 `event_id`：丢弃。
- `seq` 太旧、超出窗口：丢弃。
- 文本/图片：LWW。
- 文件：每个 `event_id` 独立应用，不做 LWW。

### 回声抑制

```text
应用远端内容：
  写入剪贴板
  记录 hash H 到 recent_applied（TTL 2s）

Watcher 看到变化：
  计算 hash H
  如果 H 在 recent_applied 里 -> 跳过
  否则 -> 本地事件
```

所有剪贴板读写走 `ClipboardActor`，串行化。

### ACK 与重试

```text
发送方：
  发送 event
  等 Applied(event_id)（10s）
    - 收到：完成
    - 超时：同一 event_id 重试，最多 2 次
    - 仍失败：UI 通知，保留最新文本/图片，等重连

接收方：
  应用成功后回 Applied(event_id)
  重复 event_id 只 ack，不重复应用
```

### 冲突

- 文本/图片：`(lamport, origin uuid)` 最大者胜。
- 文件：独立事件，全部应用；不去重内容，只去重 `event_id`。

### 大小与频率

```text
本地去抖：150ms
text  4 MiB
image 32 MiB
file  默认 2 GiB（可配置）
超限：不发送，UI 提示
压缩：双方能力协商后，对超过阈值的文本/文件启用 zstd
```

## 10. 文件/目录传输

### 不再使用 tar 作为传输格式

```text
选中文件/目录
  -> manifest（清单）
  -> 每文件一条流
  -> 接收端写入临时目录
  -> 校验通过后原子切换
  -> 设置剪贴板
```

tar 只适合打包下载，不适合可恢复、可校验、可取消、可并发的剪贴板传输。

### Manifest

```text
TransferManifest {
  transfer_id: Uuid,
  event_id: Uuid,
  entries: [
    Entry {
      index: u32,
      relative_path: String,
      kind: File | Dir,
      size: u64,
      sha256: [u8; 32],
      mode: Option<u32>,
    }
  ],
  total_size: u64,
  file_count: u32,
}
```

### 路径安全

每个 `relative_path` 必须通过：

1. 不是绝对路径。
2. 不含 `..` / `.` 组件。
3. 不含 Windows 盘符 / UNC / NUL / 控制字符。
4. 不含 Windows 保留名（CON、PRN、AUX、NUL、COM1-9、LPT1-9）。
5. 不以点或空格结尾（Windows）。
6. 路径长度、深度、组件数在限制内。
7. 不跟随符号链接；目录遍历用 `symlink_metadata`。
8. 拼接后 canonicalize，必须仍位于本次传输临时根目录内。

任何一条不满足：整次传输拒绝。

### 原子落盘

```text
incoming/<transfer_id>/
        |
        | 接收 + 校验所有文件
        v
files/<transfer_id>/
        |
        | 全部成功
        v
设置剪贴板为 files/<transfer_id>/ 下的顶层条目
```

- 失败/取消/断线：删除 `incoming/<transfer_id>`。
- 传输中禁止清理该目录；清理任务只清理非 active 的旧传输。
- 启动时清理超过 24 小时的 `incoming/*` 和不再被剪贴板引用的旧 `files/*`。

### 分块与并发

```text
每个文件一条单向 QUIC 流：

  FileStart { index, offset }
  Chunk { bytes }
  ...
  FileEnd

同时最多 4 条文件流；每 peer 有总并发上限。
```

- QUIC 保证有序可靠；chunk 不需要逐块 ACK。
- 每条流记录 offset，为续传预留。
- 取消：关闭流 + 删除临时目录。

### 断点续传

```text
重连后：
接收端 -> ResumeState { transfer_id, file_index, received_bytes }
发送端 -> 校验源文件 size / mtime / hash
        - 没变：从 received_bytes 继续
        - 变了：从头重传该文件
```

第一版可只实现 offset 记录；协议帧保留 offset，后续加真正续传不改格式。

### 校验

- 发送端读文件时算 SHA-256。
- 接收端写完后重算，与 manifest 比对。
- 不一致：该文件失败，整次传输失败/重试；不设置剪贴板。

### 压缩

- 双方 Hello 协商 zstd 能力。
- 单文件阈值（例如 > 1 MiB）才压缩；小文件不压缩。
- 文件独立压缩，不压缩整个 tar。
- benchmark 决定默认开关；允许用户关闭。

### 进度、取消、磁盘空间

- 进度事件 200ms 节流：`transfer_id`, `file_index`, `bytes_done`, `total`, `speed`。
- 取消：关闭流、删除 incoming、发 `TransferCancelled`。
- 接收前检查可用空间 >= `total_size * 1.1`，不足直接拒绝。

### 目录语义

- 每个选中项贡献自己的 basename 作为顶层条目。
- 接收端最终设置剪贴板为这些顶层路径。
- 两个选中项 basename 冲突：传输前拒绝并提示，不隐式改名。

## 11. 剪贴板

### 抽象

```text
trait ClipboardBackend {
    fn read(&self) -> Result<ClipboardContent>;
    fn write(&self, content: &ClipboardContent) -> Result<()>;
    fn watch(&self, tx: Sender<RawClipboardEvent>) -> Result<WatchHandle>;
}
```

`ClipboardActor` 单任务拥有 backend，命令通过 mpsc：

```text
Read / Write(remote) / Shutdown
```

所有 OS 剪贴板访问串行化，避免 `ClipboardContext` 跨线程和回声竞态。

### 内容模型

```text
enum ClipboardContent {
    Text { text: String },
    Image { format: ImageFormat, width: u32, height: u32, bytes: Bytes },
    Files { paths: Vec<PathBuf> },
}
```

- 读取优先级：Files > Image > Text。
- 只同步纯文本，不传 HTML/RTF（后续可扩展）。
- 图片统一转 PNG 再发送，接收端解码后写入；限制尺寸防解压炸弹。
- 空文本默认不同步；后续可加显式 Clear 事件。

### 去抖与限制

- 150ms 去抖；文本/图片快速连续复制只保留最新。
- text 4 MiB，image 32 MiB，file 2 GiB。
- 超限不发送，UI 明确提示。

### 平台适配

- 先保留 `clipboard-rs`，用 trait 隔离。
- Windows CF_HDROP、macOS pasteboard、Linux Wayland/X11 差异留在 adapter。
- 重压缩/图片解码放 `spawn_blocking`，不阻塞 watcher。

## 12. 持久化与生命周期

### 单一数据源

- Rust 是设置、名册、历史的唯一数据源；UI 通过命令/事件读取。
- `settings.json`：`schema_version` + typed `AppSettings`；原子写（temp + rename）+ 备份。
- `peers.json`：`uuid -> { public_key, name, first_seen, last_seen, blocked }`。
- 历史量变大再考虑 SQLite；首版 JSON + migration 足够。
- Dev/Release 路径保持 `.dev` 后缀；路径只在 Rust 一处定义。
- 设备私钥、房间密钥进 keychain；JSON 不存明文密钥。

### 迁移

- 旧 `.info.json` / `.settings.json` 读取：保留 `uuid`、链接设备列表、房间 key。
- 旧链接设备没有设备公钥：v2 首次连接时自动登记；旧列表作为“已知对端”提示。
- 旧默认 key `"cliprtc"`：提示更换；不静默修改。
- 迁移前备份；失败回滚并显示错误。
- 协议 v2 与 v1 不兼容：发现 v1 对端时提示升级，不尝试兼容。

### 生命周期

- `AppState { core: Arc<Core> }`，由 Tauri 管理；无全局业务状态。
- `Core::start(config) -> Result<CoreHandle>`；`start_server` 命令移除或返回“已运行”。
- 各服务有 `start` / `shutdown` 和 `CancellationToken`。
- 优雅关闭：停止接受连接、关闭 session、flush store、停止 watcher、停止 mDNS。
- `tauri-plugin-single-instance` 防止重复实例共享 uuid/端口。
- `tracing` + 文件轮转；日志包含 `peer`, `session`, `transfer_id`，不包含剪贴板内容/密钥。
- 启动错误弹窗/写日志，不 panic。

## 13. 前端与 IPC

### Typed IPC

- 使用 `tauri-specta` 或 `ts-rs` 从 Rust 生成 TS 类型/命令/事件绑定。
- 禁止手写重复 DTO；`src/utils/cmds.ts` 逐步替换为生成的 bindings。
- 统一 `AppError { code, message, details }`，UI 按 code 做 i18n。

### 命令面

```text
get_state
get_settings
update_settings
set_room_key
get_peers
block_peer
unblock_peer
get_transfers
cancel_transfer
pause_sync
resume_sync
open_logs
exit_app
restart_app
```

### 事件

```text
peer-found / peer-updated / peer-removed
clipboard-applied
transfer-progress / transfer-completed / transfer-failed / transfer-cancelled
sync-paused / sync-resumed
room-key-changed
app-error
```

### 状态与 UI

- 引入轻量状态库（Zustand）或 typed context；事件驱动更新 peers/transfers/settings。
- 设备列表：名称、短 uuid、在线状态、最后发现时间、公钥指纹、block/unblock。
- 新设备加入通知；信任表不一致时明确报错。
- 传输进度：文件、速度、取消；完成/失败 toast。
- 设置：房间密钥生成/复制/显示、强度提示、暂停同步、大小限制、压缩开关、开机启动、语言。
- 修复 `About` 无依赖数组的 `useEffect`；不要在 render 中发命令。
- 新增文案同时写入 `en-US.ts` / `zh-CN.ts`。
- Tray 状态：同步中/暂停/离线；托盘从 Rust setup 创建，避免依赖隐藏的 settings webview。

## 14. 错误处理与可观测性

- `CoreError` 使用 `thiserror`，分类：Config、Keychain、Discovery、Transport、Auth、Protocol、Transfer、Clipboard、Store、Io、Cancelled。
- 运行时禁止 `unwrap` / `expect`；测试和不可达不变量除外。
- 每个网络/文件操作有超时和取消。
- 日志：`tracing` span per peer/session/transfer；记录大小、哈希前缀、耗时、错误；不记录内容。
- 指标：事件发送/接收、字节数、错误、重连、队列丢弃；调试页或周期日志。
- 启动健康检查：discovery、endpoint、clipboard watcher 是否运行；UI 显示状态。
- panic hook：记录 backtrace；不静默崩溃。

## 15. 测试与 CI

### 单元测试

- 身份：同 key 派生同 CA；不同 key 失败；证书签发/验证；uuid 提取；key 轮换。
- 协议：编码/解码 roundtrip；畸形帧；大小上限；版本不匹配。
- 去重：重复 `event_id`、旧 `seq`、新 `boot_id`、乱序、Lamport LWW。
- 路径：`..`、绝对路径、盘符、UNC、保留名、符号链接、Unicode；用 `proptest`。
- 文件：manifest 冲突、哈希不匹配、原子清理。
- 名册：uuid->pubkey 不一致、blocklist、migration。

### 集成测试

- 两个 in-process core 走 localhost QUIC：同 key 交换 text/image/file。
- 错误 key、冒充已有 uuid、blocklist 必须失败。
- 断线重连、重复事件、乱序事件。
- mock `ClipboardBackend` 和 mock `Discovery`，不依赖真实 OS/mDNS。

### Fuzz / Benchmark

- `cargo-fuzz`：帧解码、路径解析、manifest 解析。
- `criterion`：codec、哈希、连接建立、文件吞吐。
- 大文件/大量小文件/高并发事件基准。

### CI

新增 PR workflow：

```text
pnpm install --frozen-lockfile
pnpm lint
pnpm build
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo deny check        # 可选
```

构建矩阵：Windows / macOS / Ubuntu。Tag 发布仍走现有 workflow。

### 手工 E2E

两台主机/虚拟机：

- text、PNG、单文件、目录、大文件。
- 断线重连、peer 重启、mDNS 刷新、网络切换。
- 错误 key、key 轮换、blocklist、冒充测试。
- 主测 Windows；macOS/Linux smoke。

## 16. 迁移与切换

### 阶段

0. 设计定稿、协议 v2 文档、CI、core crate 骨架、测试 harness。
1. 身份 + 证书 + 名册 + QUIC 双端握手（headless）。
2. 同步协议 + 文本/图片事件 + ACK/去重 + 双端集成测试。
3. 文件传输 + 路径安全 + 原子落盘 + 进度/取消。
4. 发现重写 + session manager + 重连/地址筛选。
5. 剪贴板 actor + 真实平台 adapter + 回声抑制。
6. 持久化/migration/keychain + 生命周期 + 单实例。
7. Tauri 适配 + typed IPC + 前端状态/UI。
8. 删除旧实现、文档、发布。

### 切换策略

- 新旧网络栈不同时运行（会重复 watcher/endpoint/mDNS）。
- 用分支或 feature flag 切换整个 core；开发期用独立 app-data 目录/端口。
- v2 与 v1 协议不兼容：双端都升级；发现 v1 显示提示。
- 保留旧版本 tag/branch 以便回滚；v2 store 迁移前备份。

## 17. 风险与取舍

| 风险 | 影响 | 缓解 |
|---|---|---|
| 共享密钥模型无法强撤销单设备 | 被撤销设备可换新 uuid 加入 | blocklist + 轮换 key；文档明确 |
| 每台设备都有 CA 私钥 | 房间成员可签发新证书 | 名册绑定 uuid->公钥；不冒充已有设备 |
| 弱房间密钥可离线爆破 | 攻击者加入房间 | 默认高熵 key；强度提示；后续 Argon2id/PAKE |
| mDNS 在多网卡/VPN 下不可靠 | 看不到/连不上 | 地址竞速、过滤、手动 IP fallback |
| 固定端口与防火墙 | 连接失败 | 默认端口 + 文档；冲突回退随机 |
| 剪贴板平台差异 | 图片/文件格式问题 | adapter 隔离；手工 E2E |
| 大文件/磁盘耗尽 | 传输失败/占满磁盘 | 空间检查、上限、临时目录清理 |
| 自定义协议握手 | 实现错误 | 协议文档、测试向量、fuzz、双端集成测试 |
| 迁移破坏现有用户 | 数据丢失/无法连接 | 备份、保留 uuid、提示升级、回滚 tag |

## 18. 分阶段路线图

| 阶段 | 交付物 | 验收 |
|---|---|---|
| 0 | core crate、CI、协议 v2 文档、测试骨架 | `cargo test` / `pnpm lint` / `pnpm build` 通过 |
| 1 | 身份、CA、证书、名册、QUIC 双端握手 | 同 key 成功；错 key/冒充失败 |
| 2 | 事件信封、Lamport、去重、ACK、文本/图片 | 双端文本/图片同步；重放/重复正确 |
| 3 | 文件 manifest、路径安全、校验、进度 | 单文件/目录/大文件；路径攻击失败 |
| 4 | 发现、session manager、重连、地址筛选 | 双主机发现/重连/移除事件 |
| 5 | 剪贴板 actor、平台 adapter | 真实复制同步；无回声循环 |
| 6 | 持久化/migration/keychain/生命周期 | 升级保留数据；优雅关闭 |
| 7 | Tauri 适配、typed IPC、UI | 全功能端到端 |
| 8 | 删除旧代码、文档、发布 | v2 发布；回滚路径可用 |

## 19. 相关文件

- 现有入口：`src-tauri/src/lib.rs`、`src-tauri/src/init.rs`。
- 现有网络：`src-tauri/src/quic/`、`src-tauri/src/mdns/`。
- 现有剪贴板：`src-tauri/src/clipboard/`。
- 现有全局状态：`src-tauri/src/utils/constant.rs`。
- 前端 IPC：`src/utils/cmds.ts`、`src/utils/store.ts`。
- 指令文件：`AGENTS.md`。
