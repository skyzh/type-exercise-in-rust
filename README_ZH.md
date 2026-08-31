# 用 Rust 构建类型安全的数据库表达式引擎

本课程通过十四天的连续练习，构建一个小型、类型安全的向量化表达式引擎。学习者只需
实现标量运算；共享的泛型求值器负责数组、常量、索引视图、空值、类型擦除、绑定以及
异步边界。

课程从第一天起使用两个单向依赖的 crate：

- `type-exercise-starter-core` 保存数组、视图和共享求值框架；
- `type-exercise-starter-expr` 保存具体的算术、比较、布尔、字符串和绑定逻辑。

请按顺序阅读[在线课程](https://skyzh.github.io/type-exercise-in-rust/)，并只在
`type-exercise-starter/` 中完成练习。每章先运行 `cargo x copy-test --chapter N`，
再使用该章给出的 `type-exercise-starter-expr` 测试命令完成当前检查点。

源代码采用 Apache 2.0 许可证；课程文字采用
[CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/) 许可证。
