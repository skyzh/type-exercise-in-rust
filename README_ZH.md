# 用 Rust 构建数据库表达式框架

本课程已经重新整理为五天的 mdBook 课程。课程从已有的数组、借用标量和表达式模板出发，
逐步实现零拷贝列视图、规划期表达式绑定、原生类型快速路径、更严格的 Rust 类型边界，以及
批次级异步适配器。

课程只对确实存在大量类型组合的数值运算和比较运算使用泛型与宏展开；字符串、列表等多数
数据库表达式仍然按照数据类型单独实现。这样既保留编译期类型安全，也避免让每个表达式都做
一遍相同的类型体操。

在线阅读新版 mdBook：

- [课程首页](https://skyzh.github.io/type-exercise-in-rust/)
- [第一天：统一读取数组、常量和字典列](https://skyzh.github.io/type-exercise-in-rust/day-1-column-views.html)

在本地预览课程：

```console
mdbook serve course --open
```

课程源码位于 [`course`](./course)，项目概览见 [`README.md`](./README.md)。

## 社区

欢迎加入 [skyzh 的 Discord 服务器](https://skyzh.dev/join/discord)，与社区一起学习数据库系统。
