# VoxMinutes Privacy Policy

**Effective date: 2026-09-30**
**Operator: VoxMinutes Developer**
**Contact: voxmin@qq.com**
**Address: available on request via the contact email above**

---

## 1. Introduction

VoxMinutes ("the App") is a meeting-notes tool that runs on your own computer. It can record your
microphone and system audio, transcribe speech, translate it, and generate meeting summaries.

We know meeting content is often sensitive, so this policy explains **item by item** what stays on
your computer, what is uploaded when you use a particular feature, to whom, and for how long.

**Please note: the App does not require an account.** We do not ask for your name, phone number or
email address. When you first use the remote service, we create an anonymous account for you based
only on your operating system's device identifier, so that we can track your credit balance.

By using the App you confirm that you have read and understood this policy.

---

## 2. What we collect

### 2.1 Data that never leaves your computer

| Data | Where it is stored |
|---|---|
| Audio recordings | The folder you choose in Settings (default: `~/recordings`) |
| Transcripts, translations, summaries | Same meeting folder (`transcripts.json`, `metadata.json`) |
| Local database (meetings, segments, settings) | App data directory (macOS: `~/Library/Application Support/com.voxminutes.app`; Windows: `%LOCALAPPDATA%\com.voxminutes.app`) |
| Logs | `logs/` next to the installed program, or `%LOCALAPPDATA%\VoxMinutes\logs` on Windows if that folder is not writable |
| Local AI models | Your models directory (customizable) |

> **Uninstalling the App does not delete the data above.** Delete the recordings folder and the app
> data directory manually if you want them gone.

### 2.2 Data you give us voluntarily

- **Feedback**: the message text, an optional screenshot, an optional contact detail, and an
  optional diagnostic log.
- **License key**: the key you obtained from us or from our card shop.

> **Log redaction**: if you tick "attach diagnostic log", we automatically strip license keys,
> API keys, bearer tokens and your OS username before upload, and only take the tail of recent log
> files (max 160 KB per file, max 2 files; in time-window mode 80 KB per file).
> **Logs never contain your audio or transcripts.**

### 2.3 Data generated when you use remote features

Only when you **explicitly select a "remote" model** or use a feature that needs the internet:

| Data | When | Notes |
|---|---|---|
| Device identifier | When you claim free credits | Provided by the OS (MachineGuid on Windows, IOPlatformUUID on macOS, machine-id on Linux). Used to bind your anonymous account and prevent duplicate claims |
| License key | On every remote call | Used for authentication |
| **Audio** | Remote speech recognition | Streaming: short segments continuously uploaded. Offline/batch: the whole recording. See 2.4 |
| **Text** | Remote translation / summarization | Sentences to translate and transcript text to summarize are forwarded through our server to the upstream AI provider |
| Usage and billing records | On every remote call | Model used, audio seconds or tokens consumed, timestamp, and a task-session id used to group one meeting's calls together |
| IP address | On remote calls / credit claims | Used for abuse prevention and rate limiting; stored in server access logs |

### 2.4 About audio, specifically

- **Local models**: audio never leaves your computer.
- **Remote streaming recognition**: audio is split into short segments (usually a few seconds) and
  streamed to our server, which forwards it to the upstream provider and returns text in real time.
- **Remote batch recognition**: the whole recording is uploaded. If the upstream provider requires a
  publicly downloadable URL, the audio is temporarily hosted by our server and **deleted
  automatically within 2 hours**. Audio may be transcoded to 16 kHz mono MP3 first to reduce
  transfer size (this does not meaningfully affect accuracy).
- We **do not** use your audio to train models and **do not** use it for any purpose not described
  in this policy.

### 2.5 What we do NOT collect

- Name, phone number, email address or government ID (no registration is required)
- Precise location
- Contacts, call logs or SMS
- Clipboard contents
- Other files on your computer (the App only accesses folders you explicitly select)
- In-app behavioural analytics — **the App contains no third-party analytics SDK**

### 2.6 If you never use remote features

If you only use local models and never claim credits or submit feedback, we receive **no data about
you** other than the IP address recorded in ordinary server logs when the App checks for new
versions and announcements.

---

## 3. How we use the information

1. To provide remote speech recognition, translation, summarization and speech synthesis;
2. To calculate and display your credit balance and usage history;
3. To prevent abuse (duplicate free-credit claims, automated scraping, license-key theft);
4. To debug and improve the product, based only on feedback and redacted logs you submit;
5. To comply with legal obligations.

**We do not** sell your personal information, use your audio or transcripts for advertising, or use
your content to train models without your consent.

---

## 4. Sharing and third parties

### 4.1 Upstream AI providers

Depending on the model you choose, audio or text is forwarded through our server to one of:

| Provider | Purpose | Location |
|---|---|---|
| Alibaba Cloud Bailian (DashScope) | ASR, translation, summarization, TTS | Mainland China |
| Volcengine (Doubao) | ASR | Mainland China |
| Xiaomi MiMo | ASR, translation, summarization | Mainland China |
| Deepgram | ASR | United States |
| DeepSeek / Moonshot and similar LLM services | Translation, summarization | Mainland China |

Each provider processes the data it receives under its own privacy policy.

### 4.2 Your own "Custom API"

The App lets you configure your own OpenAI-compatible or Anthropic-compatible endpoint. In that
case data goes **directly from your computer to the endpoint you configured**, never through our
server. We cannot access it and are not responsible for how it is handled.

### 4.3 Payments and card shop

Purchases are made on our card shop. The shop and the payment channels collect the information
needed to complete the transaction and deliver the card (typically an email address and order
details), governed by their own privacy policies: Dujiaoka (card shop), Xunhupay (Alipay / WeChat
Pay), PayPal. We never receive your card number or payment credentials.

### 4.4 Legal requirements

We may disclose information where required by law or by a lawful request from a competent authority.

---

## 5. International transfers

Our server is located in **Hong Kong**, and upstream AI providers are located in **mainland China**
and the **United States**. When you use remote features, your audio segments or text may therefore
be transferred outside your country of residence. Transfers are encrypted with HTTPS/TLS.

If you do not agree to such transfers, use local models — local features are fully offline.

---

## 6. Retention

| Data | Retention |
|---|---|
| Temporarily hosted audio | At most 2 hours, then deleted automatically |
| Server access logs (incl. IP) | 90 days |
| Usage, billing and credit records | 24 months (needed for reconciliation and legal obligations) |
| Feedback and diagnostic logs | Deleted 12 months after resolution |
| Data on your device | Controlled by you; we never touch, delete or back it up |

---

## 7. Your rights

- **Access and export**: recordings, transcripts and summaries live on your computer and can be
  viewed, exported or deleted at any time. Your balance and usage history are in the account page.
- **Deletion**: delete the meeting folder to remove recordings and text. To delete the server-side
  account and records, contact voxmin@qq.com; we will act within
  15 business days after verification.
- **Withdraw consent**: turn off "Enable remote service" in Settings at any time. No further uploads
  will occur.
- **Complaint**: you may lodge a complaint with your local data protection authority.

**For users in the EEA / UK / Switzerland**: our legal basis for processing is the performance of
our contract with you (providing the remote features you requested) and our legitimate interest in
preventing abuse. You additionally have the right to data portability and to object to processing.
Our server is outside the EEA/UK; by using remote features you consent to the transfer described in
section 5.

---

## 8. Children

The App is designed for adults. Users **under 14** must have verifiable consent from a parent or
guardian. If you believe we have collected a child's personal information without such consent,
contact us and we will delete it.

---

## 9. Security

- HTTPS/TLS for all client–server and server–provider traffic;
- License keys stored in the OS credential manager (macOS Keychain / Windows Credential Manager),
  never in plain-text config files;
- Diagnostic logs redacted before upload;
- Production database access limited to the operator;
- Temporary audio files cleaned up automatically.

No technical measure is perfectly secure. Keep your license key safe, avoid using the App on
untrusted devices, and make sure meeting participants are informed about recording.

---

## 10. Changes to this policy

We may update this policy. We will notify you in the App or on our website and update the effective
date above. Material changes will be announced more prominently (for example via an in-app notice).
Continued use constitutes acceptance.

---

## 11. Contact

voxmin@qq.com · VoxMinutes Developer
We normally respond within 15 business days.
