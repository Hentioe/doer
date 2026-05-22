<h1 align=center><code>doer</code></h1>

一个命令的管理器、复用器和运行器。

## 介绍

doer 把一系列命令组织成 KDL 配置结构，用于管理和运行。它以方便、明确的方式编写可复用任务：

```kdl
tasks {
    test "cargo test --all"
    release {
        - "cargo build --bin {bin} --release"
        arg bin
        dep test
        env {
            RUST_BACKTRACE full
        }
    }
    precommit {
        - "cargo fmt --all -- --check"
        - "cargo clippy --all-targets -- -D warnings"
    }
    prepush {
        dep precommit
        dep test
    }
}
```

友好和高度一致性的方式调用：

```plaintext
Usage: doer [TASK] [ARGS] [OPTS]...

Available tasks:
  test
  release <bin>
  precommit
  prepush
```

## 特点

- [x] 用 KDL 格式组织任务，结构清晰，易于理解。空间利用合理。
- [x] 命令模板支持参数和选项（带默认值的参数），使用相同的插值语法。
- [x] 支持依赖任务，允许向依赖任务传递参数/选项。支持插值引用参数/选项。
- [x] 支持环境变量，环境变量值支持插值引用参数/选项。
- [x] 支持工作目录（cwd），工作目录支持插值引用参数/选项。
- [x] 支持后台运行依赖任务。
- [x] 支持指定任务的运行用户。
- [ ] 支持指定任务的 `nice` 值。

## 安装

_目前您需要从源代码构建安装 doer。_

### 先决条件

确保已安装 Rust 和 Cargo：

```sh
$ rustc --version
rustc 1.95.0
$ cargo --version
cargo 1.95.0
```

### 过程

克隆本仓库到本地，运行 Cargo 命令：

```sh
cargo install --path crates/doer
```

## 配置细节

参考[博客文章](https://blog.hentioe.dev/posts/doer.html)。

## 许可

MIT 许可证。参阅 [LICENSE](LICENSE)。
