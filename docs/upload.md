# Upload 命令

将文件上传到 Telegram。

## 基本用法

```bash
tdlr upload -p <路径> [选项]
```

## 参数

| 参数 | 短参数 | 说明 |
|------|--------|------|
| `--path` | `-p` | 文件或目录路径（必需，可多个） |
| `--chat` | `-c` | 目标聊天 ID 或用户名（默认：Saved Messages） |
| `--topic` | | 话题 ID（用于论坛群组，需配合 --chat） |
| `--include` | `-i` | 仅包含指定扩展名（如：jpg,png,mp4） |
| `--exclude` | `-e` | 排除指定扩展名（如：tmp,log） |
| `--caption` | | 文件说明（HTML 格式，直接发送） |
| `--to` | | 目标表达式（与 --chat/--topic 冲突） |
| `--account` | `-a` | 指定账户 ID（可多个） |
| `--all-accounts` | | 使用所有账户 |
| `--group` | | 作为媒体组发送（仅照片/视频，最多10个） |
| `--rm` | | 上传后删除源文件 |

## Chat ID 格式

`-c` 参数支持多种格式，会自动通过 Telegram API 解析正确的类型：

| 格式 | 说明 |
|------|------|
| 空 / `me` / `self` | Saved Messages |
| `@username` | 用户名（用户/群组/频道） |
| `username` | 用户名（不带@） |
| 数字 ID | 自动从对话列表中查找匹配的用户/群组/频道 |

无需手动区分用户 ID、群组 ID 或频道 ID，程序会自动识别。

## 示例

### 基础上传

```bash
# 上传单个文件到 Saved Messages
tdlr upload -p ./file.txt

# 上传目录到指定用户
tdlr upload -p ./photos -c @username

# 上传到群组（使用群组 ID）
tdlr upload -p ./photos -c 1234567890

# 上传到群组的指定话题
tdlr upload -p ./files -c 1234567890 --topic 123
```

### 文件过滤

```bash
# 仅上传图片
tdlr upload -p ./media -i jpg,png,gif

# 排除临时文件
tdlr upload -p ./project -e tmp,log,bak

# 组合使用
tdlr upload -p ./folder -i mp4,mkv -e sample
```

### 媒体组上传

```bash
# 将照片作为相册发送
tdlr upload -p ./photos -c -1001234567890 --group

# 媒体组 + 话题
tdlr upload -p ./album -c -1001234567890 --topic 5 --group
```

### 多账户

```bash
# 使用指定账户
tdlr upload -p ./file.txt -a 123456789

# 使用多个账户
tdlr upload -p ./file.txt -a 123456789 -a 987654321

# 使用所有账户
tdlr upload -p ./file.txt --all-accounts
```

### 上传后删除

```bash
tdlr upload -p ./temp -c -1001234567890 --rm
```

## 表达式引擎

`--caption` 和 `--to` 参数支持表达式。

### 变量

#### 文件信息
| 变量 | 说明 |
|------|------|
| `name` | 文件名（含扩展名） |
| `stem` | 文件名（不含扩展名） |
| `ext` | 扩展名（小写） |
| `mime` | MIME 类型 |
| `type` | 文件类型：image/video/audio/document/archive/text/code/other |
| `path` | 完整路径 |
| `dir` | 父目录名 |
| `depth` | 目录深度 |

#### 文件大小
| 变量 | 说明 |
|------|------|
| `size` | 字节数 |
| `size_kb` | KB |
| `size_mb` | MB |
| `size_gb` | GB |
| `size_str` | 可读格式（如 "1.5 MB"） |

#### 日期时间
| 变量 | 说明 |
|------|------|
| `date` | 日期 YYYY-MM-DD |
| `time` | 时间 HH:MM:SS |
| `datetime` | 日期时间 |
| `year` / `month` / `day` | 年/月/日 |
| `hour` / `minute` | 时/分 |
| `weekday` | 星期（Mon/Tue/...） |

#### 类型判断
| 变量 | 说明 |
|------|------|
| `is_image` | 是否图片 |
| `is_video` | 是否视频 |
| `is_audio` | 是否音频 |
| `is_document` | 是否文档 |
| `is_archive` | 是否压缩包 |
| `is_text` | 是否文本 |
| `is_code` | 是否代码 |
| `is_media` | 是否媒体（图片/视频/音频） |

#### 上传上下文
| 变量 | 说明 |
|------|------|
| `index` | 当前索引（从0开始） |
| `num` | 当前序号（从1开始） |
| `total` | 总文件数 |

#### 常量
| 常量 | 值 |
|------|------|
| `KB` | 1024 |
| `MB` | 1024 * 1024 |
| `GB` | 1024 * 1024 * 1024 |

### Caption

直接传递 HTML 格式的说明文字，不做模板替换：

```bash
# 简单文本
--caption "这是文件说明"

# HTML 格式
--caption "<b>重要文件</b>"
--caption "<code>备份文件</code>"
```

### 路由表达式 (--to)

根据文件属性动态选择目标：

```bash
# 按类型路由
--to 'if(is_video, "@videos", if(is_image, "@photos", "me"))'

# 按扩展名路由
--to 'if(ext == "mp4", "-1001111111111", "-1002222222222")'

# 按大小路由
--to 'if(size > 100 * MB, "@large_files", "@small_files")'

# 按目录路由
--to 'if(dir == "photos", "@photos", if(dir == "videos", "@videos", "me"))'

# 组合条件
--to 'if(is_media && size > 50 * MB, "@large_media", "@media")'
```

### 内置函数

```
str::len(s)              # 字符串长度
str::contains(s, sub)    # 包含子串
str::starts_with(s, p)   # 前缀匹配
str::ends_with(s, p)     # 后缀匹配
str::to_lowercase(s)     # 转小写
str::to_uppercase(s)     # 转大写
str::trim(s)             # 去空白
str::from(v)             # 转字符串
str::substring(s, i, n)  # 子串
str::replace(s, a, b)    # 替换
str::regex_matches(s, p) # 正则匹配
if(cond, then, else)     # 条件
min(a, b) / max(a, b)    # 最小/最大
floor(x) / ceil(x)       # 取整
```

## 完整示例

```bash
# 将 photos 目录的图片上传到 @my_photos 频道
tdlr upload -p ./photos -i jpg,png -c @my_photos \
  --caption "<b>照片备份</b>"

# 根据文件类型自动路由到不同群组
tdlr upload -p ./media \
  --to 'if(is_video, "-1001111111111", if(is_image, "-1002222222222", "me"))'

# 上传视频到群组话题，作为媒体组，完成后删除源文件
tdlr upload -p ./videos -c -1001234567890 --topic 10 --group --rm

# 使用所有账户上传大文件
tdlr upload -p ./large_file.zip --all-accounts -c -1001234567890
```
