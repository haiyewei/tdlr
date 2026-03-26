# Android Integration Guide

This document explains how to integrate `tdlr` into an Android application.

## Prerequisites

1. **Android NDK** (`r21` or newer)
2. **Rust** (`1.70+`)
3. **Android target toolchains**

```bash
# Install Android Rust targets
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android
rustup target add i686-linux-android
```

## Build

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

After the build, the libraries are available in `target/android/jniLibs/`:

```text
target/android/jniLibs/
├── arm64-v8a/libtdlr.so
├── armeabi-v7a/libtdlr.so
├── x86_64/libtdlr.so
└── x86/libtdlr.so
```

## Android project integration

### 1. Copy library files

Copy the `jniLibs` directory into `app/src/main/` in your Android project.

### 2. Create a JNI wrapper class

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

### 3. Usage example

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

## Session management

On Android, log in on desktop first and copy the exported session files to the device.

Session file locations:

- Desktop: `sessions/<account_id>.session`
- Android: `<app_files_dir>/sessions/<account_id>.session`

## Notes

1. **Thread safety**: Run JNI calls on background threads.
2. **Lifecycle**: Call `destroy()` when the Activity or Fragment is destroyed.
3. **Permissions**: Network and storage permissions are required.
4. **ProGuard**: Keep native methods if obfuscation is enabled.

```proguard
-keep class com.tdlr.TdlrNative { *; }
```

## Minimum SDK

- `minSdkVersion`: `21` (Android 5.0)
