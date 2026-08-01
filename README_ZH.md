# 用 Rust 构建数据库表达式框架

本课程从仓库中明确提供的
[`type-exercise-starter`](./type-exercise-starter) 开始。初始代码只有一个拥有所有权的标量
枚举，不会预先实现标量、借用值和数组之间的 trait 关联。

当前可审阅范围包含两个章节：

1. 用 trait 和泛型关联类型连接 `i32`、`String`、借用值与物理数组；
2. 通过统一的借用接口读取普通数组、重复常量和字典编码数据。

请从 `skyzh/course-starter` 分支开始练习，并在 [`course`](./course) 中阅读
[第一章](./course/src/chapter-1-type-connections.md)与
[第二章](./course/src/chapter-2-column-views.md)：

```console
git fetch origin
git switch --create course-work --track origin/skyzh/course-starter
cargo test --manifest-path type-exercise-starter/Cargo.toml --locked
mdbook serve course --open
```

仓库根目录只是课程容器，不是 Cargo 工作区。`type-exercise-starter/` 是唯一的学员工作区；
[`archived/`](./archived) 下的旧实现既不是练习依赖，也不属于章节修改范围。完成章节回顾前，
请只修改 `type-exercise-starter/`，以免提前看到实现。

## 社区

欢迎加入 [skyzh 的 Discord 服务器](https://skyzh.dev/join/discord)，与社区一起学习。
