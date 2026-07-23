# 用 Rust 构建数据库表达式框架

本课程已经重新整理为 mdBook。课程不再把“类型体操”本身当成目标，而是从一个标量执行器
出发，逐步实现数组、借用标量、列视图、运行时类型擦除、表达式绑定和向量化执行。

课程只对确实存在大量类型组合的数值运算和比较运算使用泛型与宏展开；字符串、列表等多数
数据库表达式仍然按照数据类型单独实现。这样既保留编译期类型安全，也避免让每个表达式都做
一遍相同的类型体操。

在线阅读新版 mdBook：

- [课程首页](https://skyzh.github.io/type-exercise-in-rust/)
- [第一部分：从标量执行器开始](https://skyzh.github.io/type-exercise-in-rust/volcano/overview.html)
- [第二部分：构建向量化运行时](https://skyzh.github.io/type-exercise-in-rust/vectorized/overview.html)
- [性能测试与后续方向](https://skyzh.github.io/type-exercise-in-rust/benchmarks.html)

在本地预览课程：

```console
mdbook serve tutorial --open
```

课程源码位于 [`tutorial`](./tutorial)，项目概览见 [`README.md`](./README.md)。

## 社区

欢迎加入 skyzh 的 Discord 服务器，与社区一起学习数据库系统。

[![Join skyzh's Discord Server](tutorial/src/discord-badge.svg)](https://skyzh.dev/join/discord)
