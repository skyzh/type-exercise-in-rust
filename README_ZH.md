# 用 Rust 构建类型安全的数据库表达式引擎

这是一门完整的实践课程：五个模块包含十个累积且可独立测试的检查点，共同构建一个小型、
类型安全的向量化表达式引擎。学习者实现标量运算；共享的泛型求值器负责数组、常量、
索引视图、空值、类型擦除、绑定以及异步边界。每个检查点预计约半天，有 Rust 经验的
学习者大约可在五个工作日内完成。

五个模块按实际构建顺序组织：

- **Type families and nullable views（类型族与可空视图，检查点 1–2）**
- **Shared evaluation and transactional strings（共享求值与事务式字符串，检查点 3–4）**
- **Shape specialization and binary semantics（形状特化与二元语义，检查点 5–6）**
- **Runtime erasure and the physical catalog（运行时擦除与物理目录，检查点 7–8）**
- **Logical binding, one-level Lists, and batch async（逻辑绑定、单层 List 与批量异步，检查点 9–10）**

十个检查点保留各自的编号和测试反馈，第 10 个检查点是课程终点。

课程从第一个检查点起使用两个单向依赖的 crate：

- `type-exercise-starter-core` 保存数组、视图和共享求值框架；
- `type-exercise-starter-expr` 保存具体的算术、比较、布尔、字符串和绑定逻辑。

请按顺序阅读[在线课程](https://skyzh.github.io/type-exercise-in-rust/)，并只在
`type-exercise-starter/` 中完成检查点。每章先运行 `cargo x copy-test --chapter N`，
再使用该章给出的 `type-exercise-starter-supplied-tests` 测试命令完成当前检查点；实现代码
仍分别位于真实的 `core/src/` 与门面 `expr/src/` 目录中。完成最后一章时运行
`cargo x copy-test --chapter 10`，即可复制完整的累积测试契约。

源代码采用 Apache 2.0 许可证；课程文字采用
[CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/) 许可证。
