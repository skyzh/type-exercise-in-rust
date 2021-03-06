# Rust 语言中的类型体操

**本 README 已经很久没有更新了，本仓库的内容以首页 [README](README.md) 为准**

这段简短的讲座讲述如何使用 Rust 类型系统在数据库中构建所需的组件。

讲座围绕着 Rust 程序员（例如我）如何用 Rust 编程语言构建数据系统演进。我们利用类型 Rust 类型系统**最小化**运行时成本，并且用**安全**、 **nightly** 的 Rust 进行我们的开发流程。

![类型的映射](map-of-types.png)

## 第 1 天： `Array` 与 `ArrayBuilder`

`ArrayBuilder` 与 `Array` 是互逆的特质（trait）。 `ArrayBuilder` 创建 `Array`，同时我们能用 `ArrayBuilder` 与已存在的 `Array` 创建新数组。在第 1 天，我们用原始类型（如 `i32`、 `f32`）和变长类型（如 `String`）实现数组。我们用特质中的关联类型推导泛型函数中的正确类型，并用 GAT 统一面对定长和变长类型的 `Array` 接口。此框架也和 arrow 这种库非常类似，不过它带有更强的类型约束和更低的运行时开销。

## 第 2 天： `Scalar` 与 `ScalarRef`

`Scalar` 与 `ScalarRef` 是互逆的类型。我们能获得 `Scalar` 的引用 `ScalarRef`，并将 `ScalarRef` 转换回 `Scalar`。通过添加这些特质，我们能在匹配与转换上写更多的零运行时开销泛型函数。同时，我们将 `Scalar` 与 `Array` 关联，以便更容易地写函数。

# 待完成的讲座

## 第 3 天： `ArrayImpl`、 `ArrayBuilderImpl`、 `ScalarImpl` 与 `ScalarRefImpl`

可能到运行时之前无法取得某些信息。继而我们用 `XXXImpl` 枚举覆盖单一类型的所有变体。

## 第 4 天：用宏处理更多类型

由于我们有了越来越多的数据类型，我们需要多次在 match 选择支内写同样的代码。在第 4 天，我们用声明宏（而非过程宏或其他种类的代码生成器）生成这种代码，并避免书写重复累赘的代码。

## 第 5 天：二元表达式

现在我们有了 `Array`、 `ArrayBuilder`、 `Scalar` 及 `ScalarRef`，我们能用泛型将之前写的每个函数都转换成向量化版本。

## 第 6 天：聚合器

聚合器是另一类表达式。我们在第 6 天学习如何简易地用我们的类型系统实现它们。

## 第 7 天：表达式框架

现在我们有了越来越多的表达式种类，而我们需要一个表达式框架统一他们——包括一元、二元和有更多输入的表达式。同时，我们也需要用 `TryFrom` 与 `TryInto` 特质自动地转换 `ArrayImpl` 到其对应的具体类型。

同时，我门还将实验在变长类型中进行返回值优化。

## 第 8 天：物理数据类型与逻辑数据类型

`i32`、 `i64` 是简单物理类型——这些类型如何存储于内存（或磁盘）。但在数据库系统中，我们还有逻辑类型（如 `Char` 和 `Varchar`）。在第 8 天，我们学习如何用宏关联逻辑类型与物理类型。
