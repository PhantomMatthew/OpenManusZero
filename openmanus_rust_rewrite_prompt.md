# Role
你是一名资深 Rust 系统架构师、工程化专家与测试负责工程师（SRE + QA 背景），精通异步 Rust（tokio）、浏览器自动化（chromiumoxide/playwright-rs）、LLM API 集成、测试驱动开发（TDD）、持续集成/持续交付（CI/CD）与安全检测。你必须产出可编译的 Rust 代码、完整测试套件和 CI 配置文件，且代码可以在 Linux CI 环境中通过所有测试。

# Goal
将 "OpenManus"（一个 Python 编写的通用 AI Agent 框架）**完全用 Rust 重写**，并在完成后提供充分的自动化测试以保证质量、稳定性与安全性。最终交付应包括：项目仓库结构、核心实现（Agent、Tool 系统、LLM 客户端、浏览器工具、沙箱执行模块）、测试套件（单元、集成、端到端、模糊、性能）、CI/CD（GitHub Actions 示例）、测试覆盖率报告与发布脚本。

# Non-functional requirements
- 最低 Rust 版本：1.72（或最新稳定版）。在 Cargo.toml 中声明 MSRV。
- 遵循 rustfmt/Clippy 严格规则（CI 中 clippy -D warnings）。
- 所有网络调用应可被 Mock，测试不应依赖外部互联网（除性能基准可选）。
- 提供稳定的 API 抽象以便替换 LLM 后端（OpenAI, Anthropic, 本地 LLM）。
- 可在无 GPU 的 CI runner 上运行（浏览器自动化使用 headless chromium）。
- 提供明确的安全约束：避免任意代码执行（除沙箱环节），审计第三方依赖。

# Required libraries (建议与具体版本约定)
- tokio = "1"
- serde = { version = "1", features = ["derive"] }
- reqwest = { version = "0.11", features = ["json", "stream"] } 或使用 async-http-client
- chromiumoxide = "0.x" 或 playwright-rs（若选择）
- async-trait = "0.1"
- anyhow = "1"
- thiserror = "1"
- clap = "4"
- axum = "0.7"（若提供 HTTP API）
- tracing + tracing-subscriber
- serde_json
- wiremock = "0.x" 或 httpmock (用于测试 Mock LLM)
- proptest = "1"（属性测试）
- cargo-fuzz（用于 fuzz）
- tokio-test / rstest（用于测试辅助）
- assert_cmd（若执行二进制测试）
- mockall 或自定义 trait mocks（用于单元测试）
- tarpaulin 或 grcov 用于覆盖率（视 CI 环境）

# Core architecture & interfaces
1. Agent Loop（ReAct 风格）
   - 实现一个可组合的 “思考（Reason）- 行动（Act）- 观察（Observe）” 循环。
   - 必须支持工具调用、LLM 推理、上下文记忆管理、失败重试、并发任务处理。

2. Tool Trait（所有工具必须实现）
   ```rust
   #[async_trait::async_trait]
   pub trait Tool: Send + Sync {
       fn name(&self) -> &'static str;
       fn description(&self) -> &'static str;
       async fn call(&self, input: &str, ctx: &mut Context) -> Result<String, ToolError>;
   }
   ```
   - Tool 的调用必须可被 Mock，用于单元/集成测试。
   - 提供基础实现示例：LLMTool（包装 LLM 客户端）、BrowserTool（基于 chromiumoxide）、FileTool（受限的文件读写，带 sandbox 检查）。

3. LLM Client 抽象
   ```rust
   #[async_trait::async_trait]
   pub trait LlmClient: Send + Sync {
       async fn stream_completion(&self, req: &LlmRequest) -> Result<BoxStream<'static, LlmChunk>, LlmError>;
       async fn completion(&self, req: &LlmRequest) -> Result<LlmResponse, LlmError>;
   }
   ```
   - 在实现中提供一个 HttpLlmClient（基于 reqwest）和一个 MockLlmClient（用于测试，返回可控的流/响应）。

4. Context / Memory
   - 提供 ShortTermMemory（RingBuffer 或 VecDeque）和 LongTermMemory（可选持久化到 local sqlite 或 文件）。
   - Memory 需要线程安全，可用 Arc<RwLock<...>> 或 DashMap。所有测试应验证并发访问的正确性。

5. Sandboxed Execution
   - 提供两种沙箱策略：
     - 本地进程隔离 + 限制（使用 Command + resource limits）
     - WebAssembly sandbox（wasmtime）——推荐用于运行 AI 生成的代码片段
   - 测试需覆盖沙箱的安全边界（例如：试图访问文件系统、网络均被正确拒绝或限制）。

# 分阶段任务（Phase）与可交付物（每一阶段都有测试要求）
Phase 0 — 项目初始化
- 交付：
  - Cargo.toml（含 MSRV、依赖、workspace 若需要）
  - README.md（包含目标、如何运行测试、如何本地启动）
  - .github/workflows/ci.yml（CI skeleton）
- 测试：
  - clippy & fmt 检查集成进 CI

Phase 1 — 基础抽象与单元测试
- 交付：
  - Tool trait、LlmClient trait、Context/Memory 的实现
  - mock 实现（MockLlmClient、MockTool）
- 测试：
  - 单元测试覆盖基本逻辑（工具注册/查找、context 并发读写、错误传播）
  - 覆盖率阈值：>= 80%（核心库逻辑）

Phase 2 — LLM 客户端实现与集成测试（Mock）
- 交付：
  - HttpLlmClient（支持 streaming），并用 wiremock/httpmock 在测试中模拟 LLM 服务
- 测试：
  - 集成测试：以 Mock LLM 验证流式输出的消费逻辑
  - 验证在不稳定网络（延迟/断开）情况下的重试策略

Phase 3 — Browser Tool 与 E2E（本地 Headless Chromium）
- 交付：
  - BrowserTool：访问页面、取文本、点击、截图等基础动作
  - E2E Harness：可启动 headless chromium 并运行场景脚本
- 测试：
  - 使用本地简易测试页面（静态 HTML）做端到端测试（Agent 发起动作、BrowserTool 返回期望结果）
  - CI 需能运行 headless 浏览器（使用 --headless 或 xvfb if necessary）

Phase 4 — Agent Orchestrator 与完整 E2E 测试（Mock LLM + Browser）
- 交付：
  - Orchestrator/Manager：调度 agent、维护上下文、解析 LLM 输出并调用对应工具
- 测试：
  - E2E 测试场景：给定 prompt -> Agent 规划 -> 调用 BrowserTool/LLMTool -> 返回最终结果
  - 在 CI 上以 Mock LLM + headless browser 运行至少 5 个复杂场景

Phase 5 — 安全、性能与模糊测试
- 交付：
  - cargo-fuzz 目标
  - 性能基准（基于 criterion 或自定义 bench）
  - cargo-audit 集成报告
- 测试：
  - fuzz 运行至少对重要解析器和输入口 fuzz 1000 个案例（CI 可做每日任务）
  - 性能回归基线并在 PR 中验证（例如：平均延迟、内存峰值）

Phase 6 — CI/CD 与发布
- 交付：
  - 完整 GitHub Actions 流水线：格式化 -> clippy -> 单元 -> 集成 -> E2E -> 覆盖率上报 -> 打包二进制 release
  - Release 脚本（包含 cross 编译或 Docker 镜像）
- 测试：
  - Pull Request 验证 pipeline 通过
  - 生成测试覆盖率 badge 与二进制 artifact

# 测试策略细节（必须清楚、可执行）
1. 单元测试（cargo test）
   - 每个模块都有单元测试（边界、错误路径、并发）
   - 使用 mockall 或 trait mock pattern 进行依赖注入

2. 集成测试（tests/ 目录）
   - 使用 httpmock/wiremock to mock LLM endpoints，控制流式输出
   - BrowserTool 在本地运行 headless chromium 对一个静态测试页面（放在 tests/assets）执行操作

3. 端到端测试
   - 启动一个小型测试 harness（可由 GitHub Actions 启动），包含：
     - Mock LLM 服务
     - Headless chromium 实例（或 Docker 镜像）
   - 用真实的 Orchestrator 运行 Agent 工作流并断言最终结果

4. Fuzzing（cargo-fuzz）
   - 针对 Input Parsers/LLM Response Parsers/Tool Call Parsers 添加 fuzz target
   - 在 CI/Cron 中每日/每周运行并把发现的回归提交 issue

5. 属性测试（proptest）
   - 对 key 的文本解析、JSON 解码、上下文合并逻辑使用 proptest 验证不变量

6. 性能测试（criterion 或自定义）
   - 基准包括：LLM 客户端吞吐、BrowserTool 并发页面数量、Agent 单次规划延迟
   - CI 可在“基线/实验”模式下触发，并在 PR 中显示性能回归警报（若超出阈值）

7. 覆盖率
   - 使用 tarpaulin 或 grcov 生成覆盖率报告并上传到 CI（例如 codecov）
   - 阈值：核心库 >= 80%，系统集成 >= 60%

8. 安全检测
   - cargo-audit 在 CI 中运行（失败条件：发现未修复的 vuln）
   - 第三方依赖尽量 pin 版本并评估风险（列出高风险依赖并提供替代方案）

# CI 示例（GitHub Actions skeleton）
- 步骤示例：
  1. Checkout
  2. Install Rust toolchain + cargo fmt + clippy
  3. cargo fmt -- --check
  4. cargo clippy --all-targets --all-features -- -D warnings
  5. cargo test --workspace --verbose
  6. Run integration/E2E test matrix (mock LLM + headless chromium)
  7. cargo tarpaulin (or grcov) -> upload coverage
  8. cargo-audit
  9. cargo fuzz (optional / scheduled job)
  10. Build release artifact

（你需要把完整的 .github/workflows/ci.yml 按上述步骤生成并包含服务依赖配置）

# 提交/交付物清单（要求在 PR 中包含）
- 完整源码
- README.md（含 run/test/ci/bench instructions）
- docs/ 包含架构图（ascii 或 mermaid）、接口定义、测试策略文档
- .github/workflows/ci.yml（可直接在 GitHub 上运行）
- tests/: 单元/集成/E2E 脚本与测试页面资源 (tests/assets)
- fuzz/: cargo-fuzz targets
- bench/: criterion benchmark
- scripts/: release & packaging 脚本
- CHANGELOG.md（语义化版本更新）

# 验收标准（最终交付必须满足）
1. 本仓库能在 Ubuntu-latest GitHub Actions Runner 上通过 CI（clippy/format/test/integration/coverage/audit）。
2. unit coverage >= 80%（核心库），总体覆盖率报告上传。
3. 至少 5 个端到端场景通过 Mock LLM + headless chromium，并作为集成测试在 CI 运行。
4. 提供 fuzz targets，且在本地运行无致命崩溃（或记录问题）。
5. 提供基准测试结果及 baseline（用来监控性能回归）。
6. 所有代码通过 rustfmt/clippy 且没有警告（CI 严格执行）。
7. 有明确文档说明如何在本地和 CI 中复现所有测试。

# 回答格式要求（你作为 AI 需要遵守）
- 首次回复：给出项目目录结构建议（tree）、Cargo.toml 模板、主要模块的 skeleton 代码（main.rs/agent.rs/tool.rs/llm.rs/context.rs），并为每个文件给出最少 1 个单元测试样例。
- 后续按 Phase 顺序输出每个 Phase 的完整实现（每次只给一个 Phase 的可运行代码 + 相应测试），等待用户确认后继续到下一 Phase。
- 对于每个实现文件，给出简短说明、设计理由、以及如何在本地运行单元/集成/E2E 测试的命令。
- 如遇到设计歧义，主动列出可选方案（不多于 3 个）并推荐一个默认方案，继续实现时遵循默认方案。

# 开始任务：请先输出
1. 建议的项目目录结构（tree 样式）
2. Cargo.toml 模板（包含依赖）
3. 核心模块 skeleton（main.rs、lib.rs、agent.rs、tool.rs、llm.rs、context.rs）
4. 对每个 skeleton 文件给出至少一个单元测试示例（可放在相应模块的 tests 或 #[cfg(test)] 中）
5. 简短说明如何运行测试和 CI 要求（命令行）

请从第 1 步开始输出（项目目录结构、Cargo.toml、skeleton 文件和单元测试示例），并严格遵循“先实现 Phase 0/1 的逐步交付模型”。 