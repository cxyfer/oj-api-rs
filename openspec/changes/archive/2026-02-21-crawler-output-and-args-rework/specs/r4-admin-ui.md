# Spec: R4 — Admin UI 動態參數

## Requirements

### R4.1: CRAWLER_CONFIG JS 物件
- 在 `admin.js` 定義各 source 的 flag 配置
- 每項包含：flag, label, type (checkbox/date/text/number/month-year)
- `--data-dir` 和 `--db-path` 不出現在配置中

### R4.2: 動態渲染
- 切換 source 按鈕時，清空 args 容器並重新渲染
- Checkbox grid 佈局，帶值 flag 旁顯示 input（disabled 直到勾選）
- `--monthly` 特殊處理：year + month 兩個 input

### R4.3: Args 組合
- 遍歷勾選的 checkbox，收集 flag + value 組成 args 陣列
- 帶值但 value 為空時前端阻擋送出（toast 提示）

### R4.4: 輸出 Modal
- History 表格新增 "Logs" 欄位，含 "View" 按鈕
- 僅 Completed/Failed/TimedOut 狀態顯示按鈕
- 點擊 fetch `GET /admin/api/crawlers/{job_id}/output`
- Modal 內 `<pre>` 分 tab 顯示 stdout/stderr
- stderr 使用紅色字體
- Loading 中按鈕顯示 spinner

### R4.5: 向下相容
- 原有 source 按鈕行為不變
- 觸發 API 請求格式不變（`{source, args}`）

## PBT Properties

### P4.1: Source 切換清空
- **INVARIANT**: 切換 source 後，先前的 checkbox 選擇被清空
- **Falsification**: 勾選 LeetCode --daily → 切換到 AtCoder → 驗證無殘留勾選

### P4.2: 隱藏 flag 不渲染
- **INVARIANT**: `--data-dir` 和 `--db-path` 永不出現在 DOM 中
- **Falsification**: 遍歷所有 source，檢查 DOM 中無 data-dir/db-path
