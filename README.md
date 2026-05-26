<h1 align=center><code>doer</code></h1>

一个命令的管理器、复用器和运行器。

## 介绍

doer 把一系列命令组织成 KDL 配置结构，用于管理和运行。它以高可读性、明确的方式编写可复用任务：

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

友好和高一致性的方式调用：

```plaintext
Usage: doer [TASK] [ARGS] [OPTS]...

Available tasks:
  test
  release <bin>
  precommit
  prepush
```

详细请参考[博客文章](https://blog.hentioe.dev/posts/doer.html)。

## 特点

- [x] 全部模板使用相同的插值语法。
- [x] 支持位置参数和带默认值的参数（选项)。
- [x] 支持布尔选项：用于表达 `--enabled` 这种可有可无的参数。
- [x] 支持剩余参数：将未匹配参数插值给命令。
- [x] 支持依赖任务：可传递参数/选项，支持插值引用。
- [x] 支持环境变量：变量值支持插值引用。
- [x] 支持工作目录（cwd）：工作目录支持插值引用。
- [x] 支持后台运行依赖任务。
- [x] 支持指定任务的运行用户。
- [x] 支持管理任务/依赖的输入输出。
- [x] 支持自动安装 Git 钩子。
- [x] 支持指定任务/依赖的 `nice` 值（进程优先级）。

## 未来计划

- 有效值验证
- 并行支持
- 内置变量支持
- 命名空间支持（或模块支持）
- 子目录调用支持
- 自定义 Shell 支持
- 自定义插值语法

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
cargo run -- install
```

_此命令会调用 doer 自身来执行安装任务，其中有一个步骤需要 `root` 权限给予 `CAP_SYS_NICE` 能力（用于调整 nice 值）。_

## 许可

MIT 许可证。参阅 [LICENSE](LICENSE)。
