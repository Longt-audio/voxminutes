# ASR / 翻译回归评测脚本

这几个脚本是 2026-09-22 排查「译文重复 / 流式重复出句 / 离线分片丢失」时写的，
留在这里方便以后回归（它们打的是**线上** `api.voxmin.top`，会新建测试授权码并消耗少量积分）。

## 用法

```bash
# 1) 翻译模型 × 语言方向 矩阵：验证「译文不会混入前几段内容」
node scripts/asr_eval/model_matrix.mjs
#    需要先有测试授权码：脚本会自己注册一个并存到 /tmp/asr_cmp/key.txt

# 2) 单模型细看：旧提示词 vs 新提示词，逐句打印（默认 qwen-flash）
AB_MODEL=qwen-flash node scripts/asr_eval/prompt_ab.mjs

# 3) 本地 Hy-MT2（llama-helper 直连，不经过 App）
node scripts/asr_eval/hymt2_ab.mjs

# 4) 流式 ASR 的「累计串一致性」不变量（豆包等桥接必须满足）
node scripts/asr_eval/ws_invariant.mjs doubao-asr-streaming-2.0 45
```

## 每个脚本在验什么

| 脚本 | 断言 |
|---|---|
| `model_matrix.mjs` | 每个翻译模型 × {en→zh, zh→en, en→ja}：第 N 段译文里**不得出现第 1..N-1 段的特征词**，也不得出现 `来源：/Source:/译文：/<source>` 之类回声 |
| `prompt_ab.mjs` | 同一段语料分别用旧提示词（夹带前文）与新提示词（`<source>` 标签）翻译，对比混入段数 |
| `hymt2_ab.mjs` | 本地 Hy-MT2 走同一套提示词，同上对比（直接 spawn llama-helper，JSON over stdin） |
| `ws_invariant.mjs` | 流式 ASR：每个 `final` 必须以前一个 `final` 为前缀，每条 `partial` 必须以上一个 `final` 为前缀（否则客户端会回滚已提交文本 → 重复出句） |

## 2026-09-22 基线结果

- `model_matrix.mjs`：新提示词 **18/18 组无问题**；旧提示词下 qwen-flash 混入前文、
  mimo-v2.5 空输出。
- `hymt2_ab.mjs`：旧提示词第 3 段混入前两段（80 字 vs 23 字）；新提示词干净。
- `ws_invariant.mjs doubao-asr-streaming-2.0 45`：2 finals / 25 partials / **violations=0**。

> 注：脚本里的语料与特征词是为「前后段可区分」专门挑的；换语料时同步改 `markers`。
