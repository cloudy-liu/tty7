# Paperglow 阅读验收

这份样例覆盖中文、English、数字 0123456789 和混排。**粗体**、*斜体*、~~删除线~~、`inline_code()` 应清楚可读。
这一行是软换行，应接在上一行。

这一行之后是硬换行。\
这里必须换行。<br>这里是 HTML 换行。

[跳到代码示例](#代码示例) · [第二个同名标题](#重复标题-1) · [主题说明](../customization/markdown-themes.mdx) · [外部链接](https://github.com/cloudy-liu/tty7)

## 二级标题

段落应在窄分栏里换行，宽阅读区里显示居中的纸张。切换应用的浅色和深色后，所有阅读元素与代码高亮应同步变化。

### 三级标题

#### 四级标题

##### 五级标题

###### 六级标题

- 一级列表
  - 二级列表
    - 三级列表
- 普通条目里的 `代码` 和 [链接](#二级标题)

1. 有序列表
2. 第二项

- [x] 已完成任务（只读）
- [ ] 未完成任务（只读）

> 普通引用：正文、[链接](#二级标题) 和 `code`。
>
> > 嵌套引用应该使用第二层颜色。

> [!NOTE]
> 普通说明。

> [!TIP]
> 一个有用的提示。

> [!IMPORTANT]
> 需要阅读的重点。

> [!WARNING]
> 请检查相关条件。

> [!CAUTION]
> 需要格外留意的内容。

> 普通正文中出现 [!NOTE] 不能成为提示块。

> \[!WARNING]
> 转义后的标记必须保持普通引用。

## 代码示例

```rust
// 中文注释与语法颜色
fn greeting(name: &str) -> String {
    let count = 42;
    format!("Hello, {name}! {count}")
}
```

```json
{"markdown_theme": "paperglow", "enabled": true, "values": [1, 2, 3]}
```

```unknown-language
Unknown syntax remains readable.
This intentionally long line should scroll inside the code block: 0000000000 1111111111 2222222222 3333333333 4444444444 5555555555 6666666666 7777777777 8888888888 9999999999 AAAAAAAAAA BBBBBBBBBB CCCCCCCCCC
```

| 左对齐 | 居中 | 右对齐 | 很宽的内容 |
|:---|:---:|---:|---|
| 中文 | `center` | 12345 | 一段很宽的内容用于测试表格独立横向滚动，不应该让整个文档向右溢出，也不应该截断单元格文本 |
| English | ✓ | 67890 | `a_very_long_symbol_name_that_must_remain_visible_when_scrolling_the_table` |

---

<div align="center">
<p><strong>居中 HTML</strong> 与 <em>强调</em></p>
<p>按 <kbd>Ctrl</kbd> + <kbd>C</kbd> 复制。<br>这段内容保留换行。</p>
<a href="https://github.com/cloudy-liu/tty7"><img src="../../assets/app-icon.svg" alt="tty7 本地 SVG 图标" width="88" height="88"></a>
</div>

徽章与文字 [![CI](https://github.com/cloudy-liu/tty7/actions/workflows/ci.yml/badge.svg)](https://github.com/cloudy-liu/tty7/actions/workflows/ci.yml) 应在一行。

![相对图片，按可用宽度缩小](../../assets/hero.webp)

![缺失图片应保留这段替代文本](missing-preview-image.png)

## 重复标题

第一次出现。

## 重复标题

第二次出现，[回到顶部](#)。
