# 2026 夏活辅助

用于自动执行第七史诗 2026 夏活幸运冲刺小游戏。

## 核心功能

- 支持普通 ADB 截图和 MAA 自动截图。
- MAA 截图失败时可自动回退到 ADB。
- 自动识别当前米数、保护数量、幸运等级、奖励页面和结束页面。
- 支持装备分优先动态策略。
- 支持 200 米优先固定路线：`跑跑冲保保幸保冲幸幸`。
- 支持 LV0 至 LV5 幸运数量、上限和补充米数规则。
- 支持 100 米和 200 米资源补充。
- 支持保护失败处理、技能选中确认和延迟结算。
- 支持助跑后退动画期间锁定当前动作。
- 200 米后停止识别米数，自动处理奖励页面并等待 cutdown。
- 自动保存设备、截图方式、策略和其他设置。

## 目录

```text
src/                         React + Mantine UI
src-tauri/src/               Rust 后端
  controller/                ADB 与 MAA EmulatorExtras
  recognition/               区域裁剪、模板匹配、状态读取
  game/                      状态转移、动态规划、固定策略、runner
src-tauri/resources/         模板、识别配置、ADB
src-tauri/runtime/maa/       MaaFramework runtime
legacy-python/               原始 Python 工程备份
scripts/                     MAA 下载、开发、打包脚本
```

## Windows 环境

需要：

1. Node.js 20 或更高版本。
2. Rust stable MSVC 工具链。
3. Microsoft Edge WebView2 Runtime。
4. Visual Studio Build Tools 2022，勾选“使用 C++ 的桌面开发”。

安装 Rust：

```powershell
winget install Rustlang.Rustup
rustup default stable-x86_64-pc-windows-msvc
```

## 准备 MaaFramework runtime

```powershell
powershell -ExecutionPolicy Bypass -File scripts\download-maa-runtime.ps1
```

脚本从 MaaFramework 官方 Release 下载 `MAA-win-x86_64-v*.zip`，解压到 `src-tauri/runtime/maa/`。

## 开发运行

双击：

```text
run-dev.bat
```

或：

```powershell
npm install
npm run tauri dev
```


## MAA 截图说明

在 UI 的“截图后端”中选择 `MAA EmulatorExtras`。程序会：

1. 加载 `MaaFramework.dll`。
2. 使用 MaaToolkit 发现模拟器和对应配置。
3. 强制使用 `MaaAdbScreencapMethod_EmulatorExtras`。
4. 直接读取 MAA 返回的 BGR 原始帧并转换成 RGB 图像。
5. 如果模拟器或版本不支持 EmulatorExtras，并且开启了回退，会切到普通 ADB 截图。

建议先点“保存测试截图”，确认画面分辨率、方向和区域位置与原模板一致。

## 设置保存位置

设置保存在 Tauri 应用数据目录中的：

```text
lucky_sprint_settings.json
```

卸载或重新打包不会覆盖已有用户设置。

## 当前验证状态

前端源代码、资源路径、JSON 配置和工程结构已完成静态检查。由于生成环境没有安装 Rust 工具链，也没有 Windows 模拟器与 MaaFramework DLL，最终的 Windows `cargo check`、真机 MAA 连接和 EXE 运行需要在你的 Windows 电脑上执行 `run-dev.bat` 验证。

## 一键检查

```powershell
powershell -ExecutionPolicy Bypass -File scripts\validate.ps1
```

该脚本会检查必要资源、构建 Mantine 前端，并在已安装 Rust 时执行 `cargo check`。
