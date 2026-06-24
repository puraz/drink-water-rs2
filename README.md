# 喝水提醒

一个托盘常驻的喝水提醒工具。到点弹通知，可以记今天喝了几杯。

## 功能

- 托盘图标，定时弹通知提醒喝水
- 提醒间隔、饮水量、工作时段都可以调
- 今日喝水计数，菜单栏直接看
- 设置窗口改完自动保存，不用点确定
- 配置文件改了自动重载，不用重启程序

## 下载

发布页下载对应平台的压缩包，解压就能用。

**macOS**: 解压后把 `喝水提醒.app` 拖进 `/Applications`。首次运行要右键 -> 打开（Gatekeeper 会拦一下）。

**Windows**: 解压到任意目录，双击 `drink-water-rs2.exe`。

## 构建

```bash
cargo build --release

# 生成图标（assets/icon.png，仓库里已经有，一般不需要重新跑）
cargo run --release --bin gen-icons

# macOS 打包 .app
bash scripts/bundle.sh
```

构建产物：

- `target/release/drink-water-rs2` — 主程序
- `target/release/drink-water-settings` — 设置界面
- `target/喝水提醒.app` — macOS bundle（运行 `bundle.sh` 后生成）

## 开发

```bash
# 跑主程序看日志
RUST_LOG=debug cargo run --bin drink-water-rs2

# 跑设置窗口
cargo run --bin drink-water-settings

# 代码检查
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

### 项目结构

```
src/
  main.rs              # 托盘主程序
  reminder.rs          # 定时提醒 + 通知（macOS 用 osascript，其他用 notify-rust）
  config.rs            # 配置读写（JSON）
  stats.rs             # 每日喝水统计
  icon.rs              # 程序化生成水滴图标 RGBA
  lib.rs               # 导出 config, icon
  bin/
    settings.rs        # Iced 写的设置窗口
    gen_icons.rs       # 图标生成工具
scripts/
  bundle.sh            # macOS .app 打包脚本
```

## 依赖

- [tray-icon](https://crates.io/crates/tray-icon) — 托盘图标
- [tao](https://crates.io/crates/tao) — 事件循环
- [notify-rust](https://crates.io/crates/notify-rust) — 系统通知（Windows Toast、Linux DBus）
- [iced](https://crates.io/crates/iced) — 设置界面 GUI
- [serde](https://crates.io/crates/serde) / [serde_json](https://crates.io/crates/serde_json) — 配置序列化
- [chrono](https://crates.io/crates/chrono) — 时间处理
