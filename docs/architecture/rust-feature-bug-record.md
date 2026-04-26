# Rust Feature Bug Record

本文档记录 SERAPH 在真实 crate 验证中暴露出的典型问题，重点保留三类信息：

- 问题在什么真实 crate / 阶段出现
- 其背后是否与 Rust 特有语义有关
- SERAPH 为了适配该 Rust 特性做了什么设计修复，以及这如何提升发现 bug 的能力

当前重点样例为 `moonfire-ffmpeg`。

## 1. Phase 2 / RAG 工具缺陷

### 1.1 深层 setup 链缺失：`AVCodecParameters::dims` 上下文不完整

- **crate / 阶段**：`moonfire-ffmpeg`，Phase 2 检索
- **现象**：目标是 `api::moonfire_ffmpeg::avcodec::AVCodecParameters::dims` 时，旧上下文里没有 `codecpar -> InputCodecParameters -> AVCodecParameters` 这条关键 setup 链。
- **Rust 特性**：
  - `Deref<Target = ...>` 把 wrapper 类型暴露成目标类型
  - 生命周期参数和 FFI wrapper 让“真实可用的 owner”并不总是目标 owner 本身
- **根因**：
  - 旧实现主要依赖浅层 `unsafe_context_subgraph`
  - 旧图没有稳定表示 `InputCodecParameters --Deref--> AVCodecParameters`
  - setup 推导只认直接 `api_returns_type`，不会沿着 wrapper / deref 扩展
- **工具修复**：
  - 在图中加入 `type_deref_target` 边
  - 从 `trait_impl_registry.associated_type_bindings` 解析 `Deref::Target`
  - 检索时把 `Required Setup APIs` 和普通 `Related APIs` 分开
  - setup 推导沿着 producer chain + deref chain 回溯
- **能力提升**：
  - 深层 target 不再只看到“同层邻居”，而是看到真正可构造的 setup 链
  - `AVCodecParameters::dims` 现在能稳定拉出：
    - `InputFormatContext::streams`
    - `Streams::get`
    - `InputStream::codecpar`
- **真实验证**：
  - 新上下文：`/tmp/seraph-moonfire-ffmpeg-phase3-dims-rerun-hashing/contexts/rag_target_001.md`

### 1.2 外部 trait 只出现在 impl 表中，图里没有 trait 节点

- **crate / 阶段**：`moonfire-ffmpeg`，Phase 2 构图
- **现象**：真实 `knowledge.json` 中存在
  - `trait_impl::core::ops::deref::Deref::for::InputCodecParameters<'s>`
  - 但 `trait_registry` 中没有对应 `Deref` 节点
- **Rust 特性**：
  - crate 会为外部标准库 trait（例如 `core::ops::Deref`）实现 impl
  - 提取结果里可能只有 impl 元数据，没有本 crate trait 定义
- **根因**：
  - 旧图构建逻辑假设 `trait_registry` 一定已有 trait 节点
- **工具修复**：
  - 当 `trait_impl_registry` 中出现未知 trait 时，自动合成 trait 节点
  - 再补 `impl_connects_type_trait` 与 `type_deref_target`
- **能力提升**：
  - 不再丢失“wrapper 类型如何投影到真实 target 类型”的结构化信息
  - 对 FFI wrapper / smart wrapper 类 API 的检索更稳定

### 1.3 不可构造 target 被自动排入默认 round 队列

- **crate / 阶段**：`moonfire-ffmpeg`，Phase 2 target 选择
- **现象**：`EncodeContext::open` / `EncodeContext::set_params` 会出现在高优先级 unsafe target 中，但它们缺少公开可用的 owner 构造路径。
- **Rust 特性**：
  - 生命周期包装类型和 borrowed FFI wrapper 可以公开方法，但不公开构造器
  - 典型例子是内部 `AVCodecContext` 的借用包装
- **库侧事实**：
  - `Encoder::alloc_context` 在源码里被注释掉了
  - `EncodeContext<'a>` 只是 `&'a mut AVCodecContext` 的包装
  - 见：
    - `/tmp/seraph-real-crates/moonfire-ffmpeg-0.0.2/src/avcodec.rs:316`
    - `/tmp/seraph-real-crates/moonfire-ffmpeg-0.0.2/src/avcodec.rs:328`
    - `/tmp/seraph-real-crates/moonfire-ffmpeg-0.0.2/src/avcodec.rs:351`
- **根因**：
  - 旧 target 选择只看 unsafe / 排名分数，不分析 owner seed 是否可公开构造
- **工具修复**：
  - 新增 seed-type constructibility 分析
  - 自动 round 选择时默认跳过缺少公开 producer 的 target
  - 如果整个集合都不可构造，则保留向后兼容回退
- **能力提升**：
  - 避免把 LLM 预算浪费在无法从公开 API 面到达的 target 上
  - 减少为了“硬凑 owner”而诱发的 hallucination
- **真实验证**：
  - 自动 round 1 现在落到 `DecodeContext::decode_video`
  - 见：`/tmp/seraph-moonfire-ffmpeg-phase3-round1-auto/contexts/rag_target_001.md`

### 1.4 统一 CLI 真实执行时缺少 `PYTHONPATH`

- **crate / 阶段**：真实 `seraph-cli run` 执行
- **现象**：真实运行统一 CLI 时，Rust 侧调用 `python3 -m seraph_rag.cli` 会直接报：
  - `ModuleNotFoundError: No module named 'seraph_rag'`
- **根因**：
  - 集成测试中一直手动设置了 `PYTHONPATH=<repo>/rag`
  - 真实 `run` 执行路径没有自动注入这个环境
- **工具修复**：
  - 在 `crates/seraph-cli/src/main.rs` 的执行层，识别 `python -m seraph_rag.cli` 子命令并自动把 `<repo>/rag` 注入 `PYTHONPATH`
  - 增加回归测试，要求在没有外部 `PYTHONPATH` 时统一 CLI 也能跑通
- **能力提升**：
  - 真正实现“单一统一入口”而不是“文档上统一、运行时还要靠人工环境”

### 1.5 target 只当 target、不进别人的 `Related APIs`，会让邻域覆盖率虚低

- **crate / 阶段**：`moonfire-ffmpeg`，Phase 2 检索 / related API 统计
- **现象**：
  - 旧实现里，`Ffmpeg::new` 和若干 `Error::*` constructor 自己会被当作 target 跑到
  - 但它们几乎不会出现在别的 target 的 `Related APIs` 中
  - 导致早期真实批量统计里，`related API ratio` 只有 `62 / 70 = 88.57%`
- **根因**：
  - 旧 `Related APIs` 只来自 target 自身的局部 `unsafe_context_subgraph`
  - 对“孤立 utility / leaf target”没有额外 companion lane
- **工具修复**：
  - 在检索阶段加入 target companion 机制：
    - 保留原本的局部 related ranking
    - 再从全局 ranked unsafe target 列表中注入少量 companion target API
    - 合并时去重，并限制 companion slot，避免完全挤掉本地邻域
- **能力提升**：
  - target 不再只是“单点测试对象”，也会成为别的 target harness 的上下文 API
  - 更容易形成多 unsafe API 的组合序列
  - 基于同一份 `knowledge.json + graph.pkl` 重新渲染 `55` 个 target 上下文后，`Related APIs` 的并集已经覆盖全部 `70` 个 public API
- **真实验证**：
  - `moonfire-ffmpeg` rerender 后：`related union = 70 / 70 = 100%`

## 2. Phase 3 中暴露的 Rust 专有编译问题

### 2.1 模块路径 / re-export 幻觉：`Dictionary` 不在 crate root

- **crate / 阶段**：`moonfire-ffmpeg`，Phase 3 `AVCodecParameters::dims`
- **现象**：
  - 初始 harness 写成 `use moonfire_ffmpeg::Dictionary;`
  - `rustc` 报 `E0432`，提示应改为 `moonfire_ffmpeg::avutil::Dictionary`
- **Rust 特性**：
  - 类型是否从 crate root 重导出完全取决于模块公开策略
  - “名字像常用类型”并不意味着可从 root import
- **根因**：
  - LLM 依据直觉猜测 import 路径
  - 这类错误很常见于模块树较深、又没有 root re-export 的 Rust crate
- **工具设计 / 修复方式**：
  - 不试图在 prompt 中穷举所有 import 路径
  - 让 compile-check 捕获 `rustc` 诊断
  - fix-loop 把编译错误连同原始 harness 一起交给 LLM
  - 本轮中 fix-loop 在第 1 次修复就把 import 改成 `moonfire_ffmpeg::avutil::Dictionary`
- **能力提升**：
  - 即使初始生成出现模块路径幻觉，只要目标 API 设计正确，SERAPH 仍可通过 compiler-guided fix 收敛到可编译 harness
- **证据**：
  - 初始失败：`/tmp/seraph-moonfire-ffmpeg-phase3-dims-rerun-hashing/reports/compile_001_01.json`
  - 修复响应：`/tmp/seraph-moonfire-ffmpeg-phase3-dims-rerun-hashing/fixes/fix_response_001_01_01.md`

### 2.2 trait 必须实现完整接口，enum match 必须穷尽

- **crate / 阶段**：`moonfire-ffmpeg`，Phase 3 自动 round 1（`DecodeContext::decode_video`）
- **现象**：
  - LLM 为 `with_io_context` 生成了自定义 `FuzzIo: IoContext`
  - 初始 harness 编译失败：
    - `E0046`: `IoContext` 缺少 `buf_len`
    - `E0004`: `match Whence` 漏掉 `Whence::Size`
- **Rust 特性**：
  - trait impl 必须完整实现 trait 要求的方法
  - Rust enum pattern match 默认必须穷尽
  - 这类错误会在编译期被严格拦截
- **根因**：
  - LLM 生成了“看起来像对的” trait impl，但没有完全覆盖 trait surface
  - 同时忽略了 `Whence` 的一个枚举变体
- **工具设计 / 修复方式**：
  - compile-check 保留完整 `rustc` 诊断
  - fix-loop 把错误原样反馈给 LLM
  - 修复后 harness 增加：
    - `fn buf_len(&self) -> usize`
    - `Whence::Size => ...`
- **能力提升**：
  - 对需要手写 trait adapter / callback 的 Rust harness，SERAPH 不要求首轮就完美
  - 依靠编译器语义检查 + fix-loop，可以让复杂 trait/enum 细节自动收敛
- **证据**：
  - 初始失败：`/tmp/seraph-moonfire-ffmpeg-phase3-round1-auto/reports/compile_001_01.json`
  - 修复后 harness：`/tmp/seraph-moonfire-ffmpeg-phase3-round1-auto/fuzz/harness_001_01_fixed_01.rs`

### 2.3 编译关键事实如果只散落在 setup / related 文本里，首轮很容易“看懂语义但写错 Rust”

- **crate / 阶段**：`moonfire-ffmpeg`，Phase 3 首轮 harness 生成
- **现象**：
  - 在旧上下文中，`Dictionary`、`IoContext`、`Whence` 等事实虽然已经存在于 `knowledge.json` 与普通 RAG 文本中，
    但没有被单独提升成“编译时硬约束”
  - 结果是 LLM 容易出现：
    - import 走错模块路径
    - trait impl 缺方法或写错签名
    - enum match 漏分支
- **Rust 特性**：
  - Rust 的模块路径、trait surface、enum exhaustiveness 都是精确编译约束，不是“语义接近就行”
  - 尤其在 FFI wrapper / callback 型 API 中，编译成功依赖非常具体的模块与签名细节
- **工具修复**：
  - 在 Phase 2 context 中新增显式 section：
    - `Exact Import Paths`
    - `Required Traits`
    - `Trait Method Signatures`
    - `Enum Variants`
  - 在 Phase 3 prompt 中把这些 section 升级成 authoritative compile-time facts
  - 同时修复 context budget 裁剪逻辑，避免小预算时把这些关键 section 整块裁掉
- **能力提升**：
  - 减少“setup 语义是对的，但因为 Rust 细节写错而首轮编译失败”的情况
  - 让 fix-loop 更多处理真正复杂的 borrow / runtime 问题，而不是重复修正低级 import / enum / trait 漏项

### 2.4 trait 声明签名里的 `Self` receiver 不能直接照搬到 impl

- **crate / 阶段**：`moonfire-ffmpeg`，Phase 3 批量 round 1 / round 3
- **现象**：
  - 在引入 `Trait Method Signatures` 后，LLM 会把上下文里的
    - `fn buf_len(&Self) -> usize`
    - `fn read(&mut Self, ...) -> ...`
    直接原样抄进 `impl IoContext for ...`
  - `rustc` 随即报：
    - 语法错误
    - `E0186`（trait 要求 `&self` / `&mut self`，但 impl 中不是合法 receiver）
- **Rust 特性**：
  - trait item 的声明表面可以写成 `&Self` / `&mut Self`
  - 但在 impl 方法定义里，receiver 必须写成 `&self` / `&mut self`
  - 这属于 Rust trait surface 与 impl syntax 的细粒度语法差异
- **根因**：
  - 工具此前把 `knowledge.json` 里的 trait method `signature_text` 直接当作 authoritative compile-time facts 暴露给 LLM
  - 但这些签名是“声明态”，不是“实现态”
- **工具修复**：
  - 在 `Trait Method Signatures` section 生成时，将 receiver 规范化为 impl-ready 形式：
    - `Self -> self`
    - `&Self -> &self`
    - `&mut Self -> &mut self`
  - prompt 同步改成“copy the exact implementation-ready signature”
- **能力提升**：
  - 避免 LLM 因为 trait receiver 语法差异在首轮就踩编译错误
  - 真实 `round 1` 与 `round 3` 都从首轮失败变成首轮编译通过
- **证据**：
  - 初始失败：
    - `/tmp/seraph-moonfire-batch-rounds/round_1/reports/compile_001_01.json`
    - `/tmp/seraph-moonfire-batch-rounds/round_3/reports/compile_003_01.json`
  - 修复后 rerun：
    - `/tmp/seraph-moonfire-batch-rounds-rerun/round_1/reports/compile_001_index.json`
    - `/tmp/seraph-moonfire-batch-rounds-rerun/round_3/reports/compile_003_index.json`

### 2.5 深 setup 链被过早截断后，LLM 会退化成生命周期占位写法并触发 HRTB 编译错误

- **crate / 阶段**：`moonfire-ffmpeg`，Phase 3 批量 round 4（`AVCodecContext::params`）
- **现象**：
  - target `AVCodecContext::params` 的真实 setup 链需要经过：
    - `InputFormatContext::open/with_io_context`
    - `InputFormatContext::streams`
    - `Streams::get`
    - `InputStream::codecpar`
    - `InputCodecParameters::new_decoder`
    - `DecodeContext::ctx`
  - 旧 `Required Setup APIs` 因为 depth cap 太浅，只保留到中间层，丢掉了根构造器
  - LLM 随后生成了这种“占位式引用”：
    - `let _streams_get: fn(&Streams, usize) -> InputStream<'_> = Streams::get;`
    - `let _codecpar: fn(&InputStream<'_>) -> InputCodecParameters<'_> = ...;`
  - `rustc` 报：
    - `E0106`（missing lifetime specifier）
    - `E0308`（fn item / fn pointer 与更一般的 lifetime 约束不匹配）
- **Rust 特性**：
  - 带生命周期的 method item 一旦被强行写成显式 fn pointer 类型，常常需要 higher-ranked lifetime（HRTB）
  - 这不是普通“类型写错”，而是 Rust 生命周期与函数项多态的真实语义约束
- **根因**：
  - Phase 2 setup 回溯默认 `max_depth = 3`，对这个 target 来说过浅，导致真正的根 producer 没进 `Required Setup APIs`
  - prompt 也没有明确禁止“typed fn pointer / size_of / dead helper”这种假 reachability 写法
- **工具修复**：
  - 将 setup 回溯深度放宽到 `5`，把深层但可构造的 root API 也拉进 `Required Setup APIs`
  - prompt 与 markdown generation rules 新增约束：
    - 禁止 typed function-pointer bindings
    - 禁止 `std::mem::size_of` / `PhantomData` / dead helper 仅用于“提及” API
    - 如果 setup 不足，就 early return；不要伪造 reachability
- **能力提升**：
  - 对深层、带生命周期 wrapper 的 Rust target，LLM 更容易走真实构造链，而不是退化成编译上更难的 HRTB 占位技巧
  - `AVCodecContext::params` 在修复后已首轮 compile + smoke 通过
- **证据**：
  - 初始失败：`/tmp/seraph-moonfire-batch-rounds-post-receiver-fix/round_4/reports/compile_004_01.json`
  - 修复后 context：`/tmp/seraph-moonfire-round4-post-setup-fix/contexts/rag_target_004.md`
  - 修复后 compile：`/tmp/seraph-moonfire-round4-post-setup-fix/reports/compile_004_index.json`
  - 修复后 smoke：`/tmp/seraph-moonfire-round4-post-setup-fix/reports/smoke_004_index.json`

### 2.6 借用型 FFI owner 会把 `&mut` 借用一直保活，回读 backing IO 立刻触发 `E0502`

- **crate / 阶段**：`moonfire-ffmpeg`，2026-04-25 全量真实 batch，`round 22`（`Streams::get`）与 `round 43`（`Packet::pts`）
- **现象**：
  - LLM 先通过 `InputFormatContext::with_io_context(..., &mut io, ...)` 构造 owner
  - 随后又在 owner 仍存活时读取 `io.buf_len()` / `io.data`
  - `rustc` 报：
    - `E0502`: `cannot borrow ... as immutable because it is also borrowed as mutable`
- **Rust 特性**：
  - `InputFormatContext` 内部持有对 `IoContext` 的借用
  - Rust 借用检查会把这条 `&mut` 借用一直保留到 owner drop 为止
  - 因此 backing object 不能在同一作用域里再被共享借用读取
- **根因**：
  - LLM 虽然理解了 setup 链，但没有完全意识到“FFI owner 包装器仍在语义上保有 `&mut` 借用”
  - 这不是普通语法错误，而是 Rust aliasing / exclusivity 规则在 wrapper API 上的真实体现
- **工具修复方式**：
  - 不靠 prompt 微调硬压首轮完美
  - 直接把 `rustc` 的 `E0502` 诊断交给 fix-loop
  - fix-loop 在第 `1` 次修复中删掉或前移这些对 `io` 的读取，恢复编译
- **能力提升**：
  - SERAPH 可以自动收敛这类“语义 setup 正确，但违反 Rust 借用规则”的 harness
  - 说明 compiler-guided repair 对 Rust FFI wrapper 尤其关键
- **证据**：
  - `round 22` 初始失败：`/tmp/seraph-moonfire-ffmpeg-alltargets-full-20260425/reports/compile_022_01.json`
  - `round 22` 修复后 smoke：`/tmp/seraph-moonfire-ffmpeg-alltargets-full-20260425/reports/smoke_022_01_fixed_01.json`
  - `round 43` 初始失败：`/tmp/seraph-moonfire-ffmpeg-alltargets-full-20260425/reports/compile_043_01.json`
  - `round 43` 修复后 smoke：`/tmp/seraph-moonfire-ffmpeg-alltargets-full-20260425/reports/smoke_043_01_fixed_01.json`

## 3. Phase 3 中暴露的运行时 / 幻觉问题

### 3.1 target 后的额外“练手调用”会触发与目标无关的 panic

- **crate / 阶段**：`moonfire-ffmpeg`，早期真实批量评估
- **现象**：
  - 某些 harness 在 target 已经到达后，继续调用 `VideoFrame::plane(0)` 等 API
  - 这会触发 frame 内部状态相关 panic，污染 smoke 结果
- **Rust 特性**：
  - safe-looking wrapper API 仍可能编码隐藏不变量
  - FFI wrapper 的内部状态不满足条件时，safe 方法也可能 panic
- **根因**：
  - 旧 prompt 鼓励“多调用一些相关 API”
  - 但并没有约束“target 成功后应立即停止”
- **工具修复**：
  - prompt 明确要求：
    - target 成功后除非需要清理，否则立刻停止
    - 不要增加 post-target exercise 调用
  - smoke-run 与 `runtime_error` 机制保留问题证据，但不把这类运行时问题误当成 target 未覆盖
- **能力提升**：
  - 降低无关 panic 对覆盖判断的干扰
  - 保留运行时知识，为后续论文中的“Rust/FFI wrapper 隐藏不变量”分析提供证据
- **早期样例**：
  - `/tmp/seraph-moonfire-ffmpeg-alltargets-eval/fuzz/harness_009_01.rs`
  - `/tmp/seraph-moonfire-ffmpeg-alltargets-eval/fuzz/harness_034_01.rs`

### 3.2 伪造 enum / constructor / transmute 造成无意义 harness

- **crate / 阶段**：`moonfire-ffmpeg`，早期真实批量评估
- **现象**：
  - 对 `MediaType::*` 等 target，旧生成可能编造构造器、做任意 `transmute`
  - 这类 harness 编译或运行都不可靠
- **Rust 特性**：
  - enum 的公开构造方式、layout 与合法值空间并不应该靠猜
  - `transmute` 虽能制造值，但常常违背 API 想表达的语义前提
- **工具修复**：
  - prompt / fixer 规则显式禁止：
    - fabricated enum constructors
    - arbitrary transmute
    - unsafe initialization tricks
- **能力提升**：
  - 减少“看起来运行了，其实是幻觉造值”的无效 harness
  - 让编译/运行预算集中在可解释、可复现的真实 API 路径上

### 3.3 不可构造 target 会诱发两阶段假 reachability：diverging helper 与 `MaybeUninit` 伪状态构造

- **crate / 阶段**：`moonfire-ffmpeg`，2026-04-25 全量真实 batch，`round 55`（`EncodeContext::set_params`）
- **现象**：
  - **第一阶段（旧 batch）**：
    - harness 编译通过、smoke 也返回 `0`
    - 但源码实际通过 `fn helper<T>() -> T` 一类 placeholder 返回任意引用
    - helper 内部最终执行 `std::process::exit(0)`
    - smoke 输出里没有出现 `SERAPH_STEP_ENTER` / `SERAPH_STEP_OK`
    - 说明 target 调用在运行时根本不可达
  - **第二阶段（修掉 diverging helper 之后的 rerun）**：
    - LLM 不再使用 `process::exit` helper
    - 但改成：
      - `MaybeUninit::<EncodeContext>::zeroed()`
      - `Box::into_raw(...)`
      - 原始指针 cast
    - 表面上 target 调用真的“发生了”，但 owner 状态完全是伪造出来的，不是来自任何真实 public setup chain
- **Rust 特性**：
  - diverging expression / `!` 可以协变成任意返回类型
  - 这使得“永不返回的 helper”在类型检查层面可以伪装成任意带生命周期的引用或 wrapper
  - `MaybeUninit` / 原始指针 / `Box::into_raw` 也能绕过公开构造路径，伪造 FFI wrapper 所需状态
  - 对 borrowed FFI wrapper 来说，这是一种很 Rust-specific 的“假可构造性”
- **根因**：
  - `EncodeContext::set_params` 与 `EncodeContext::open` 一样，本质上缺少公开可用的 owner 构造路径
  - 旧流水线虽然已经在 prompt 里禁止 dead helper / placeholder，但 compile-check 之前没有语义级 validator 去拦：
    - diverging helper 假 reachability
    - `MaybeUninit` / raw-pointer fabrication 假状态构造
- **工具修复**：
  - 在 `rag/seraph_rag/compile_check.py` 加入 semantic guard：
    - 如果 harness 定义了 `fn helper<T>() -> T` 一类 generic placeholder
    - 且其函数体里使用 `std::process::exit` / `panic!` / `unreachable!` / `todo!` / `unimplemented!`
    - 则在真正调用 `cargo` / `rustc` 前直接判定为 `failed`
  - semantic guard 还会拒绝这些典型伪状态构造标记：
    - `MaybeUninit`
    - `mem::zeroed`
    - `transmute`
    - `assume_init`
    - `Box::into_raw`
  - 同时把这条约束同步到：
    - `rag/seraph_rag/harness_prompt.py`
    - `rag/seraph_rag/compile_fixer_bundle.py`
  - 这样 fix-loop 仍然可以继续接管，而不会把这种“表面 compile 成功”的假 harness 记成可用产物
- **能力提升 / 设计启示**：
  - 这说明 Rust 上“能通过类型检查”不等于“真的构造出了可运行的目标状态”
  - 通过把 diverging placeholder 提前降级成 compile-fixable failure，SERAPH 可以更早把预算拉回真实 public setup chain
  - 评估时仍然可以继续区分：
    - `generated coverage`
    - `compile/smoke surface pass`
    - `marker-confirmed target-hit coverage`
- **证据**：
  - 第一阶段假覆盖 harness：`/tmp/seraph-moonfire-ffmpeg-alltargets-full-20260425/fuzz/harness_055_01.rs`
  - 第一阶段 smoke：`/tmp/seraph-moonfire-ffmpeg-alltargets-full-20260425/reports/smoke_055_01.json`
  - 第二阶段伪状态构造 harness：`/tmp/seraph-moonfire-ffmpeg-alltargets-full-20260425-rerun2/fuzz/harness_055_01.rs`
  - 第二阶段 semantic guard 复验：`/tmp/seraph-moonfire-ffmpeg-alltargets-full-20260425-rerun2/reports/recheck_055.json`

### 3.4 仅打印 marker 而不真正调用 target，会制造“命中 target”的假象

- **crate / 阶段**：`moonfire-ffmpeg`，2026-04-25 rerun，`round 54`（`EncodeContext::open`）
- **现象**：
  - fix-loop 第 `2` 次尝试生成的 harness 已经不再伪造 constructor
  - 但它只是：
    - 打印 `SERAPH_STEP_ENTER`
    - 打印 `SERAPH_STEP_OK`
    - 中间没有真正调用 `EncodeContext::open`
  - 因而 smoke 表面返回 `0`，但 target 实际并未被执行
- **Rust / 流水线特性**：
  - 这不是单纯的 Rust 类型系统漏洞
  - 而是“marker 作为 coverage 证据”时容易出现的流水线语义漏洞：只要没有验证 marker 之间真的有 target call，LLM 就可能学会“打印 marker 充数”
- **根因**：
  - `EncodeContext::open` 同样缺少公开可用的 owner 构造路径
  - 在被 prompt 多次限制后，fix-loop 退化成“只保留 marker，不保留 target 调用”的假覆盖策略
  - 旧 compile-check 只看能否编译，不会检查 `SERAPH_STEP_ENTER` 与 `SERAPH_STEP_OK` 之间是否真的存在 target 调用
- **工具修复**：
  - 在 `rag/seraph_rag/compile_check.py` 增加 marker 区间语义检查：
    - 找到同一 target 的 `SERAPH_STEP_ENTER` 与 `SERAPH_STEP_OK`
    - 扫描两者之间的源码片段
    - 如果没有发现对应 target method 的真实 call-shaped use，则直接拒绝
- **能力提升**：
  - coverage marker 不再只是“打印过就算”
  - 对不可构造 target，SERAPH 现在会更诚实地把它们保留为未覆盖 / 不可达，而不是让 fix-loop 生成假 hit harness
- **证据**：
  - 假 marker harness：`/tmp/seraph-moonfire-ffmpeg-alltargets-full-20260425-rerun2/fuzz/harness_054_01_fixed_02.rs`
  - semantic guard 复验：`/tmp/seraph-moonfire-ffmpeg-alltargets-full-20260425-rerun2/reports/recheck_054.json`

## 4. 2026-04-25 `moonfire-ffmpeg` 真实 rerun 结果

### 4.1 深层 target：`AVCodecParameters::dims`

- **workspace**：`/tmp/seraph-moonfire-ffmpeg-phase3-dims-rerun-hashing`
- **target**：`api::moonfire_ffmpeg::avcodec::AVCodecParameters::dims`
- **Phase 2 结果**：
  - `Required Setup APIs` 已正确包含 `InputFormatContext::streams -> Streams::get -> InputStream::codecpar`
- **Phase 3 结果**：
  - 生成 `2` 个 harness variant
  - 初始 compile：`2/2` 失败
  - fix-loop：`2/2` 都在第 `1` 次修复后编译成功
  - smoke：`2/2` 成功
  - runtime diagnosis：无运行时错误
  - coverage：`validated`

### 4.2 自动 round 1：`DecodeContext::decode_video`

- **workspace**：`/tmp/seraph-moonfire-ffmpeg-phase3-round1-auto`
- **target**：`api::moonfire_ffmpeg::avcodec::DecodeContext::decode_video`
- **验证点**：
  - 默认自动选择没有落到 `EncodeContext::open`
  - 说明“不可构造 target 默认跳过”已经在真实执行中生效
- **Phase 3 结果**：
  - 生成 `1` 个 harness variant
  - 初始 compile：失败
  - fix-loop：第 `1` 次修复后编译成功
  - smoke：成功
  - runtime diagnosis：无运行时错误
  - coverage：`validated`

### 4.3 2026-04-25 首轮通过率改进验证

- **deep target workspace**：`/tmp/seraph-moonfire-ffmpeg-phase3-dims-postfix`
- **deep target workspace（v2）**：`/tmp/seraph-moonfire-ffmpeg-phase3-dims-postfix-v2`
- **auto round 1 workspace**：`/tmp/seraph-moonfire-ffmpeg-phase3-round1-postfix`
- **新增验证点**：
  - context 是否显式包含 `Exact Import Paths` / `Required Traits` / `Enum Variants`
  - 首轮 compile 是否比前一轮更少依赖 fix-loop
- **结果**：
  - `AVCodecParameters::dims`
    - 旧结果：首轮 `0/2` 编译成功
    - 中间结果：加入 `Exact Import Paths` / `Required Traits` / `Enum Variants` 后，首轮 `1/2` 编译成功
    - 最终结果：再加入 `Trait Method Signatures` 后，首轮 `2/2` 编译成功
    - 已不再出现 `Dictionary` root import 幻觉
    - 之前剩余的 trait signature 失败也被压下去
    - smoke：`2/2` 成功
  - `DecodeContext::decode_video`
    - 旧结果：首轮 `0/1` 编译成功
    - 新结果：首轮 `1/1` 编译成功
    - `IoContext` impl 已正确使用 `moonfire_ffmpeg::Error`、覆盖 `Whence::Size`、补全 `buf_len`
    - smoke：`1/1` 成功
- **结论**：
  - 新的 compile-critical context 确实提升了首轮通过率
  - 即使对需要手写 callback/adapter 的复杂 harness，首轮通过率也可以通过暴露更精确的 Rust 结构化事实显著提升
  - fix-loop 仍然有价值，但它现在更偏向兜底和处理更深的 Rust 语义细节，而不是基础 import/trait/enum 漏项

### 4.4 2026-04-25 `moonfire-ffmpeg` 批量 round 1–5 再验证

- **receiver 语法修复后批量 rerun**：`/tmp/seraph-moonfire-batch-rounds-rerun`
  - `round 1`：compile `ok`，smoke `ok`
  - `round 3`：compile `ok`，smoke `ok`
- **setup depth + placeholder rule 修复后 round 4 rerun**：`/tmp/seraph-moonfire-round4-post-setup-fix`
  - `round 4`：compile `ok`，smoke `ok`
- **最终批量 compile 复验**：`/tmp/seraph-moonfire-batch-rounds-final`
  - `round 1`：`ok`
  - `round 2`：`ok`
  - `round 3`：`ok`
  - `round 4`：`ok`
  - `round 5`：`ok`
- **结论**：
  - 新增的两类 Rust 特性 bug（trait receiver 声明态/实现态差异、生命周期 method item/HRTB 占位写法）都已经被工具侧修复
  - 在当前 `moonfire-ffmpeg` 批量 1–5 round 复验中，首轮 compile 已达到 `5/5`

### 4.5 2026-04-25 `moonfire-ffmpeg` 批量 round 6–15 扩展验证

- **workspace**：`/tmp/seraph-moonfire-batch-rounds-hashing`
- **说明**：
  - 本轮继续使用真实 `moonfire-ffmpeg` crate，从 `round 6` 一直推进到 `round 15`
  - 由于配置的 SiliconFlow embedding 服务返回 `401 Unauthorized`，本轮使用 `SERAPH_EMBEDDING_BACKEND=hashing` 完成 Phase 2 RAG 检索与 Phase 3 生成验证
- **命中 target**：
  - `round 6`：`api::moonfire_ffmpeg::avformat::InputFormatContext::read_frame`
  - `round 7`：`api::moonfire_ffmpeg::avutil::VideoFrame::owned`
  - `round 8`：`api::moonfire_ffmpeg::Ffmpeg::new`
  - `round 9`：`api::moonfire_ffmpeg::avcodec::AVCodecContext::codec_id`
  - `round 10`：`api::moonfire_ffmpeg::avcodec::AVCodecContext::codec_type`
  - `round 11`：`api::moonfire_ffmpeg::avcodec::AVCodecContext::pix_fmt`
  - `round 12`：`api::moonfire_ffmpeg::avcodec::AVCodecParameters::codec_id`
  - `round 13`：`api::moonfire_ffmpeg::avcodec::AVCodecParameters::codec_type`
  - `round 14`：`api::moonfire_ffmpeg::avcodec::AVCodecParameters::dims`
  - `round 15`：`api::moonfire_ffmpeg::avcodec::CodecId::find_decoder`
- **Phase 3 结果**：
  - `round 6`：compile `ok`，smoke `ok`
  - `round 7`：compile `ok`，smoke `ok`
  - `round 8`：compile `ok`，smoke `ok`
  - `round 9`：compile `ok`，smoke `ok`
  - `round 10`：compile `ok`，smoke `ok`
  - `round 11`：compile `ok`，smoke `ok`
  - `round 12`：compile `ok`，smoke `ok`
  - `round 13`：compile `ok`，smoke `ok`
  - `round 14`：compile `ok`，smoke `ok`
  - `round 15`：compile `ok`，smoke `ok`
- **结论**：
  - 在当前修复后版本中，没有观察到新的首轮 compile 失败，也没有观察到新的 smoke 阶段 Rust 运行时问题
  - 因此本轮**没有新增 Rust 特性 bug 记录项**
  - 这说明前面修复的 receiver 语法、深 setup 链恢复、禁止占位 API 伪引用等改动，对 `moonfire-ffmpeg` 的后续目标具有持续稳定性

### 4.6 2026-04-25 `moonfire-ffmpeg` 全量 55 target 真实 batch

- **workspace**：`/tmp/seraph-moonfire-ffmpeg-alltargets-full-20260425`
- **输入配置**：
  - RAG embedding：SiliconFlow `Qwen/Qwen3-Embedding-8B`
  - Phase 3 LLM：本地 Responses 网关 `gpt-5.4`
  - smoke seed：`/tmp/seraph-real-crates/moonfire-ffmpeg-0.0.2/src/testdata/clip.mp4`
- **总体结果**：
  - public API 总数：`70`
  - target-scope public API（`unsafe fn` 或 `contains_unsafe_block`）总数：`55`
  - harness-generated target coverage：`55 / 55 = 100%`
  - compile-success target coverage：`54 / 55 = 98.18%`
  - smoke-reached target coverage：`54 / 55 = 98.18%`
  - smoke-clean target coverage：`53 / 55 = 96.36%`
  - related API ratio（`Required Setup APIs` + `Related APIs` 的并集，占全部 public API）：`62 / 70 = 88.57%`
  - 如果把 target 自身也算进 neighborhood，则 `target ∪ related = 70 / 70 = 100%`
- **新增观察**：
  - `round 22` 与 `round 43` 暴露出新的 Rust borrow-checker 收敛案例，已记录为 **2.6**
  - `round 33`（`VideoFrame::plane`）在 smoke 中命中 target，但因为 `plane >= 8` 触发 precondition panic，被记录为 runtime error，而不是内存安全 bug
  - `round 54`（`EncodeContext::open`）在显式目标模式下仍无法编译通过，和 Phase 2 constructibility 分析一致：当前库没有公开 owner 构造路径
  - `round 55`（`EncodeContext::set_params`）暴露出新的 diverging-helper 假覆盖问题，已记录为 **3.3**
  - 后续按“target API 也应进入别的 target 的 Related APIs”修正检索后，基于同一份 `knowledge.json + graph.pkl` 重新渲染 55 个上下文，`related API ratio` 已提升到 `70 / 70 = 100%`

### 4.7 2026-04-25 `moonfire-ffmpeg` rerun2：加入 semantic guard 后的诚实复核

- **workspace**：`/tmp/seraph-moonfire-ffmpeg-alltargets-full-20260425-rerun2`
- **复核背景**：
  - 本轮在 target companion 检索修复后，重新渲染并执行全部 `55` 个 target
  - 之后又针对 `round 54` / `round 55` 做了额外 semantic guard 复验
- **复核结论**：
  - target-scope public API 总数：`55`
  - honest compile-success target coverage：`53 / 55 = 96.36%`
  - honest smoke-reached target coverage：`53 / 55 = 96.36%`
  - honest smoke-clean target coverage：`52 / 55 = 94.55%`
  - `Related APIs` 并集覆盖：`70 / 70 = 100%`
- **关键解释**：
  - `round 54`（`EncodeContext::open`）现在会因为“marker 之间没有真实 target 调用”被 semantic guard 拒绝
  - `round 55`（`EncodeContext::set_params`）现在会因为 `MaybeUninit` 伪状态构造被 semantic guard 拒绝
  - 因而这两个 target 不再被计作 compile / smoke 成功
  - 唯一 remaining runtime-bad round 是 `round 33`，它属于 target 已命中后的 panic case，不是 semantic fake coverage
- **设计启示**：
  - 这次复核说明：对 Rust FFI wrapper 类不可构造 target，仅靠 compile success 或 smoke exit code 还不够
  - 必须同时检查：
    - target 是否真的被调用
    - owner 状态是否来自真实 public setup chain
  - 否则 fix-loop 会自然收敛到“类型上能骗过编译器、流程上能骗过 marker”的假 harness

### 4.8 2026-04-25 `moonfire-ffmpeg` honest coverage replay：按最新 writer 重建 `coverage.json`

- **workspace**：`/tmp/seraph-moonfire-ffmpeg-alltargets-full-20260425-rerun2`
- **方法**：
  - 不直接复用旧批量日志里的统计口径
  - 而是使用当前版本的 `s3-coverage` 重新回放 `55` 个 round 的：
    - context
    - compile index / fix-loop index / smoke index / runtime_error index
  - 对 `round 54` 与 `round 55`，使用新的 semantic-guard `recheck` 结果覆盖旧的假成功记录
- **最新重建结果**：
  - target-scope public API 总数：`55`
  - latest honest validated target coverage：`53 / 55 = 96.36%`
  - latest honest uncovered targets：`2 / 55`
    - `api::moonfire_ffmpeg::avcodec::EncodeContext::open`
    - `api::moonfire_ffmpeg::avcodec::EncodeContext::set_params`
  - related API static execution coverage：`66 / 70 = 94.29%`
  - latest honest uncovered related APIs：`4 / 70`
    - `api::moonfire_ffmpeg::avcodec::EncodeContext::open`
    - `api::moonfire_ffmpeg::avcodec::EncodeContext::set_params`
    - `api::moonfire_ffmpeg::avutil::VideoFrame::dims`
    - `api::moonfire_ffmpeg::avutil::VideoFrame::pts`
  - `found_bugs = []`
  - `needs_review = []`
- **为什么 related 不是 `70 / 70`**：
  - 前两个未覆盖 related API 与 target 未覆盖完全同源：它们本身不可从 public setup chain 诚实构造
  - `VideoFrame::dims` 与 `VideoFrame::pts` 虽然会出现在别的 target 的 related context 中，但它们只是 leaf getter
  - 当前 prompt 已明确限制“target 成功后不要追加无关 exercise call”，因此 validated harness 不会为了刷 related coverage 而额外静态调用这些 getter
  - 这不是工具 bug，而是当前“只统计真实使用过的 related API”口径下的合理剩余空白
- **与旧统计的关系**：
  - 旧统计更偏向“batch 当时看起来通过了多少”
  - 新 replay 统计代表“在当前 semantic guard 与 honest overwrite 口径下，系统今天真正认可多少”
  - 因此论文与后续评估应优先引用 replay 后的这组数字
- **证据**：
  - 重建后的覆盖文件：`/tmp/seraph-moonfire-ffmpeg-alltargets-full-20260425-rerun2/coverage.json`
  - 汇总：`/tmp/seraph-moonfire-ffmpeg-alltargets-full-20260425-rerun2/reports/honest_coverage_summary.json`

### 4.9 可直接写入论文的结果表述

可直接复用下面这段文字：

> 在 `moonfire-ffmpeg` 的真实 `55` 个 target-scope public unsafe API 验证中，SERAPH 在引入 constructibility analysis、target-companion retrieval、compile-critical Rust facts、compiler-guided fix-loop，以及 semantic guard 之后，按最新 honest replay 口径达到了 `53/55 (96.36%)` 的 target coverage。剩余 `2` 个未覆盖 target（`EncodeContext::open` 与 `EncodeContext::set_params`）都不是简单的生成失败，而是由于该 crate 没有公开 owner 构造路径；若不增加 semantic guard，LLM 会自然退化到 diverging helper、`MaybeUninit`、marker-only 等“类型上能过、语义上是假的” harness。与此同时，系统对 related API 的静态执行覆盖达到 `66/70 (94.29%)`，剩余未覆盖 mainly 来自两个不可构造 encode target，以及两个无需为了真实性而额外调用的 leaf getter。这个结果说明：Rust unsafe harness synthesis 的关键难点，不是单纯让代码“编译过去”，而是让 target reachability、owner state constructibility 与 runtime behavior 同时保持诚实。 

## 5. 论文写作可直接提炼的结论

可以直接把上述问题归纳成下面这条主线：

1. **Rust unsafe API 的真实可达性，不只由 target 自身决定，还受 wrapper、trait、lifetime、模块导出、trait 完整性、enum 穷尽性等语言机制约束。**
2. **如果工具只做浅层邻域检索或一次性生成，很容易在深层 setup、trait adapter、import path、隐藏不变量这些地方失败。**
3. **SERAPH 的改进策略不是“让首轮 LLM 完美”，而是把 Rust 编译器和运行时语义接入循环：**
   - Phase 2 用结构化图恢复 setup 链和 constructibility
   - Phase 2/3 显式暴露 import path、trait 要求、enum variants 等 compile-critical facts
   - compile-check 用 `rustc` 精确暴露 trait / enum / import 问题
   - fix-loop 用 compiler-guided repair 收敛到可编译 harness
   - smoke-run / runtime_error 保存运行时不变量问题，不把它们静默丢掉
4. **因此，SERAPH 对 Rust 特有语义的适配越强，越能把预算集中到真正“可编译、可运行、可覆盖”的 harness 上，从而提升真实 bug 发现能力。**

## 6. 执行备注

- 本次尝试使用配置的 SiliconFlow embedding 服务时，`/embeddings` 请求返回 `401 Unauthorized`，因此 rerun 采用 `SERAPH_EMBEDDING_BACKEND=hashing` 完成真实验证。
- 这属于部署/鉴权问题，不属于本次 `moonfire-ffmpeg` 工具逻辑修复本身。

## 7. 2026-04-25 `bytes` 真实 Phase 3 验证：public path / unsafe 签名渲染

- **真实 workspace**：
  - 初始：`/tmp/seraph-bytes-phase3-real-20260425`
  - public-path 修复后：`/tmp/seraph-bytes-phase3-real-20260425-rerun1`
  - unsafe-signature 修复后：`/tmp/seraph-bytes-phase3-real-20260425-rerun2`
- **模型配置**：
  - RAG embedding：SiliconFlow `Qwen/Qwen3-Embedding-8B`
  - Phase 3 LLM：本地 Responses 网关 `gpt-5.4`
- **目标 API**：
  - `api::bytes::buf::uninit_slice::UninitSlice::from_raw_parts_mut`

### 7.1 Bug A：RAG context 将 public re-export 类型渲染成 private canonical path

- **现象**：
  - 初始上下文把 target 与 setup/related 的可编译路径渲染成 `bytes::buf::uninit_slice::UninitSlice`
  - LLM 首轮按此生成后，compile 直接报 `E0603: module uninit_slice is private`
- **根因**：
  - `retrieve.py` 在 `Target API` / `Required Setup APIs` / `Related APIs` / `Exact Import Paths` 中优先使用 `canonical_path`
  - 但对 `bytes::buf::UninitSlice` 这种通过 public re-export 暴露的类型，`canonical_path` 对应的是实现模块，不是对外可导入路径
- **修复**：
  - context 渲染层改为优先使用 schema 里的 `public_paths`
  - `Exact Import Paths` 也统一改为优先输出 public path，而不是 private canonical path
- **修复后效果**：
  - `rag_target_001.md` 现在会写：
    - `bytes::buf::UninitSlice::from_raw_parts_mut`
    - `type::bytes::buf::uninit_slice::UninitSlice => bytes::buf::UninitSlice`
  - 首轮 compile 错误从 `E0603` 收敛为后续更细的语义问题，不再卡在模块可见性

### 7.2 Bug B：unsafe API 在 context 中被渲染成普通 `fn` 签名

- **现象**：
  - `knowledge.json` 中 `api::bytes::buf::uninit_slice::UninitSlice::as_uninit_slice_mut` 已正确标记 `is_unsafe = true`
  - 但 context 里 related API 仍显示成 `fn as_uninit_slice_mut(...)`
  - LLM 因此首轮把它当作安全方法直接调用，compile 报 `E0133`
- **根因**：
  - 当前 context 直接透传 `signature_text`
  - 某些 API 的 `signature_text` 没显式带出 `unsafe`，需要结合 `is_unsafe` 重新渲染
- **修复**：
  - 新增统一的 API signature display 逻辑：
    - 若 `is_unsafe = true` 且签名文本未显式带 `unsafe`，则在 context 中补出 `unsafe fn ...`
  - 同样用于：
    - `Target API`
    - `Required Setup APIs`
    - `Related APIs`
    - trait method compile hints
- **修复后效果**：
  - `rag_target_001.md` 中现在明确显示：
    - `unsafe fn from_raw_parts_mut(...)`
    - `unsafe fn as_uninit_slice_mut(...)`
    - `unsafe fn advance_mut(...)`
  - 首轮 compile 错误从 `E0133` 进一步收敛为 `E0502`，说明模型已不再误把 unsafe helper 当作 safe call

### 7.3 真实 rerun 结论

- **首轮失败类型收敛链**：
  - 初始：`E0603`（private module path）
  - rerun1：`E0133`（unsafe call 未放入 unsafe block）
  - rerun2：`E0502`（Rust borrow checker 冲突）
- **解释**：
  - 前两轮失败都属于工具提供给 LLM 的 compile-facing facts 不够准确
  - 修复这两点之后，首轮失败已经下降为普通 Rust 所有权/借用编排问题，这是 compiler-guided fix-loop 应该处理的正常范畴
- **最终结果**：
  - Phase 3 fix-loop 仍在第 `1` 次 attempt 收敛
  - smoke-run 通过，无 panic、无 runtime diagnosis 记录
  - target coverage：`1 / 1 = 100%`
  - related API static execution coverage：`5 / 27 = 18.52%`
- **设计启示**：
  - 对 Rust crate 来说，RAG context 不应把“实现位置”误当成“可编译使用路径”
  - 对 unsafe API 来说，`is_unsafe` 这种结构化字段必须反映到最终 prompt 文本，否则 LLM 会在首轮系统性低估 unsafe 边界
  - 因而 SERAPH 的 compile-facing context 需要优先表达：
    - public re-export path
    - explicit unsafe signature
    - 然后再把借用检查、生命周期、owner state 等难题交给 compile/fix-loop 继续收敛

### 7.4 2026-04-26 `bytes`：加入 borrow-check 规避规则后首轮 compile 通过

- **workspace**：`/tmp/seraph-bytes-phase3-real-20260426-rerun3`
- **新增规则**：
  - 在 Phase 3 system prompt 与 context `Generation Rules` 中显式加入：
    - 创建 `&mut` borrow / mutable view 之前，先取完后面还要用的 index、len、只读字节、克隆源 buffer
    - 一旦拿到指向 owner 的 `&mut` borrow，就不要再回头对原 owner 做读、切片或只读借用
- **修复前现象**：
  - `rerun2` 首轮 harness 已经不再犯 private-path / unsafe-call 错，但仍会写出：
    - `let existing = UninitSlice::new(data.as_mut_slice());`
    - 然后再去读 `&data[..pre_copy_len]` 与 `data.get(1)`
  - 这会触发 Rust borrow checker 的 `E0502`
- **修复后结果**：
  - `rerun3` 首轮 `compile_001_01.json` 直接成功，`fix-loop` 不再介入
  - 生成的首轮 harness 会先计算：
    - `requested_len`
    - `copy_len`
    - 只读源切片
  - 然后才创建 `UninitSlice::new(&mut backing[..])`
- **后续运行时现象**：
  - smoke-run 中仍出现 panic，被 runtime diagnose 归类为 `invalid_input_or_precondition`
  - 原因是 harness 先对长度为 `copy_len` 的源切片调用 `copy_from_slice`，但目标 `UninitSlice` 长度是整个 `backing.len()`，违反了该 API 的长度相等前置条件
  - 这说明：
    - borrow-check 规则已经有效消除了首轮 compile 级别错误
    - 下一层仍需要更强的 API precondition 对齐能力，来降低 smoke 阶段的 panic
- **设计启示**：
  - Rust harness 首轮失败并不都是“模型太差”
  - 很多失败其实来自可以结构化提示的编译期语言规律，例如：
    - public path
    - unsafe header
    - borrow exclusivity
  - 把这些规律前置到 prompt 中，能够把首轮错误从“编译不过”推进到“已编过、开始暴露真实运行时前置条件问题”

### 7.5 2026-04-26 `bytes`：修复三个真实 target 的首轮 compile 缺陷

- **验证目标**：
  - `api::bytes::buf::buf_mut::BufMut::advance_mut`
  - `api::bytes::buf::uninit_slice::UninitSlice::new`
  - `api::bytes::buf::uninit_slice::UninitSlice::uninit`
- **说明**：
  - 这三个失败 target 不是“fix-loop 修两次还没收敛”
  - 它们都属于首轮生成阶段暴露出的 compile-facing 工具缺陷
  - 修复后再用真实 `bytes` crate 单 target 重跑，检查 compile / smoke / coverage

- **Bug A：trait method target 缺少具体 implementor setup**
  - **现象**：
    - `BufMut::advance_mut` 是 trait method target
    - 旧 context 里 `Required Setup APIs` 为空，模型看不到应该选哪个具体实现类型来承接 `Self`
    - 虽然 `knowledge.json` 已经有 `owner_trait_id = trait::bytes::buf::buf_mut::BufMut` 和多个 impl 信息，但 Phase 2 没把这些事实渲染出来
  - **根因**：
    - Phase 2 在 trait-method target 且 receiver 含 `Self` 时，没有把 trait implementor 类型纳入 setup 搜索与 compile hints
  - **修复**：
    - 在 `rag/seraph_rag/retrieve.py` 中：
      - trait-method target 的 setup 搜索加入 implementor type
      - compile hints 的 `Exact Import Paths` 与 trait facts 同步加入 implementor
      - `Generation Rules` 明确要求：trait target 必须落到 `Required Setup APIs` / `Exact Import Paths` / `Required Traits` 给出的具体 implementor 上
  - **验证结果**：
    - 新 context 现在会给出 `BytesMut::new`、`BytesMut::with_capacity`、`bytes::BytesMut`
    - 真实重跑时首轮 harness 直接在 `BytesMut` 上调用 `advance_mut`
    - compile `ok`，smoke `ok`

- **Bug B：`MaybeUninit` 被 semantic guard 误杀**
  - **现象**：
    - `UninitSlice::uninit(&mut [MaybeUninit<u8>])` 的签名本身就要求 `MaybeUninit`
    - 但旧 semantic guard 只要看到 `MaybeUninit` 就拒绝，导致合法 harness 也会被拦截
  - **根因**：
    - 工具把“危险的未初始化状态伪造”与“API 签名显式要求 `MaybeUninit`”混为一谈
  - **修复**：
    - 在 `rag/seraph_rag/compile_check.py` 中移除对裸 `MaybeUninit` 文本的硬拒绝
    - 保留对 `zeroed(`、`transmute(`、`assume_init(` 等真正危险构造的拦截
    - 在 `rag/seraph_rag/harness_prompt.py` 与 `rag/seraph_rag/compile_fixer_bundle.py` 中同步收窄规则：只有在伪造状态时才禁止 `MaybeUninit`
  - **验证结果**：
    - 真实重跑时 harness 合法构造 `Vec<MaybeUninit<u8>>`
    - compile `ok`，smoke `ok`

- **Bug C：`Type::new(owner.as_mut_slice())` 模式下首轮容易触发 `E0502`**
  - **现象**：
    - `UninitSlice::new(owner.as_mut_slice())` 返回依附于 owner 的 `&mut` wrapper
    - 旧 prompt 虽然有泛化的 borrow 规则，但不够具体，模型仍可能在拿到 wrapper 后继续读 owner，触发 `E0502`
  - **根因**：
    - 这是 Rust 特有的借用编排模式；只给“避免借用冲突”这种泛规则，首轮生成不够稳定
  - **修复**：
    - 在 `rag/seraph_rag/retrieve.py` 与 `rag/seraph_rag/harness_prompt.py` 中加入显式规则：
      - 对 `Type::new(owner.as_mut_slice())` 这类模式，要先算完长度、索引、源字节，再创建 `&mut` wrapper
      - wrapper 存活期间不要再读 owner
  - **验证结果**：
    - `UninitSlice::new` 真实重跑时首轮 compile `ok`
    - 说明原来的 compile 缺陷已消失
    - 早先单次 rerun 曾出现过 runtime panic，说明这个 target 还可能受生成细节影响
    - 但本轮 fresh rerun 中 compile `ok`、smoke `ok`
    - 因而当前可以确认：compile 级 root cause 已修复；是否还存在运行时变体，需要后续在更大样本下单独统计

- **本轮回归测试**：
  - `rag/tests/test_compile_check.py::test_run_compile_check_allows_signature_required_maybe_uninit_usage`
  - `rag/tests/test_harness_prompt.py::test_build_prompt_bundle_preserves_rag_context_and_marker_rules`
  - `rag/tests/test_retrieve.py::test_render_context_markdown_trait_target_surfaces_implementor_setup`
  - 以及整组：
    - `pytest -q rag/tests/test_compile_check.py rag/tests/test_harness_prompt.py rag/tests/test_retrieve.py`
    - `pytest -q rag/tests/test_compile_fixer_bundle.py`

- **真实 crate 验证 workspace**：
  - `/tmp/seraph-bytes-bugfix-verify-20260426/advance_mut`
  - `/tmp/seraph-bytes-bugfix-verify-20260426/uninit_new`
  - `/tmp/seraph-bytes-bugfix-verify-20260426/uninit_uninit`
  - `/tmp/seraph-bytes-bugfix-verify-rerun-20260426/advance_mut`
  - `/tmp/seraph-bytes-bugfix-verify-rerun-20260426/uninit_new`
  - `/tmp/seraph-bytes-bugfix-verify-rerun-20260426/uninit_uninit`

- **最终结论**：
  - 三个原始 compile 缺陷都已修复
  - 在本轮新鲜 rerun 中：
    - `advance_mut`：compile `ok`，smoke `ok`
    - `UninitSlice::new`：compile `ok`，smoke `ok`
    - `UninitSlice::uninit`：compile `ok`，smoke `ok`
  - 说明这三个首轮 compile 缺陷已经从真实 `bytes` 流程中消失
  - 先前单次 rerun 里 `UninitSlice::new` 暴露过 runtime panic，但当前 fresh rerun 未复现，说明那一项更像是特定生成结果触发的运行时变体，而不是本次 compile 级 root-cause 修复失败
  - 这轮修复再次说明：
    - Rust trait target 不能只给名字，必须给 implementor
    - Rust 的 `MaybeUninit` 不能一刀切地视为危险构造
    - Rust borrow-check 规则需要按常见 API 形状写成更具体的生成约束
