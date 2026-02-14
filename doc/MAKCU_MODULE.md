# MAKCU 盒子键鼠功能模块

## 概述

`makcu` 模块是一个独立的 Rust 模块，用于通过串口与 MAKCU 硬件盒子通信，实现硬件级的鼠标和键盘控制功能。该模块完全基于 MAKCU 开发文档实现，提供了完整的 API 接口。

## 模块结构

```
src/makcu/
├── mod.rs          # 模块入口，导出公共接口
├── error.rs        # 错误类型定义
├── config.rs       # 配置结构
├── client.rs       # 核心客户端类
├── mouse.rs        # 鼠标控制接口
├── keyboard.rs     # 键盘控制接口
└── led.rs          # LED 控制接口
```

## 核心组件

### 1. MakcuClient

核心客户端类，负责串口通信和命令执行。

#### 初始化

```rust
use nzm_cmd::makcu::{MakcuClient, MakcuConfig};

// 使用默认配置
let config = MakcuConfig::new("COM3");
let mut client = MakcuClient::new(config)?;

// 自定义配置
let config = MakcuConfig::new("COM3")
    .with_baud_rate(115200)
    .with_timeout(100)
    .with_screen_size(1920, 1080);
let mut client = MakcuClient::new(config)?;
```

#### 基础方法

```rust
// 发送命令并等待响应
let response = client.send_command(".help()\r\n")?;

// 发送命令不等待响应
client.send_command_no_wait(".reboot()\r\n")?;

// 获取最后一次响应
let last_response = client.get_last_response();

// 清空缓冲区
client.clear_buffer();

// 发送二进制帧
let frame = vec![0xDE, 0xAD, 0x02, 0x01, 0x55];
client.send_binary_frame(&frame)?;
```

### 2. 鼠标控制

#### 按键控制

```rust
use nzm_cmd::makcu::{MouseButtons, MouseControl};

// 按下左键
client.mouse_left(Some(1))?;

// 释放左键
client.mouse_left(Some(0))?;

// 查询左键状态
let state = client.mouse_left(None)?;

// 点击（默认1次）
client.mouse_click(MouseButtons::Left, 1)?;

// 点击3次，每次间隔50ms
client.mouse_click_with_delay(MouseButtons::Left, 3, 50)?;
```

#### Turbo 模式

```rust
// 启用左键连发，延迟500ms
client.mouse_turbo(MouseButtons::Left, 500)?;

// 禁用左键连发
client.mouse_disable_turbo(MouseButtons::Left)?;

// 禁用所有连发
client.mouse_disable_all_turbo()?;
```

#### 鼠标移动

```rust
// 相对移动
client.mouse_move(100, -50, None, None)?;

// 分段移动（8段）
client.mouse_move(100, -50, Some(8), None)?;

// 贝塞尔曲线移动
let control_points = [(40, 25), (80, 10)];
client.mouse_move(100, 50, Some(8), Some(control_points))?;

// 绝对定位
client.mouse_moveto(640, 360, None, None)?;

// 贝塞尔曲线绝对定位
client.mouse_moveto(100, 50, Some(8), Some(control_points))?;
```

#### 滚轮和轴控制

```rust
// 滚轮滚动（1=向上，-1=向下）
client.mouse_wheel(1)?;

// 水平滚动
client.mouse_pan(3)?;

// 倾斜滚动
client.mouse_tilt(2)?;

// 获取当前位置
let pos = client.mouse_getpos()?;

// 静默点击
client.mouse_silent(400, 300)?;
```

### 3. 键盘控制

```rust
use nzm_cmd::makcu::{Key, SystemKey, ModifierKey};

// 按下按键
client.keyboard_down(Key::Letter('a'))?;

// 释放按键
client.keyboard_up(Key::Letter('a'))?;

// 按键（随机35-75ms）
client.keyboard_press(Key::Letter('a'), None, None)?;

// 按键（精确50ms）
client.keyboard_press(Key::Letter('a'), Some(50), None)?;

// 按键（50ms + 随机0-10ms）
client.keyboard_press(Key::Letter('a'), Some(50), Some(10))?;

// 输入字符串
client.keyboard_string("Hello World!")?;

// 初始化键盘（释放所有按键）
client.keyboard_init()?;

// 查询按键状态
let is_down = client.keyboard_isdown(Key::Modifier(ModifierKey::LeftCtrl))?;

// 禁用按键
client.keyboard_disable(vec![
    Key::Letter('a'),
    Key::Letter('c'),
    Key::Letter('f'),
])?;

// 启用按键
client.keyboard_enable(Key::Letter('a'))?;

// 重映射按键
client.keyboard_remap(
    Key::Letter('a'),
    Key::Letter('b'),
)?;

// 清除重映射
client.keyboard_clear_remap(Key::Letter('a'))?;

// 重置所有重映射
client.keyboard_reset_remap()?;
```

### 4. LED 控制

```rust
use nzm_cmd::makcu::{LedTarget, LedMode};

// 查询设备LED状态
let state = client.led_query(LedTarget::Device)?;

// 打开设备LED
client.led_set(LedTarget::Device, LedMode::On)?;

// 关闭设备LED
client.led_set(LedTarget::Device, LedMode::Off)?;

// 慢闪
client.led_set(LedTarget::Device, LedMode::SlowBlink)?;

// 快闪
client.led_set(LedTarget::Device, LedMode::FastBlink)?;

// 闪烁3次，每次间隔200ms
client.led_blink(LedTarget::Device, 3, 200)?;
```

### 5. 系统命令

```rust
// 获取帮助
let help = client.help()?;

// 获取设备信息
let info = client.info()?;

// 获取固件版本
let version = client.version()?;

// 获取设备类型
let device = client.device()?;

// 重启设备
client.reboot()?;

// 设置序列号
client.serial(Some("MAKCU001"))?;

// 查询序列号
let serial = client.serial(None)?;

// 设置日志级别
client.log(Some(3))?;

// 启用/禁用回显
client.echo(Some(true))?;

// 设置波特率
client.baud(Some(115200))?;

// 绕过模式
client.bypass(Some(1))?;

// 高速模式
client.hs(Some(true))?;

// 释放按键
client.release(Some(100))?;

// 获取故障信息
let fault = client.fault()?;
```

### 6. 流式数据

```rust
// 流式键盘数据
client.stream_keyboard(2, 50)?;

// 流式按键状态
client.stream_buttons(2, 25)?;

// 流式轴数据
client.stream_axis(1, 25)?;

// 流式鼠标数据
client.stream_mouse(2, 25)?;
```

## 错误处理

模块使用 `MakcuError` 和 `MakcuResult` 进行错误处理：

```rust
use nzm_cmd::makcu::{MakcuError, MakcuResult};

fn example() -> MakcuResult<()> {
    let config = MakcuConfig::new("COM3");
    let mut client = MakcuClient::new(config)?;

    client.mouse_click(MouseButtons::Left, 1)?;

    Ok(())
}

fn handle_error() {
    match example() {
        Ok(_) => println!("成功"),
        Err(MakcuError::SerialPortError(msg)) => {
            eprintln!("串口错误: {}", msg);
        }
        Err(MakcuError::TimeoutError) => {
            eprintln!("操作超时");
        }
        Err(e) => {
            eprintln!("错误: {}", e);
        }
    }
}
```

## 配置选项

### MakcuConfig

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| port_name | String | "COM3" | 串口名称 |
| baud_rate | u32 | 115200 | 波特率 |
| timeout_ms | u64 | 100 | 超时时间（毫秒） |
| screen_width | u16 | 1920 | 屏幕宽度 |
| screen_height | u16 | 1080 | 屏幕高度 |

## 使用示例

### 示例1：基础鼠标操作

```rust
use nzm_cmd::makcu::{MakcuClient, MakcuConfig, MouseButtons};

fn basic_mouse_example() -> nzm_cmd::makcu::MakcuResult<()> {
    let config = MakcuConfig::new("COM3");
    let mut client = MakcuClient::new(config)?;

    // 移动鼠标到指定位置
    client.mouse_moveto(640, 360, None, None)?;

    // 点击左键
    client.mouse_click(MouseButtons::Left, 1)?;

    Ok(())
}
```

### 示例2：键盘输入

```rust
use nzm_cmd::makcu::{MakcuClient, MakcuConfig, Key, ModifierKey};

fn keyboard_example() -> nzm_cmd::makcu::MakcuResult<()> {
    let config = MakcuConfig::new("COM3");
    let mut client = MakcuClient::new(config)?;

    // 输入文本
    client.keyboard_string("Hello, World!")?;

    // 按下 Ctrl+C
    client.keyboard_down(Key::Modifier(ModifierKey::LeftCtrl))?;
    client.keyboard_press(Key::Letter('c'), None, None)?;
    client.keyboard_up(Key::Modifier(ModifierKey::LeftCtrl))?;

    Ok(())
}
```

### 示例3：贝塞尔曲线移动

```rust
use nzm_cmd::makcu::{MakcuClient, MakcuConfig};

fn bezier_move_example() -> nzm_cmd::makcu::MakcuResult<()> {
    let config = MakcuConfig::new("COM3");
    let mut client = MakcuClient::new(config)?;

    // 使用贝塞尔曲线平滑移动
    let control_points = [(100, 50), (200, 100)];
    client.mouse_move(300, 200, Some(8), Some(control_points))?;

    Ok(())
}
```

### 示例4：Turbo 模式

```rust
use nzm_cmd::makcu::{MakcuClient, MakcuConfig, MouseButtons};

fn turbo_example() -> nzm_cmd::makcu::MakcuResult<()> {
    let config = MakcuConfig::new("COM3");
    let mut client = MakcuClient::new(config)?;

    // 启用左键连发，每500ms触发一次
    client.mouse_turbo(MouseButtons::Left, 500)?;

    // 按住左键（连发会自动触发）
    client.mouse_left(Some(1))?;

    // 等待一段时间
    std::thread::sleep(std::time::Duration::from_secs(5));

    // 释放左键
    client.mouse_left(Some(0))?;

    // 禁用连发
    client.mouse_disable_turbo(MouseButtons::Left)?;

    Ok(())
}
```

### 示例5：LED 控制

```rust
use nzm_cmd::makcu::{MakcuClient, MakcuConfig, LedTarget, LedMode};

fn led_example() -> nzm_cmd::makcu::MakcuResult<()> {
    let config = MakcuConfig::new("COM3");
    let mut client = MakcuClient::new(config)?;

    // 闪烁设备LED 3次
    client.led_blink(LedTarget::Device, 3, 200)?;

    // 打开主机LED
    client.led_set(LedTarget::Host, LedMode::On)?;

    Ok(())
}
```

## 注意事项

1. **串口权限**：确保程序有访问串口的权限
2. **波特率匹配**：确保波特率设置与设备一致
3. **超时设置**：根据实际网络情况调整超时时间
4. **资源释放**：`MakcuClient` 实现了 `Drop` trait，会自动释放资源
5. **错误处理**：所有操作都可能返回错误，建议使用 `?` 操作符或显式处理错误

## 性能优化

1. **批量操作**：尽量批量发送命令减少串口通信次数
2. **无等待模式**：对于不需要响应的命令使用 `send_command_no_wait`
3. **合理超时**：根据实际情况设置合适的超时时间

## 扩展开发

模块设计为可扩展的，您可以：

1. 添加新的命令支持
2. 实现自定义的错误处理
3. 扩展配置选项
4. 添加日志记录功能

## 依赖项

模块依赖以下 crate：

- `serialport`：串口通信
- `std`：标准库

## 许可证

本模块遵循项目许可证。
