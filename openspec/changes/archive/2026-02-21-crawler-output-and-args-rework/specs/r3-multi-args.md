# Spec: R3 — 多參數組合

## Requirements

### R3.1: validate_args 支援多 flag
- 輸入 `args` 可含多個 flag + 各自的 value
- 逐 token 走訪，每個 flag 根據 arity 消耗後續 N 個 value token
- 所有 flag 獨立驗證，最終原樣透傳給腳本

### R3.2: 重複 flag 拒絕
- 同一 flag 出現兩次 → HTTP 400
- 錯誤訊息明確指出重複的 flag 名稱

### R3.3: 空 args 合法
- `args = []` → `Ok(vec![])`（腳本自行處理無參數情況）

### R3.4: 無衝突偵測
- 不做參數間的互斥/衝突偵測
- 如 `--fetch-all` + `--contest abc123` 同時出現由腳本 argparse 處理

## PBT Properties

### P3.1: 組合順序無關
- **INVARIANT**: `validate_args(s, [A, B])` 和 `validate_args(s, [B, A])` 同為 Ok 或同為 Err
- **Falsification**: 隨機排列合法 flag 組合，驗證結果一致性

### P3.2: 重複冪等拒絕
- **INVARIANT**: `validate_args(s, [F, F]).is_err()` 對所有 boolean flag F
- **Falsification**: 遍歷每個 boolean flag 構造重複輸入

### P3.3: 空輸入恆成功
- **INVARIANT**: `validate_args(any_source, []).is_ok()`
