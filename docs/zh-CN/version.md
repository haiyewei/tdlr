# `version` 命令

`version` 用于输出当前程序构建信息。

## 用法

```bash
tdlr version
```

## 输出内容

命令会输出以下字段：

- `Version`
- `Rustc`
- `Target`

其中：

- `Version` 来自构建期注入的 `TDLR_VERSION`
- `Rustc` 是编译时使用的 Rust 编译器版本
- `Target` 是当前运行平台的 `OS/ARCH`

## 示例

```bash
tdlr version
```

## 参考

| 文件 | 说明 |
|------|------|
| `build.rs` | 构建期版本信息注入 |
| `src/commands/version.rs` | 命令实现 |
