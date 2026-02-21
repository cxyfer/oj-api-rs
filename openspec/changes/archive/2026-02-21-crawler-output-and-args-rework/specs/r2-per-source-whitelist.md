# Spec: R2 — Per-source 參數白名單

## Requirements

### R2.1: ArgSpec 資料結構
```rust
struct ArgSpec {
    flag: &'static str,
    arity: u8,
    value_type: ValueType,
    ui_exposed: bool,
}
```

### R2.2: 各 source 白名單常數表
- `LEETCODE_ARGS: &[ArgSpec]` — 8 entries
- `ATCODER_ARGS: &[ArgSpec]` — 12 entries
- `CODEFORCES_ARGS: &[ArgSpec]` — 13 entries
- 對應關係見 design.md AD3 表格

### R2.3: CrawlerSource enum
```rust
enum CrawlerSource {
    LeetCode,
    AtCoder,
    Codeforces,
}
```
- `CrawlerSource::script_name() -> &str` 回傳腳本檔名
- `CrawlerSource::arg_specs() -> &[ArgSpec]` 回傳白名單
- `CrawlerSource::parse(s: &str) -> Result<Self, String>` 從字串解析

### R2.4: 移除 CrawlerAction enum
- 刪除 `src/models.rs` 中的 `CrawlerAction` enum 及其 impl
- Admin handler 和 daily fallback 改用 `validate_args()`

### R2.5: 值類型驗證
| ValueType | 驗證規則 |
|-----------|----------|
| None | 不消耗 value |
| Date | `YYYY-MM-DD` regex + `chrono::NaiveDate` |
| Int | 正整數 (u64) |
| Float | 正 f64, finite |
| String | 非空，相對路徑（不含 `..`、不以 `/` 開頭） |
| YearMonth | arity=2: year(2000-2100) + month(1-12) |

### R2.6: 未知參數拒絕
- 不在白名單中的 `--flag` → HTTP 400 明確指出無效參數名
- 孤立 value（不以 `--` 開頭且非前一 flag 的值）→ HTTP 400

## PBT Properties

### P2.1: 白名單完備性
- **INVARIANT**: 各腳本 argparse 定義的每個 flag 都對應一個 ArgSpec entry
- **Falsification**: 解析腳本 argparse，提取 flag 列表，與白名單比對

### P2.2: 合法參數透傳
- **INVARIANT**: `validate_args(source, valid_args) == Ok(valid_args)`（原樣透傳）
- **Falsification**: 對每個 source 的每個 flag 構造合法輸入，驗證輸出等於輸入

### P2.3: 非法參數拒絕
- **INVARIANT**: `validate_args(source, ["--nonexistent"]) == Err(_)`
- **Falsification**: 隨機生成不在白名單中的 flag 字串

### P2.4: 跨 source 隔離
- **INVARIANT**: LeetCode 專有 flag（如 `--daily`）對 AtCoder/Codeforces 回 Err
- **Falsification**: 用 source A 的專有 flag 呼叫 source B 的 validate_args
