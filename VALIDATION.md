# 验证记录

## 已完成

- 原 RAR 已完整解包，原 Python 项目保留在 `legacy-python/`。
- React + Mantine 前端已执行 `npm install`。
- 前端已执行 `npm run build`，TypeScript 与 Vite 构建通过。
- `recognition_config.json`、模板目录、ADB 文件与图标资源已迁入 Tauri resources。
- 16 个 Rust 源文件已完成语法树解析检查。
- 已核对 MaaFramework Rust API 所用的设备发现、EmulatorExtras、截图、点击和 BGR 缓冲接口。
- 已添加 Windows GitHub Actions，能在 Windows runner 上执行前端构建和 `cargo check`。

## 需要在 Windows 实机完成

当前生成环境没有 Rust 工具链、Windows WebView2、模拟器和 MaaFramework DLL，因此以下项目无法在这里完成运行验证：

1. `cargo check` 与 Tauri EXE 链接。
2. MuMu 12 或雷电 9 的 EmulatorExtras 连接。
3. 实际帧分辨率、方向和模板区域校准。
4. 游戏内完整一轮自动操作。

请在 Windows 电脑上先运行：

```text
scripts\download-maa-runtime.ps1
run-dev.bat
```

连接模拟器后，先执行“保存测试截图”和“识别测试”，确认画面与模板区域一致，再执行正式跑图。
