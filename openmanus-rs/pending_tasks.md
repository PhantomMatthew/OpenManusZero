# OpenManus Rust Rewrite - Pending Tasks

## 功能对比 (Python vs Rust)

### ✅ 已完成功能

| 模块 | Python版本 | Rust版本 | 状态 |
|------|-----------|---------|------|
| **Agent系统** | | | |
| BaseAgent | ✅ | ✅ | 完成 |
| ReActAgent | ✅ | ✅ | 完成 |
| ToolCallAgent | ✅ | ✅ | 完成 |
| ManusAgent | ✅ | ✅ | 完成 |
| **LLM客户端** | | | |
| OpenAI兼容API | ✅ | ✅ | 完成 |
| 流式响应 | ✅ | ✅ | 完成 |
| 重试机制 | ✅ | ✅ | 完成 |
| **Tool系统** | | | |
| BashTool | ✅ | ✅ | 完成 |
| PythonExecute | ✅ | ✅ | 完成 |
| AskHuman | ✅ | ✅ | 完成 |
| Terminate | ✅ | ✅ | 完成 |
| FileOperators | ✅ | ✅ | 完成 |
| BrowserTool | ✅ | ✅ | 完成 |
| ToolCollection | ✅ | ✅ | 完成 |
| **Flow编排** | | | |
| BaseFlow | ✅ | ✅ | 完成 |
| PlanningFlow | ✅ | ✅ | 完成 |
| **Sandbox沙箱** | | | |
| LocalSandbox | ✅ | ✅ | 完成 |
| **Memory/Context** | | | |
| ShortTermMemory | ✅ | ✅ | 完成 |
| LongTermMemory | ✅ | ✅ | 完成 |
| Context | ✅ | ✅ | 完成 |

---

### ❌ 未完成功能

| 模块 | Python版本功能 | 描述 | 优先级 |
|------|---------------|------|--------|
| **Agent系统** | | | |
| BrowserAgent | `app/agent/browser.py` | 专用浏览器代理，自动网页交互 | 高 |
| SWEAgent | `app/agent/swe.py` | 软件工程代理，代码修复 | ✅ |
| MCPAgent | `app/agent/mcp.py` | MCP协议代理 | 中 |
| DataAnalysisAgent | `app/agent/data_analysis.py` | 数据分析代理 | 中 |
| SandboxAgent | `app/agent/sandbox_agent.py` | 沙箱执行代理 | 中 |
| **Tool系统** | | | |
| StrReplaceEditor | `app/tool/str_replace_editor.py` | 字符串替换编辑器 | ✅ |
| WebSearch | `app/tool/web_search.py` | 网页搜索工具 | 高 |
| Crawl4aiTool | `app/tool/crawl4ai.py` | 网页爬虫工具 | 中 |
| ComputerUseTool | `app/tool/computer_use_tool.py` | 电脑控制工具 | 中 |
| PlanningTool | `app/tool/planning.py` | 规划工具 | 中 |
| CreateChatCompletion | `app/tool/create_chat_completion.py` | 聊天补全创建 | 低 |
| ChartVisualization | `app/tool/chart_visualization/` | 图表可视化工具集 | 中 |
| MCPServerTool | `app/tool/mcp.py` | MCP服务器工具 | 中 |
| SearchTools | `app/tool/search/` | 搜索工具集（多个搜索引擎） | 中 |
| **Sandbox沙箱** | | | |
| DockerSandbox | `app/sandbox/core/sandbox.py` | Docker容器沙箱 | 高 |
| SandboxManager | `app/sandbox/core/manager.py` | 沙箱管理器 | 高 |
| WasmSandbox | - | WASM沙箱(部分实现) | 中 |
| **Flow编排** | | | |
| FlowFactory | `app/flow/flow_factory.py` | 流程工厂 | 低 |
| **协议支持** | | | |
| A2A Protocol | `protocol/a2a/` | Agent-to-Agent协议 | 中 |
| MCP Protocol | `app/mcp/` | Model Context Protocol | 中 |
| **LLM支持** | | | |
| AWS Bedrock | `app/bedrock.py` | AWS Bedrock集成 | 低 |
| Daytona | `app/daytona/` | Daytona集成 | 低 |
| **其他** | | | |
| Prompt模板系统 | `app/prompt/` | 完整的提示词模板 | 中 |
| 配置系统 | `app/config.py` | 完整配置管理 | 高 |
| 日志系统 | `app/logger.py` | 完整日志系统 | 中 |

---

## 完成度统计

| 类别 | Python功能数 | Rust已实现 | 完成率 |
|------|-------------|-----------|--------|
| Agent类型 | 6 | 4 | 67% |
| Tool工具 | 14+ | 6 | 43% |
| Sandbox类型 | 2 | 1 | 50% |
| Flow类型 | 2 | 2 | 100% |
| 协议支持 | 2 | 0 | 0% |
| **总体** | ~30 | ~15 | **~50%** |

---

## 建议优先完成的功能

### 高优先级 (核心功能)

- [ ] **DockerSandbox** - 安全执行环境
  - 文件: `src/sandbox/docker.rs`
  - 依赖: bollard crate (Docker API)
  - 功能: 容器化执行、资源限制、隔离

- [ ] **StrReplaceEditor** - 代码编辑
  - 文件: `src/tool/str_replace_editor.rs`
  - 功能: 字符串替换、文件编辑、预览

- [ ] **WebSearch** - 网页搜索
  - 文件: `src/tool/web_search.rs`
  - 功能: 多搜索引擎支持、结果解析

- [ ] **BrowserAgent** - 完整浏览器代理
  - 文件: `src/agent/browser.rs`
  - 功能: 自动网页导航、表单填写、截图

- [ ] **SWEAgent** - 代码修复能力
  - 文件: `src/agent/swe.rs`
  - 功能: 代码分析、bug修复、测试运行

### 中优先级 (增强功能)

- [ ] **MCP Protocol支持**
  - 文件: `src/mcp/`
  - 功能: MCP服务器和客户端

- [ ] **图表可视化**
  - 文件: `src/tool/chart/`
  - 功能: 图表生成、数据可视化

- [ ] **搜索工具集成**
  - 文件: `src/tool/search/`
  - 功能: Google、Bing、DuckDuckGo搜索

- [ ] **Prompt模板系统**
  - 文件: `src/prompt/`
  - 功能: 模板管理、变量替换

### 低优先级 (扩展功能)

- [ ] **A2A Protocol**
  - 文件: `src/protocol/a2a/`
  - 功能: Agent间通信

- [ ] **AWS Bedrock集成**
  - 文件: `src/llm/bedrock.rs`
  - 功能: Bedrock API调用

- [ ] **FlowFactory**
  - 文件: `src/flow/factory.rs`
  - 功能: 流程动态创建

---

## 当前测试覆盖

| 测试类型 | 数量 | 状态 |
|---------|------|------|
| 单元测试 | 143 | ✅ 通过 |
| E2E测试 | 18 | ✅ 通过 |
| 集成测试 | 6 | ✅ 通过 |
| 属性测试 | 13 | ✅ 通过 |
| 文档测试 | 1 | ✅ 通过 |
| **总计** | **181** | ✅ |

---

## 技术栈对比

| 组件 | Python版本 | Rust版本 |
|------|-----------|---------|
| 异步运行时 | asyncio | tokio |
| HTTP客户端 | httpx/aiohttp | reqwest |
| 浏览器自动化 | browser-use/playwright | chromiumoxide |
| 序列化 | pydantic | serde |
| 测试 | pytest | cargo test + proptest |
| 基准测试 | pytest-benchmark | criterion |
| 代码覆盖 | pytest-cov | tarpaulin |
| CI/CD | GitHub Actions | GitHub Actions |

---

*最后更新: 2026-02-20*
