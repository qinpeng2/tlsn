# Binance KYC Demo - 使用真实的混淆电路和OT

这个示例演示了如何使用 TLSNotary 连接到真实的网站（binance.com/setting/kyc），并使用**真实的混淆电路（Garbled Circuits）和不经意传输（Oblivious Transfer）**来证明 TLS 会话。

## 重要说明

⚠️ **本示例使用真实的 MPC 协议**，确保 `tlsn_insecure` 特性**未启用**。默认情况下，代码使用：
- `Garbler` 和 `Evaluator`（混淆电路）
- `DerandCOTSender/Receiver`（不经意传输）

只有在设置了 `tlsn_insecure` 特性时才会使用不安全的 `IdealVm`。

## 架构

```
Prover (Rust)  ←→  WebSocket  ←→  Verifier (Rust)
     ↓
  TLS 连接
     ↓
  binance.com
```

- **Prover**: 连接到真实的 binance.com，通过 WebSocket 与 Verifier 通信
- **Verifier**: 监听 WebSocket 连接，验证 Prover 的证明
- **WebSocket**: Prover 和 Verifier 之间的通信通道

## 运行步骤

### 1. 启动 Verifier

在一个终端中运行：

```bash
cd /opt/workspace/playground/tlsnotary/tlsn
cargo run --release --bin binance_verifier
```

或者指定监听地址：

```bash
VERIFIER_ADDR=127.0.0.1:8080 cargo run --release --bin binance_verifier
```

Verifier 将监听 `127.0.0.1:8080`（默认）并等待 Prover 连接。

### 2. 启动 Prover

在另一个终端中运行：

```bash
cd /opt/workspace/playground/tlsnotary/tlsn
cargo run --release --bin binance_prover
```

或者指定 Verifier URL 和目标 URL：

```bash
VERIFIER_URL=ws://127.0.0.1:8080 \
TARGET_URL=https://www.binance.com/setting/kyc \
cargo run --release --bin binance_prover
```

### 3. 观察输出

Prover 将：
1. 连接到 Verifier
2. 建立到 binance.com 的 TLS 连接
3. 发送 HTTP 请求
4. 接收响应
5. 创建证明并发送给 Verifier

Verifier 将：
1. 接受 Prover 的连接
2. 验证 TLS 承诺协议配置
3. 运行 MPC-TLS 协议（使用真实的混淆电路和OT）
4. 验证证明
5. 显示验证结果和 transcript 预览

## 环境变量

### Verifier

- `VERIFIER_ADDR`: Verifier 监听地址（默认: `127.0.0.1:8080`）

### Prover

- `VERIFIER_URL`: Verifier 的 WebSocket URL（默认: `ws://127.0.0.1:8080`）
- `TARGET_URL`: 要连接的目标 URL（默认: `https://www.binance.com/setting/kyc`）

## 验证使用真实的 MPC

要确认使用了真实的混淆电路和OT，可以：

1. **检查代码**: 查看 `tlsn/crates/tlsn/src/mpz.rs`，确认使用了 `Garbler`、`Evaluator` 和 `DerandCOTSender/Receiver`
2. **检查特性**: 确保编译时没有启用 `tlsn_insecure` 特性
3. **观察性能**: 真实的 MPC 会比理想化的实现慢，因为需要执行真实的密码学操作

## 故障排除

### 连接失败

- 确保 Verifier 在 Prover 之前启动
- 检查防火墙设置
- 确认 WebSocket URL 正确

### DNS 解析失败

- 检查网络连接
- 尝试使用不同的 DNS 服务器

### TLS 握手失败

- 确保目标网站支持 TLS 1.2 或更高版本
- 检查证书是否有效

## 技术细节

### MPC 协议

本示例使用：
- **混淆电路（Garbled Circuits）**: 用于 MPC 计算
- **不经意传输（Oblivious Transfer）**: 用于安全的数据传输
- **DEAP**: 双执行 + 非对称隐私

### 网络通信

- Prover ↔ Verifier: WebSocket（通过 `async-tungstenite`）
- Prover ↔ Server: 标准 TLS 连接

### 数据限制

- `MAX_SENT_DATA`: 64KB（可发送的最大数据）
- `MAX_RECV_DATA`: 1MB（可接收的最大数据）

这些限制在承诺阶段之前设置，用于预计算优化。

## 下一步

- 尝试连接到其他网站
- 修改数据披露策略（只披露部分内容）
- 使用零知识证明进行更复杂的验证
