# Android 集成指南

本文档介绍如何将 `tdlr` 集成到 Android 应用中。

## 前置要求

1. **Android NDK** (`r21` 或更高版本)
2. **Rust** (`1.70+`)
3. **Android 目标工具链**

```bash
# 安装 Android 目标
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android
rustup target add i686-linux-android
```

## 编译

### Windows (PowerShell)

```powershell
$env:ANDROID_NDK_HOME = "C:\Android\ndk\26.1.10909125"
.\scripts\build-android.ps1
```

### Linux/macOS

```bash
export ANDROID_NDK_HOME=/path/to/android-ndk-r26b
./scripts/build-android.sh
```

编译完成后，库文件位于 `target/android/jniLibs/` 目录：

```text
target/android/jniLibs/
├── arm64-v8a/libtdlr.so
├── armeabi-v7a/libtdlr.so
├── x86_64/libtdlr.so
└── x86/libtdlr.so
```

## Android 项目集成

### 1. 复制库文件

将 `jniLibs` 目录复制到 Android 项目的 `app/src/main/` 目录下。

### 2. 创建 JNI 包装类

```kotlin
// app/src/main/java/com/tdlr/TdlrNative.kt
package com.tdlr

import org.json.JSONObject

class TdlrNative {
    companion object {
        init {
            System.loadLibrary("tdlr")
        }
    }

    private external fun initRuntime(): Long
    private external fun destroyRuntime(handle: Long)
    private external fun download(handle: Long, url: String, outputPath: String, accountId: Long): String
    private external fun getVersion(): String
    private external fun hasSession(accountId: Long): Boolean
    private external fun setSessionDir(path: String): Boolean
    private external fun setApiCredentials(apiId: String, apiHash: String): Boolean

    private var runtimeHandle: Long = 0

    fun init(): Boolean {
        runtimeHandle = initRuntime()
        return runtimeHandle != 0L
    }

    fun destroy() {
        if (runtimeHandle != 0L) {
            destroyRuntime(runtimeHandle)
            runtimeHandle = 0
        }
    }

    fun version(): String = getVersion()

    fun sessionExists(accountId: Long): Boolean = hasSession(accountId)

    fun configureSessionDir(path: String): Boolean = setSessionDir(path)

    fun configureApi(apiId: String, apiHash: String): Boolean = setApiCredentials(apiId, apiHash)

    fun downloadFile(url: String, outputPath: String, accountId: Long): DownloadResult {
        if (runtimeHandle == 0L) {
            return DownloadResult(false, null, "Runtime not initialized")
        }

        val json = download(runtimeHandle, url, outputPath, accountId)
        return try {
            val obj = JSONObject(json)
            if (obj.getBoolean("success")) {
                DownloadResult(true, obj.getString("path"), null)
            } else {
                DownloadResult(false, null, obj.getString("error"))
            }
        } catch (e: Exception) {
            DownloadResult(false, null, "Failed to parse result: ${e.message}")
        }
    }

    data class DownloadResult(
        val success: Boolean,
        val path: String?,
        val error: String?
    )
}
```

### 3. 使用示例

```kotlin
class MainActivity : AppCompatActivity() {
    private lateinit var tdlr: TdlrNative

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        tdlr = TdlrNative()

        val sessionDir = filesDir.resolve("sessions").absolutePath
        tdlr.configureSessionDir(sessionDir)

        tdlr.configureApi("YOUR_API_ID", "YOUR_API_HASH")

        if (!tdlr.init()) {
            Log.e("TDLR", "Failed to initialize runtime")
            return
        }

        Log.i("TDLR", "Version: ${tdlr.version()}")
    }

    fun downloadFromTelegram(url: String, accountId: Long) {
        val outputDir = getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS)?.absolutePath
            ?: return

        lifecycleScope.launch(Dispatchers.IO) {
            val result = tdlr.downloadFile(url, outputDir, accountId)

            withContext(Dispatchers.Main) {
                if (result.success) {
                    Toast.makeText(this@MainActivity, "Downloaded: ${result.path}", Toast.LENGTH_SHORT).show()
                } else {
                    Toast.makeText(this@MainActivity, "Error: ${result.error}", Toast.LENGTH_SHORT).show()
                }
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        tdlr.destroy()
    }
}
```

## Session 管理

Android 端需要先在桌面端登录并导出 session 文件，然后复制到 Android 设备。

Session 文件位置：

- 桌面端: `sessions/<account_id>.session`
- Android: `<app_files_dir>/sessions/<account_id>.session`

## 注意事项

1. **线程安全**: 所有 JNI 调用都应在后台线程执行
2. **生命周期**: 确保在 Activity 或 Fragment 销毁时调用 `destroy()`
3. **权限**: 需要网络权限和存储权限
4. **ProGuard**: 如果使用混淆，需要保留 native 方法

```proguard
-keep class com.tdlr.TdlrNative { *; }
```

## 最小 SDK 版本

- `minSdkVersion`: `21` (Android 5.0)
