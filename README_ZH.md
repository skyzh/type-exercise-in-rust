# 用 Rust 构建类型安全的数据库表达式引擎

这是一门完整的实践课程：七个模块包含十四个可独立测试的实验，共同构建一个小型、
类型安全的向量化表达式引擎。学习者实现标量运算；共享的泛型求值器负责数组、常量、
索引视图、空值、类型擦除、绑定以及异步边界。每个实验预计约半天，有 Rust 经验的
学习者大约可在七个工作日内完成。

七个模块按实际构建顺序组织：

- **Type families（类型族，1–2）**
- **Borrowed columns and first batch evaluation（借用列与首次批量求值，3–4）**
- **Generic numeric evaluation（泛型数值求值，5–6）**
- **Specialized execution and Boolean nulls（特化执行与布尔空值，7–8）**
- **Runtime expressions and variable-width output（运行时表达式与变长输出，9–10）**
- **Logical binding and nested storage（逻辑绑定与嵌套存储，11–12）**
- **Thread-safe and async boundaries（线程安全与异步边界，13–14）**

十四个实验仍保留各自的编号、检查点和测试反馈。

课程从第一个实验起使用两个单向依赖的 crate：

- `type-exercise-starter-core` 保存数组、视图和共享求值框架；
- `type-exercise-starter-expr` 保存具体的算术、比较、布尔、字符串和绑定逻辑。

请按顺序阅读[在线课程](https://skyzh.github.io/type-exercise-in-rust/)，并只在
`type-exercise-starter/` 中完成实验。每章先运行 `cargo x copy-test --chapter N`，
再使用该章给出的 `type-exercise-starter-supplied-tests` 测试命令完成当前检查点；实现代码
仍分别位于真实的 `core/src/` 与门面 `expr/src/` 目录中。

源代码采用 Apache 2.0 许可证；课程文字采用
[CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/) 许可证。
