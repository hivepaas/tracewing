# tracewing in traditional Chinese. Each message is found by its English, the
# comment above it; scripts/i18n.py keeps this file in step with the code.

# … {count} line {closer} | … {count} lines {closer}
t2ba67348 = … { $count } 行 { $closer }

# … {size} more
ta8dc6eab = … 還有 { $size }

# {size} {format} · {count} line of JSON | {size} {format} · {count} lines of JSON
t0a4220dc = { $size } { $format } · { $count } 行 JSON

# Unfold all
t3849d44e = 全部展開

# {size} · {count} line | {size} · {count} lines
t51332550 =
    { $n ->
       *[other] { $size } · { $count } 行
    }

# {size} · reading {percent}%
t49fe2cb3 = { $size } · 正在讀取 { $percent }%

# Appearance: {now}. Click for {next}.
t332cfa6d = 外觀：{ $now }。按一下切換為 { $next }。

# Method
t98a2c750 = 方法

# Status
t005ef20f = 狀態

# App
t81f51acc = App

# Host
t302af18f = 主機

# Path
teb66e456 = 路徑

# Operation
t218e02a6 = 操作

# Size
ta6478e7c = 大小

# Time
tdfe4e404 = 時間

# Number
t61e9eac0 = 序號

# "{method}" is not a method
tacdcf5cb = 「{ $method }」不是有效的方法

# its request body was not kept whole, so it cannot be sent again
t1f9e5368 = 其請求本文未完整保留，因此無法重新傳送

# name: value, one a line
t4df37bea = 名稱: 值，每行一個

# No body
tfa4c2902 = 無本文

# No flow is chosen
t550b0a98 = 未選取流量紀錄

# This flow: {e}
t296762f8 = 此流量紀錄：{ $e }

# Request
t82454152 = 請求

# Headers
t334f5989 = 標頭

# Body
t428a1095 = 本文

# Sending…
te946e7bf = 正在傳送…

# Send
tf28e14cf = 傳送

# New
t8b242db1 = 新增

# … {count} more bytes: the flow in Traffic has them all.
tf43ea06c = … 還有 { $count } 位元組：「流量」中的該筆流量紀錄包含完整內容。

# Response
t55a982fe = 回應

# Compose
t18049f93 = 撰寫

# A request of your own, or one sent again. It goes through tracewing, so rules apply and it shows in Traffic too.
t7a9cf113 = 自行撰寫的請求，或重新傳送的請求。它會經過 tracewing，因此規則同樣適用，也會顯示在「流量」中。

# {version} both sides
t961a5ea0 = 兩端皆為 { $version }

# {client} to the proxy, {server} to the server
t3d237646 = 與代理伺服器之間 { $client }，與伺服器之間 { $server }

# Queued
t9e437c74 = 佇列

# Connect
tdb89a899 = 連線

# Waiting
tc9450e48 = 等待回應

# Download
t6964bb19 = 下載

# No file picker: {e}
t6935fbd3 = 無法開啟檔案選擇器：{ $e }

# Writing the HAR…
t725a0e89 = 正在寫入 HAR…

# {n} flow written to {path} | {n} flows written to {path}
t643f96ee =
    { $n ->
       *[other] 已將 { $n } 筆流量紀錄寫入 { $path }
    }

# The HAR was not written: {e}
tef8ec569 = 無法寫入 HAR：{ $e }

# Open
t538b10e9 = 開啟

# Reading {path}…
tc0e7c8f4 = 正在讀取 { $path }…

# , {n} entry left out | , {n} entries left out
td848c31b =
    { $n ->
       *[other] ，略過 { $n } 個項目
    }

# {n} flow read from a {format} | {n} flows read from a {format}
t610d7810 =
    { $n ->
       *[other] 已從 { $format } 讀取 { $n } 筆流量紀錄
    }

# The file was not read: {e}
t7f49510f = 無法讀取檔案：{ $e }

# it is no longer kept
ta7b4c380 = 已不再保留

# {size} saved to {path}
tc6932467 = 已將 { $size } 儲存至 { $path }

# The body was not saved: {e}
t189900b4 = 無法儲存本文：{ $e }

# Fiddler session
t9f5c89a3 = Fiddler 工作階段

# Charles session
tb4f56f2b = Charles 工作階段

# All
t75afe364 = 全部

# Errors
te9ed60a6 = 錯誤

# Media
td10fcaa3 = 媒體

# Rule: {rule}
teca2074f = 規則：{ $rule }

# Events
t003420a4 = 事件

# JSON lines
te8ce2db0 = JSON 行

# Not decrypted
t8090a1ba = 未解密

# open
td35ec4c9 = 進行中

# Reading the request…
t51c96267 = 正在讀取請求…

# The request's body was not kept, so its query cannot be shown.
tba9e5628 = 請求本文未保存，因此無法顯示其查詢。

# An operation without a name
t38c4d30b = 未命名的操作

# {n} of {count}: {name}
t53045460 = 第 { $n }/{ $count } 個：{ $name }

# Query
t9e89e783 = 查詢

# A persisted query: the server keeps its text, and the request sends only its hash.
tf23e3c2a = 持久化查詢：伺服器保存其文字，請求只傳送其雜湊。

# Variables
t7996f062 = 變數

# Cut
t58e25863 = 剪下

# Copy
t658f3664 = 複製

# Paste
tb34a0288 = 貼上

# Select all
t6af21072 = 全選

# This flow is gone.
t3343999e = 此流量紀錄已不存在。

# This host is not decrypted, so what it sends cannot be seen.
tf85fe84f = 此主機未解密，因此無法檢視其傳送的內容。

# Still coming: it shows once it ends.
t42be95fb = 仍在接收：結束後顯示。

# No body.
t31e4d044 = 無本文。

# This body was not kept: the disk was nearly full, or too much was waiting for it.
t884aad28 = 此本文未保留：磁碟空間即將用盡，或待寫入的內容過多。

# This body was not kept.
te46616b5 = 此本文未保留。

# This body is gone: it was dropped with the oldest flows, or cannot be read.
tf9a7ab60 = 此本文已不存在：已隨最舊的流量紀錄一併捨棄，或無法讀取。

# Cannot open this body: {e}
t620a039b = 無法開啟此本文：{ $e }

# Inspector
t4def410e = 檢查器

# Choose a flow to see it here
tc7be91e8 = 選取一筆流量紀錄即可在此檢視

# Not decrypted. New connections to this host go through unread.
ta05ec37a = 未解密。連往此主機的新連線將直接通過，不會被讀取。

# If an app fails with this host, it may pin its certificates.
t3cbba118 = 如果某個 App 連線此主機失敗，它可能使用了憑證綁定。

# Make a rule
t0ac0e062 = 建立規則

# Decrypt this host
tb7367e2c = 解密此主機

# Do not decrypt this host
t5fa113f6 = 不要解密此主機

# Request and response
t04e060e6 = 請求和回應

# Request and response one at a time
t70397119 = 請求和回應逐一顯示

# Request and response side by side
t3d35ce69 = 請求和回應並排顯示

# Request above the response
t83c63123 = 請求在回應上方

# Pretty
t7e09470d = 格式化

# Raw
ta689aa91 = 原始

# Picture · {size}
te97ed633 = 圖片 · { $size }

# Messages {count}
te61b841a = 訊息 { $count }

# Headers {headers}
t04b0146d = 標頭 { $headers }

# Cookies {cookies}
td58eb84b = Cookie { $cookies }

# Query {query}
t4756f9f5 = 查詢參數 { $query }

# Form {form}
tfa6e7203 = 表單 { $form }

# Connection
t09ea4ff9 = 連線

# Save body
te2c44424 = 儲存本文

# Send again
tf5910557 = 重新傳送

# Edit and send
tf837e542 = 編輯後傳送

# Copied
t8dc21305 = 已複製

# Copy as cURL
t866d1e3e = 複製為 cURL

# Copy as…
t9c1ad448 = 複製為…

# This picture cannot be drawn. Save the body to look at it.
t1e92fb27 = 無法顯示此圖片。請儲存本文後檢視。

# Opening…
tc40f2321 = 正在開啟…

# No response
t8c2c37b1 = 無回應

# Tunnel, not decrypted
tce693254 = 通道，未解密

# GraphQL: {operation}
t2d65757d = GraphQL：{ $operation }

# Over connection {id}, made before: no time went on connecting.
t84144a81 = 沿用先前建立的連線 { $id }：未花費連線時間。

# {app}, pid {pid}
t627147b4 = { $app }，pid { $pid }

# Not found: it may have ended, or belong to another user
t0efce58b = 找不到：可能已結束，或屬於其他使用者

# On another device
t2b964a37 = 在其他裝置上

# CLIENT
t6d3cd99e = 用戶端

# Device
tbdcbb413 = 裝置

# Program
tc4253b6b = 程式

# Address
t700560f3 = 位址

# SERVER
te3b27612 = 伺服器

# What this connection was is no longer kept: it went with older flows.
t1ef5a00a = 此連線的資訊已不再保留：已隨較早的流量紀錄一併清除。

# Not decrypted: tracewing passed it on without looking.
t0427c9b1 = 未解密：tracewing 未檢視內容便直接轉送。

# No connection was made.
tf348eb92 = 未建立連線。

# None: it was answered without one.
tac71808e = 無：未建立連線即已回應。

# {id}, given up during TLS
t0ec66a8c = { $id }，在 TLS 階段放棄

# {id}, made for this flow
t7fe32cfc = { $id }，為此流量紀錄新建

# {id}, made before and used again
ta904c078 = { $id }，沿用先前建立的連線

# Server
t6d7302f2 = 伺服器

# None: plain HTTP
t05606349 = 無：純 HTTP

# THE SERVER SHOWED NO CERTIFICATES
te476fbb8 = 伺服器未出示憑證

# WHAT THE SERVER SHOWED, ITS OWN FIRST
ta51f3107 = 伺服器出示的憑證，其自身憑證在前

# NO CERTIFICATES
te6f31dce = 無憑證

# CERTIFICATE
t1c902790 = 憑證

# {n} CERTIFICATE, THE SERVER'S FIRST | {n} CERTIFICATES, THE SERVER'S FIRST
td5295f62 =
    { $n ->
       *[other] { $n } 個憑證，伺服器自身憑證在前
    }

# tracewing would not go on with this server: {error}. If the app trusts it some other way, such as through a company CA, choose “Do not decrypt this host” and the app will check the server itself.
t13ced472 = tracewing 拒絕繼續連線此伺服器：{ $error }。如果 App 透過其他方式信任它（例如透過公司 CA），請選擇「不要解密此主機」，由 App 自行驗證伺服器。

# Cipher
t5b3bed6a = 加密套件

# none
tada7afdb = 無

# not valid until {when}
t0a14b2d8 = { $when }才生效

# expired {when}
t932538c2 = 已於 { $when }過期

# expires {when}
t679bcb51 = 將於 { $when }過期

# {names}, and {n} more | {names}, and {n} more
t4cd375ad =
    { $n ->
       *[other] { $names }，以及另外 { $n } 個
    }

# Issued by {issuer}
t15a1ce5a = 簽發者：{ $issuer }

# Valid {from} to {to} · {when}
t0e47d4f2 = 有效期間 { $from } 至 { $to } · { $when }

# Names {names}
tda75e459 = 名稱：{ $names }

# Serial {serial}
td1f0d98b = 序號：{ $serial }

# These headers were not kept.
tb8739fec = 這些標頭未保留。

# No headers.
tf3dacb5e = 沒有標頭。

# Trailers, sent after the body
t3447e9de = Trailer（在本文之後傳送）

# File {filename} · {kind} · {size}
t6dbd239f = 檔案 { $filename } · { $kind } · { $size }

# no type
t902a6ece = 無類型

# No cookies were sent.
t58e85ab2 = 未傳送 Cookie。

# No cookies were set.
tf57dbabc = 未設定 Cookie。

# JWT · {algorithm} · signature not checked
t2db33271 = JWT · { $algorithm } · 未驗證簽章

# JWT · signature not checked
t05f8b533 = JWT · 未驗證簽章

# Header
t114f9980 = 標頭

# Payload
ta0639745 = 酬載

# Expires
tc7aa0523 = 到期時間

# Issued
tb1835ecc = 簽發時間

# Not before
td10dacd9 = 生效時間

# in {amount}
tfda57c3e = { $amount }後

# {amount} ago
t5f968014 = { $amount }前

# Light
t8ad2f8cf = 淺色

# Dark
t45570065 = 深色

# System
t8f3184bc = 跟隨系統

# streamed in {count} chunk | streamed in {count} chunks
t76711a1a =
    { $n ->
       *[other] 分 { $count } 個區塊串流傳輸
    }

# stopped: {stop}
tfa48fb30 = 停止原因：{ $stop }

# {input} in · {output} out tokens
tc8bbc724 = 輸入 { $input } · 輸出 { $output } token

# Reading the call…
t33b52a3c = 正在讀取呼叫…

# Answer
tf94ea0ff = 回答

# Thinking: {thinking}
t737ece33 = 思考：{ $thinking }

# Calls {name}{id}
t017882e4 = 呼叫 { $name }{ $id }

# Nothing yet.
td9b71d16 = 尚無內容。

# {text}… and {count} more character | {text}… and {count} more characters
t6a16c074 =
    { $n ->
       *[other] { $text }… 還有 { $count } 個字元
    }

# Copy the answer
t69975ed7 = 複製回答

# Asked
t4e3af9c3 = 提問

# Tools offered: {tools}
tb871bcf4 = 提供的工具：{ $tools }

# Not kept
tba30b5fa = 未保留

# This message was not kept: too much was waiting, or the disk was nearly full.
tb888a3c3 = 此訊息未保留：待處理的內容過多，或磁碟空間即將用盡。

# This message is empty.
tdb0ec01f = 此訊息為空。

# No messages yet
t51e7624c = 尚無訊息

# {said} kept, of {size}
t3558745a = 已保留 { $said }，共 { $size }

# {what}: {text} is not a whole number
t2a8515ce = { $what }：{ $text } 不是整數

# Latency
t23a45ef9 = 延遲

# Down
ta4995035 = 下載

# Up
t42f35940 = 上傳

# Loss
t69db303a = 封包遺失

# Give a latency, a speed or a loss to slow it down by
t7cfd6404 = 請輸入要模擬的延遲、速度或封包遺失率

# Latency, ms
tefe18ab9 = 延遲（ms）

# Down, kbit/s
t08c4ae33 = 下載（kbit/s）

# Up, kbit/s
td55cfe9a = 上傳（kbit/s）

# Loss, %
tea4af439 = 封包遺失率（%）

# Only these hosts, like *.example.com; all if empty
t77a1f991 = 僅限這些主機，例如 *.example.com；留空則套用至全部

# Not kept for next time: {e}
tdce4d67c = 無法儲存供下次使用：{ $e }

# Slow network
td51ddfce = 慢速網路

# Off: traffic goes as fast as it can
tf0f34db0 = 關閉：流量以最快速度傳輸

# Custom
t9525b78e = 自訂

# Only these hosts
tfe16992b = 僅限這些主機

# Apply
tca77bbdb = 套用

# Applies from the next request. Latency is added to each round trip; the speed is a link that every slowed flow shares; a lost piece waits to be sent again, as TCP does. Hosts left alone are not slowed.
t5ee4034d = 從下一個請求開始生效。每次往返都會增加延遲；速度是所有被限速的流量紀錄共用的一條連結；遺失的封包會像 TCP 一樣等待重新傳送。未列出的主機不受限速。

# Header line {line} has no ":"
t891b076d = 標頭第 { $line } 行缺少「:」

# Header line {line} has no name
te3247b96 = 標頭第 { $line } 行缺少名稱

# URL
t7569633e = URL

# {n} bytes that are not text: they go on as they came. | {n} bytes that are not text: they go on as they came.
td6aec821 =
    { $n ->
       *[other] { $n } 位元組非文字內容：將原樣傳遞。
    }

# Nothing is paused
t8cf68b4c = 沒有暫停的項目

# "{status}" is not a status
t40a529af = 「{ $status }」不是有效的狀態碼

# REQUEST
t1f3f6dd2 = 請求

# RESPONSE
tc85f77de = 回應

# Nothing is paused. Add a Pause rule on the Rules screen to stop traffic here.
tf9ab4327 = 沒有暫停的項目。在「規則」頁面新增「暫停」規則，即可在此攔下流量。

# for {method} {url}
t093a86f6 = 針對 { $method } { $url }

# The request, before the server gets it
tc840fed6 = 伺服器收到之前的請求

# The response, before the app gets it
t6683d72a = App 收到之前的回應

# Send as it came
tb593e730 = 原樣傳送

# Drop
t108524ac = 捨棄

# Paused
tde281feb = 已暫停

# Nothing is waiting.
tea613769 = 沒有等待中的項目。

# {n} waits to be sent on. | {n} wait to be sent on.
t745634c6 =
    { $n ->
       *[other] { $n } 個項目等待繼續傳送。
    }

# Send all as they came
tdce25640 = 全部原樣傳送

# Block
tbd074002 = 封鎖

# Answer from a file
td8edab50 = 以檔案回應

# Send to another host
te3cd85d7 = 轉送到其他主機

# Set a request header
tfad4aaaa = 設定請求標頭

# Set a response header
tcbbc20d0 = 設定回應標頭

# Remove a request header
t5228dd84 = 移除請求標頭

# Remove a response header
teeb5735a = 移除回應標頭

# Set a query parameter
t73d9419b = 設定查詢參數

# Remove a query parameter
t12633da9 = 移除查詢參數

# Change the request body
t41101201 = 修改請求本文

# Change the response body
tdd259b73 = 修改回應本文

# Change the status
t4bbdfba8 = 修改狀態碼

# Pause the request
t40ecc39b = 暫停請求

# Pause the response
t18a34799 = 暫停回應

# Status to answer with, like 403
t8f3ebb6c = 要回應的狀態碼，例如 403

# Full path of a file or folder
t9fb9ae4c = 檔案或資料夾的完整路徑

# Where to, like http://localhost:3000
tdfedd004 = 目標位址，例如 http://localhost:3000

# Header's name
t80793085 = 標頭名稱

# Its value
t43968778 = 值

# Parameter's name
t313c8e35 = 參數名稱

# Text to find, or empty for the whole body
t8255fc87 = 要尋找的文字，留空則取代整個本文

# What to put in its place
t03671fd5 = 取代成的內容

# Status the client gets, like 500
t5e4d3088 = 用戶端收到的狀態碼，例如 500

# "{first}" is not a status
td43128d3 = 「{ $first }」不是有效的狀態碼

# Block with {status}
t1727e984 = 以 { $status } 封鎖

# Answer from {path}
taefc08fa = 以 { $path } 回應

# Send to {to}
t5e256873 = 轉送到 { $to }

# Request: {header}: {value}
taa230766 = 請求：{ $header }: { $value }

# Response: {header}: {value}
t30f9e2c2 = 回應：{ $header }: { $value }

# Request: no {header}
t6f0b3f5a = 請求：去掉 { $header }

# Response: no {header}
t23960a2e = 回應：去掉 { $header }

# Query: {name}={value}
t0ee6c1c0 = 查詢：{ $name }={ $value }

# Query: no {name}
tf2e7bf27 = 查詢：去掉 { $name }

# Status {status}
tad048e53 = 狀態碼 { $status }

# Request body: {replace}
te4856c36 = 請求本文：{ $replace }

# Response body: {replace}
t8600dc42 = 回應本文：{ $replace }

# Request body: {find} → {replace}
taec471a9 = 請求本文：{ $find } → { $replace }

# Response body: {find} → {replace}
tc1449525 = 回應本文：{ $find } → { $replace }

# Name
t0fe07306 = 名稱

# URL, like api.myapp.dev/v1/*
t4dd91658 = URL，例如 api.myapp.dev/v1/*

# Method, or empty for all
tb2b869ac = 方法，留空則符合全部

# {n} rule written to {path} | {n} rules written to {path}
t4a2e843a =
    { $n ->
       *[other] 已將 { $n } 條規則寫入 { $path }
    }

# The rules were not written: {e}
te7ac257f = 無法寫入規則：{ $e }

# Import
t573e0af4 = 匯入

# {path} could not be read: {e}
t32a3ce94 = 無法讀取 { $path }：{ $e }

# Nothing was imported: {e}
tb9f7257a = 未匯入任何內容：{ $e }

# Edit
tc2c76cb1 = 編輯

# Delete
t5797ea6a = 刪除

# Change this rule
t9160f393 = 修改此規則

# A new rule
t55a722a0 = 新規則

# It waits under Paused until you send it on, changed or not.
tca566a57 = 它會在「已暫停」中等待，直到你繼續傳送（修改與否皆可）。

# Save
t4d2d5d68 = 儲存

# Add
t9dc3aa14 = 新增

# Cancel
t35afca3b = 取消

# Rules
t428b2d80 = 規則

# They apply from the next request, while recording is paused too. Top to bottom: the first block, file or host wins, and every other rule that fits applies, in order. A rule that changes a body or a status changes what the server sent.
tbe44b31e = 規則從下一個請求起生效，暫停記錄時也一樣。由上而下：第一個符合的封鎖、檔案或主機規則生效，其他符合的規則依序全部生效。修改本文或狀態碼的規則，改的是伺服器送來的內容。

# Import rules…
t5edc205d = 匯入規則…

# Export rules…
t6e09ca20 = 匯出規則…

# {n} rule added | {n} rules added
tbc5e0866 =
    { $n ->
       *[other] 已新增 { $n } 條規則
    }

# {n} here already | {n} here already
t65dc1281 =
    { $n ->
       *[other] { $n } 條已存在
    }

# left out: {why}
ta81e95e2 = 已略過：{ $why }

# Copy all
t6aef5c37 = 全部複製

# Too large to copy all
t2fe8ff9f = 太大，無法全部複製

# No flows
t6dc463eb = 無流量紀錄

# Name it to keep it
t52c1a402 = 命名後即可保留

# {size} · written by another version, so it cannot be opened
t878e5c80 = { $size } · 由其他版本寫入，無法開啟

# {count} flow · {size} | {count} flows · {size}
t3f3c0eb4 =
    { $n ->
       *[other] { $count } 筆流量紀錄 · { $size }
    }

# Recording now
t3cb3e6fc = 正在錄製

# Export HAR
t4d6458cc = 匯出 HAR

# Delete for good?
ta7737f13 = 永久刪除？

# Sessions
tb69a8aec = 工作階段

# A session with a name is kept until you delete it. Of the others, the newest {kept} stay.
t6391f080 = 已命名的工作階段會一直保留，直到你將其刪除。其餘工作階段只保留最新的 { $kept } 個。

# Working…
td5507854 = 處理中…

# Open a session file
tae8622fc = 開啟工作階段檔案

# HAR, Fiddler .saz, or Charles .chlsj
td8955e6d = HAR、Fiddler .saz 或 Charles .chlsj

# Exported HAR files leave out cookies, tokens, keys and passwords, in headers, URLs and bodies
t3435c213 = 匯出的 HAR 檔案會去掉標頭、URL 和本文中的 Cookie、權杖、金鑰和密碼

# Other devices
t9e7d8362 = 其他裝置

# Let other devices use the proxy
t13e626b3 = 允許其他裝置使用代理伺服器

# Phones and other computers on your network can then send their traffic here. So can anyone else on it: leave this off on a network you do not trust.
t76098911 = 開啟後，你網路中的手機和其他電腦可以將流量傳送到這裡。但同一網路中的任何人也都可以：在不信任的網路上請保持關閉。

# Servers
tcd0cf113 = 伺服器

# Trust what this computer trusts
tfea2d91b = 信任這台電腦信任的憑證

# Servers with a certificate from an authority installed on this computer, such as a company's own, are reached as this computer's browsers reach them. Off, only the public authorities are trusted. Takes effect when tracewing starts again.
t1705bfb4 = 對於持有這台電腦上已安裝的憑證授權單位（例如公司自己的 CA）所簽發憑證的伺服器，將像這台電腦的瀏覽器一樣正常連線。關閉時，僅信任公開的憑證授權單位。重新啟動 tracewing 後生效。

# AI tools
t294d7ea4 = AI 工具

# Let AI tools see tokens and cookies
te1d26d3c = 允許 AI 工具檢視權杖和 Cookie

# AI tools read recorded traffic through tracewing's MCP server. Off, the values of Authorization, Cookie and Set-Cookie are left out of what they get. Applies at once.
tb496980f = AI 工具透過 tracewing 的 MCP 伺服器讀取已錄製的流量。關閉時，提供給它們的內容不含 Authorization、Cookie 和 Set-Cookie 的值。立即生效。

# HTTP/3
tfec0f477 = HTTP/3

# Keep apps off HTTP/3
te42733a9 = 避免 App 使用 HTTP/3

# HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around tracewing. This takes servers' offers of it out of their responses, and apps stay on HTTP/2. Flows still show the offers as servers sent them.
t2ff32f01 = HTTP/3 走 UDP，而代理伺服器無法轉送 UDP，因此使用 HTTP/3 的 App 會繞過 tracewing。此選項會從伺服器回應中移除 HTTP/3 宣告，讓 App 繼續使用 HTTP/2。流量紀錄中仍會顯示伺服器送出的原始宣告。

# Size of a session
ta20fa102 = 工作階段大小

# Past this, the oldest flows are dropped.
td2914f46 = 超過此值時，將捨棄最舊的流量紀錄。

# Room to leave on the disk
t8674a3b3 = 磁碟保留空間

# With less than this free, flows are still listed but bodies are not kept.
t5b3a31a3 = 可用空間低於此值時，流量紀錄仍會列出，但不保留本文。

# Earlier sessions to keep
td356194a = 保留的先前工作階段數

# Older ones are deleted when a new session starts.
t66556513 = 新工作階段開始時，將刪除較舊的工作階段。

# Bodies
tdb4d0409 = 本文

# Most kept of one body
t777508dd = 單一本文最大保留量

# It still reaches the client whole.
t386fee28 = 用戶端仍會收到完整內容。

# Most unpacked for viewing
tc3fe2efd = 檢視時最大解壓縮量

# Stops a small download that unpacks to many GB.
tf9c49515 = 防止小型下載內容解壓縮後膨脹至數 GB。

# Small body size
ta8996af1 = 小型本文門檻

# Smaller bodies wait in memory and share a file. Larger ones get a file of their own as they arrive.
t20c108b4 = 較小的本文會先暫存在記憶體中，並共用一個檔案。較大的本文在接收時各自寫入獨立的檔案。

# Compress plain text on disk
tee503c2e = 在磁碟上壓縮純文字

# Takes about half the room, and is skipped when traffic is too fast for it.
t6d70f53f = 約可節省一半空間；流量過大來不及壓縮時會略過。

# Compression level
td386c9ef = 壓縮等級

# 1 to 19. Higher is smaller and slower.
td42a987d = 1 到 19。數值越高，檔案越小、速度越慢。

# Search
tc646a2c9 = 搜尋

# Search URLs
t0c3e0b3d = 搜尋 URL

# Finds any part of a URL as you type. Costs little.
t3f22d666 = 輸入時即可比對 URL 的任何部分。負擔很小。

# Search bodies
t8eec0293 = 搜尋本文

# Finds words in bodies. Can take as much disk as the bodies do.
tcac3872a = 在本文中尋找文字。佔用的磁碟空間可能與本文本身相當。

# Searched part of each body
tab293b21 = 每個本文的搜尋範圍

# Words past this are not found. More takes more disk.
t0bf65f4a = 超出此範圍的文字不會被搜尋到。範圍越大，佔用的磁碟空間越多。

# Under heavy traffic
ta99888cb = 高流量時

# Memory for bodies waiting to be written
t7d6ff799 = 待寫入本文的記憶體上限

# Past this, flows are listed without their bodies. Traffic is never held back.
t5b268fb2 = 超過此值時，流量紀錄將不含本文列出。流量本身絕不會被延遲。

# Memory for large bodies waiting to be written
t07b9de08 = 待寫入大型本文的記憶體上限

# Past this, large bodies stop being kept.
tfe61a4f3 = 超過此值時，將不再保留大型本文。

# {key}: "{text}" is not a whole number
ta8606963 = { $key }：「{ $text }」不是整數

# {key}: must not be negative
t9c4debaa = { $key }：不可為負數

# After a restart
t58dcea2d = 重新啟動後生效

# Every setting is back to its default.
t763785e7 = 所有設定已恢復為預設值。

# {n} change takes effect when tracewing starts again. | {n} changes take effect when tracewing starts again.
t69f2bb4d = { $n } 項變更將在重新啟動 tracewing 後生效。

# Settings
t4b058728 = 設定

# Undo
t71fd4acf = 復原

# Back to defaults
ta3162a13 = 還原為預設值

# Hosts not to decrypt
tb457f0b3 = 不解密的主機

# One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies at once.
t38e21213 = 每行一個：主機名稱，或 *.example.com（符合該網域及其所有子網域）。這些主機的流量將直接通過，不會被讀取。此清單立即生效。

# Language
t9a73db9b = 語言

# System ({system_name})
tb5c7a1d4 = 跟隨系統（{ $system_name }）

# Applies at once.
tefb7a0f9 = 立即生效。

# Your browser
tb5a48dd2 = 你的瀏覽器

# Its private key is in the system keychain.
t62a47808 = 其私密金鑰存放在系統鑰匙圈中。

# Its private key is in ca.key, which only you can read.
td9c6165d = 其私密金鑰存放在 ca.key 中，只有你能讀取。

# Its private key is in ca.key, which only you can read, as the system keychain cannot take it: {why}.
t453607d0 = 其私密金鑰存放在 ca.key 中，只有你能讀取，因為系統鑰匙圈無法存放它：{ $why }。

# Open the page. If the browser warns about the certificate, or cannot find the page, one of the steps above is not done for that browser.
t2e93b53b = 開啟該頁面。如果瀏覽器出現憑證警告，或找不到頁面，表示該瀏覽器尚未完成上述某個步驟。

# {name} goes through tracewing and trusts it.
t82bd2456 = { $name } 已透過 tracewing 連線，並信任 tracewing。

# {name} goes through tracewing, but opened the page without HTTPS. Open it again over https://.
tfb6feb04 = { $name } 已透過 tracewing 連線，但未使用 HTTPS 開啟頁面。請透過 https:// 重新開啟。

# A phone or another computer can send its traffic here too. Turn on “Let other devices use the proxy” in Settings.
tc18aec73 = 手機或其他電腦也可以將流量傳送到這裡。在「設定」中開啟「允許其他裝置使用代理伺服器」。

# this computer's address, port {port}
t2188b3f4 = 這台電腦的位址，連接埠 { $port }

# On the device, set the Wi-Fi proxy to {addr}. Then open http://{host} there, get the certificate and trust it. Its flows show its address, and can be filtered with device:. Name it from one of them, in Connection.
tc8ccf306 = 在該裝置上，將 Wi-Fi 代理伺服器設為 { $addr }。接著在裝置上開啟 http://{ $host }，下載憑證並信任它。其流量紀錄會顯示裝置位址，可用 device: 篩選。可在任一筆流量紀錄的「連線」中為裝置命名。

# Looking for it…
t50ee7452 = 正在尋找…

# Installing. The system may ask you to confirm.
ta41c38ec = 正在安裝。系統可能會要求你確認。

# Removing. The system may ask you to confirm.
t62b22726 = 正在移除。系統可能會請你確認。

# This computer trusts it.
t27d6bacc = 這台電腦已信任此憑證。

# Installed in: {places}.
tb5bcc244 = 已安裝至：{ $places }。

# Command-line tools trust it, but browsers keep stores of their own. Until it is in them, HTTPS through tracewing fails in a browser.
t327bf8fc = 命令列工具已信任此憑證，但瀏覽器有各自的憑證存放區。在憑證加入其中之前，瀏覽器中經過 tracewing 的 HTTPS 將會失敗。

# tracewing reads HTTPS with a certificate made for this computer alone. Until it is trusted, HTTPS through tracewing fails.
tafd8dae2 = tracewing 使用專為這台電腦產生的憑證來讀取 HTTPS。在此憑證受信任之前，經過 tracewing 的 HTTPS 將會失敗。

# Trust the certificate
t26618699 = 信任憑證

# Install
tb50cbd40 = 安裝

# Also for command-line tools
t0db8faea = 同時套用至命令列工具

# Remove
t21a5901d = 移除

# Take the certificate out of every store on this computer? HTTPS through tracewing then fails until it is installed again. tracewing keeps it, to install again later.
t3c9fc304 = 要從這台電腦的所有憑證庫中移除此憑證嗎？重新安裝之前，經過 tracewing 的 HTTPS 都會失敗。tracewing 會保留它，以便日後重新安裝。

# Browsers on Linux need certutil to be told to trust it. Install it, then Install here:
t76cf4bfa = 在 Linux 上，需要 certutil 才能讓瀏覽器信任它。先安裝 certutil，再在這裡按「安裝」：

# Browsers on Linux need certutil to be told to trust it. Install your distribution's NSS tools, then Install here.
td2606dbb = 在 Linux 上，需要 certutil 才能讓瀏覽器信任它。先安裝發行版的 NSS 工具，再在這裡按「安裝」。

# This desktop has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
tb13a87e6 = 這個桌面沒有 tracewing 能變更的系統代理設定。請在下方「瀏覽器」中單獨開啟瀏覽器，或開啟終端機。

# {desktop} has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
t8d1ad87a = { $desktop } 沒有 tracewing 能變更的系統代理設定。請在下方「瀏覽器」中單獨開啟瀏覽器，或開啟終端機。

# Apps that use the system's setting now go through {addr}. It goes back to what it was when tracewing closes, or if it crashes.
t83387edf = 使用系統設定的 App 現在會透過 { $addr } 連線。tracewing 關閉或當機時，會還原為原本的設定。

# Sends this computer's traffic through tracewing, for apps that use the system's setting. Or point one app at {addr} yourself.
tdcc53241 = 讓使用系統設定的 App 將這台電腦的流量經由 tracewing 傳送。也可以自行將個別 App 指向 { $addr }。

# Turn on the system proxy
t52b88441 = 開啟系統代理伺服器

# Turn off
tcd03e04b = 關閉

# Turn on
t65a0c5a7 = 開啟

# Check it in your browser
tbed6e83a = 在瀏覽器中檢查

# Open the test page
te4e36f65 = 開啟測試頁面

# Set up
t43c06abc = 初始設定

# Three steps to see HTTPS traffic from this computer.
tf3a7f60b = 只要三個步驟，即可檢視這台電腦的 HTTPS 流量。

# See traffic
t980eee79 = 檢視流量

# the browsers' own stores
tddf6836c = 各瀏覽器自己的憑證庫

# Trusts the certificate.
tc5bba858 = 信任此憑證。

# Does not trust the certificate yet.
tf2a0b8c5 = 尚未信任此憑證。

# Looking…
t284579ba = 正在尋找…

# Has no profile yet. Open it once, then look again.
t00e4c88d = 還沒有設定檔。先開啟一次，再重新尋找。

# {holding} of its {n} profile trusts the certificate. | {holding} of its {n} profiles trust the certificate.
t2a9826b9 = { $n } 個設定檔中有 { $holding } 個信任此憑證。

# Has not opened the test page yet.
t67f9cb49 = 尚未開啟測試頁。

# Opened the test page through tracewing, over HTTPS.
t99f8c07e = 已透過 tracewing 以 HTTPS 開啟測試頁。

# Opened the test page through tracewing, but without HTTPS.
t7d4b60bb = 已透過 tracewing 開啟測試頁，但未使用 HTTPS。

# Browsers
td580efd4 = 瀏覽器

# Each keeps its own idea of what to trust and where to send its traffic. Check opens the test page in it. Open on its own starts it with a profile of its own that goes through tracewing, the system proxy left alone.
tce72f1ec = 每個瀏覽器各自決定信任什麼、把流量送往何處。「檢查」會在該瀏覽器中開啟測試頁。「單獨開啟」會用一個經過 tracewing 的獨立設定檔啟動它，不更動系統代理。

# Look again
t98416222 = 重新尋找

# No browser found on this computer.
t0e46bb34 = 在這台電腦上找不到瀏覽器。

# Check
t592c8de7 = 檢查

# Open on its own
t0da7783c = 單獨開啟

# UDP port 443 is blocked on this computer, so apps fall back from HTTP/3 to HTTP/2 and go through tracewing. It stops when tracewing closes, or if it crashes.
t3c727cd5 = 這台電腦已封鎖 UDP 連接埠 443，因此 App 會從 HTTP/3 退回 HTTP/2 並經過 tracewing。tracewing 關閉或當機時，封鎖會自動解除。

# Some apps speak HTTP/3, over UDP, and go around the proxy. Blocking UDP port 443 on this computer makes them fall back to HTTP/2 through tracewing. It takes an administrator's password, and stops when tracewing closes. A VPN that uses UDP port 443 stops working meanwhile.
t64ab600f = 有些 App 透過 UDP 使用 HTTP/3，會繞過代理伺服器。在這台電腦上封鎖 UDP 連接埠 443，可讓它們退回 HTTP/2 並經過 tracewing。此操作需要管理員密碼，tracewing 關閉時會自動解除。在此期間，使用 UDP 連接埠 443 的 VPN 將無法運作。

# Apps on HTTP/3
t2cfe5270 = 使用 HTTP/3 的 App

# Stop blocking
t318358d8 = 停止封鎖

# Block HTTP/3
t88231e40 = 封鎖 HTTP/3

# Or one app at a time
t91c63f1c = 或逐一設定個別 App

# A terminal that goes through tracewing while the system proxy and every other app are left as they are. For a browser, Open on its own in Browsers does the same with a profile of its own.
tc21013a9 = 一個經過 tracewing 的終端機，系統代理和其他 App 都保持原樣。瀏覽器可用「瀏覽器」中的「單獨開啟」，以獨立設定檔做到同樣的事。

# Open a terminal
t4b629c72 = 開啟終端機

# iPhone and iPad
ta550cc97 = iPhone 和 iPad

# Flutter apps
ta04ee397 = Flutter App

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy, Manual: server {server}, port {port}.
t996be047 = 1. 在「設定」>「Wi-Fi」中選擇該網路，然後選擇「設定代理伺服器」>「手動」：伺服器填入 { $server }，埠填入 { $port }。

# 2. Scan the code with the camera, or open {page} in Safari, and get the certificate. Allow the download.
t4fa14138 = 2. 用相機掃描 QR 碼，或在 Safari 中開啟 { $page }，下載憑證。出現提示時選擇「允許」。

# 3. In Settings, General, VPN & Device Management, choose the tracewing CA profile and install it.
t7127295f = 3. 在「設定」>「一般」>「VPN與裝置管理」中，選擇 tracewing CA 描述檔並安裝。

# 4. In Settings, General, About, Certificate Trust Settings, turn on full trust for tracewing CA.
t1b22723c = 4. 在「設定」>「一般」>「關於本機」>「憑證信任設定」中，為 tracewing CA 開啟完全信任。

# 5. Open https://{host} in Safari. If it says it works, apps that use the system's settings go through tracewing.
t4adfec01 = 5. 在 Safari 中開啟 https://{ $host }。如果頁面顯示運作正常，使用系統設定的 App 就會經過 tracewing。

# When you are done
t62466f08 = 用完之後

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy: Off. Left on, the device has no Internet on this network once tracewing closes.
t21a2fdae = 1. 在「設定」→「Wi-Fi」中選擇該網路，然後「設定代理伺服器」：關閉。若保持開啟，tracewing 關閉後裝置在此網路上將無法上網。

# 2. In Settings, General, VPN & Device Management, choose tracewing CA and remove the profile. Its full trust goes with it.
t2090b614 = 2. 在「設定」→「一般」→「VPN 與裝置管理」中選擇 tracewing CA 並移除描述檔。它的完全信任也會一併取消。

# 1. In Settings, Wi-Fi, edit the network, and under Advanced set the proxy to Manual: host {host}, port {port}.
t4f837b10 = 1. 在「設定」>「Wi-Fi」中編輯該網路，在「進階選項」中將「Proxy」設為「手動」：主機名稱填入 { $host }，通訊埠填入 { $port }。

# 2. Scan the code, or open {page} in the browser, and get the certificate.
t654a829b = 2. 掃描 QR 碼，或在瀏覽器中開啟 { $page }，下載憑證。

# 3. In Settings, Security, Encryption & credentials (the names vary by maker), choose Install a certificate, then CA certificate, and pick tracewing.crt.
t8263ba6b = 3. 在「設定」>「安全性」>「加密和憑證」中（名稱因製造商而異），選擇「安裝憑證」>「CA 憑證」，然後選取 tracewing.crt。

# 4. Since Android 7, apps trust certificates the user added only if they say so. For a debug build of your own app, add res/xml/network_security_config.xml:
tb7fa0462 = 4. 從 Android 7 開始，App 只有在明確宣告時才會信任使用者新增的憑證。若是你自己 App 的偵錯版本，請新增 res/xml/network_security_config.xml：

# and point to it from AndroidManifest.xml:
t02ad844b = 並在 AndroidManifest.xml 中參照它：

# Other apps keep refusing the certificate: their traffic shows as failed flows.
t02e57826 = 其他 App 仍會拒絕此憑證：它們的流量會顯示為失敗的流量紀錄。

# 1. In Settings, Wi-Fi, edit the network, and set the proxy back to None. Left on, the device has no Internet on this network once tracewing closes.
t76754f49 = 1. 在「設定」→「Wi-Fi」中修改該網路，把代理伺服器改回「無」。若保持開啟，tracewing 關閉後裝置在此網路上將無法上網。

# 2. In Settings, Security, Encryption & credentials, User credentials, choose tracewing and remove it. Delete tracewing.crt from Downloads too.
tf08245e1 = 2. 在「設定」→「安全性」→「加密與憑證」→「使用者憑證」中選擇 tracewing 並刪除。也請刪除「下載」中的 tracewing.crt。

# Dart's HttpClient, which Flutter's http and Dio use, does not follow the phone's proxy setting, and on Android does not trust certificates the user added. In a debug build, send it here:
t4ffc0abf = Dart 的 HttpClient（Flutter 的 http 和 Dio 都以它為基礎）不會遵循手機的代理伺服器設定，而且在 Android 上不信任使用者新增的憑證。在偵錯版本中，可將其指向這裡：

# badCertificateCallback lets any certificate through, which is why it is for debug builds only.
t991340f1 = badCertificateCallback 會放行任何憑證，因此僅限用於偵錯版本。

# Take TracewingProxy and its line in main() out before a release, though they do nothing outside debug builds. Nothing on the phone was changed.
t12aba34f = 發布前請拿掉 TracewingProxy 以及 main() 中對應的那一行，儘管它們在除錯版本之外不起作用。手機上沒有任何變更。

# Open settings
t1336b4d6 = 開啟設定

# Phones and other devices
t2e3c591c = 手機和其他裝置

# A phone, a tablet or another computer on this network can go through tracewing at {proxy}. The code opens the page with its certificate. Its flows show its address, can be filtered with device:, and can be named from one of them, in Connection.
td0629ca6 = 此網路中的手機、平板或其他電腦可以透過 { $proxy } 經由 tracewing 連線。掃描 QR 碼即可開啟憑證下載頁面。其流量紀錄會顯示裝置位址，可用 device: 篩選，也可在任一筆流量紀錄的「連線」中為裝置命名。

# Other devices cannot reach tracewing at one of this computer's addresses: {problem}
t77fb5f02 = 其他裝置無法透過這台電腦的某個位址連線到 tracewing：{ $problem }

# {n} device went through tracewing ({names}). When you are done, turn its Wi-Fi proxy off, as below: left on, it has no Internet on this network once tracewing closes. | {n} devices went through tracewing ({names}). When you are done, turn their Wi-Fi proxy off, as below: left on, they have no Internet on this network once tracewing closes.
t864ac93f = { $n } 台裝置經過了 tracewing（{ $names }）。用完之後，請依下方說明關閉其 Wi-Fi 代理伺服器：若保持開啟，tracewing 關閉後它在此網路上將無法上網。

# Set up. The certificate is among the system's until the device restarts.
t84db6961 = 已設定完成。在裝置重新啟動之前，此憑證會保留在系統憑證中。

# Set up. The certificate is in Downloads: in the Settings just opened, choose Encryption & credentials, Install a certificate, CA certificate, and pick tracewing.crt.
tbcad7fa7 = 已設定完成。憑證位於「下載」資料夾：在剛開啟的「設定」中，選擇「加密和憑證」>「安裝憑證」>「CA 憑證」，然後選取 tracewing.crt。

# Apps that trust only the system's certificates will still refuse it.
t6fb5ba4b = 只信任系統憑證的 App 仍會拒絕它。

# An emulator.
t14b6b86d = 模擬器。

# On USB.
t67dafc14 = 透過 USB 連接。

# Accept the USB debugging prompt on the device, then look again.
tff6a046e = 請在裝置上接受 USB 偵錯提示，然後重新尋找。

# Still starting. Look again in a moment.
t13f68b64 = 仍在啟動中。請稍後重新尋找。

# adb says it is {state}.
t2b7d71da = adb 回報其狀態為 { $state }。

# Sets the proxy on an Android emulator, or a phone plugged in with USB debugging on, and installs the certificate. The traffic comes over ADB, so neither has to be on this network.
tc28e5d10 = 為 Android 模擬器或已開啟 USB 偵錯並連接的手機設定代理伺服器，並安裝憑證。流量透過 ADB 傳輸，因此兩者都不必連上此網路。

# adb was not found. Install Android Studio, or Android's platform tools, then look again.
t4af01006 = 找不到 adb。請安裝 Android Studio 或 Android 平台工具，然後重新尋找。

# No device is connected. Start an emulator, or plug in a phone with USB debugging on, then look again.
tc65b7cf7 = 未連接任何裝置。請啟動模擬器，或連接已開啟 USB 偵錯的手機，然後重新尋找。

# Set up a device to send its traffic here. Undo, or closing tracewing, takes the proxy off it again.
t9144d1fc = 設定裝置，將其流量傳送到這裡。按一下「復原」或關閉 tracewing，即可移除裝置上的代理伺服器設定。

# Android emulators and USB devices
t379e433c = Android 模擬器和 USB 裝置

# Find devices
t54d4af39 = 尋找裝置

# The system proxy and the block on HTTP/3 go back when tracewing closes, or if it crashes, and so does an Android device set up over USB. The certificate stays trusted until Remove, in step 1, takes it out. A phone set up by hand keeps its Wi-Fi proxy and certificate until they are taken off there: each device's guide says how. Other devices can reach this computer's proxy until that is turned off in Settings.
tf4b6cd8a = tracewing 關閉或當機時，系統代理和 HTTP/3 封鎖都會恢復原狀，透過 USB 設定的 Android 裝置也是如此。憑證會一直受信任，直到第 1 步中的「移除」將其刪除。手動設定的手機會保留 Wi-Fi 代理伺服器和憑證，直到在手機上將其刪除：各裝置的指南說明了做法。在「設定」中關閉該選項之前，其他裝置都能使用這台電腦的代理伺服器。

# Claude Code, Cursor, VS Code and other AI tools can list and read the flows tracewing records, through its MCP server. Tokens and cookies stay out unless Settings let them in. In Claude Code:
t2dc39047 = Claude Code、Cursor、VS Code 等 AI 工具可以透過 tracewing 的 MCP 伺服器列出並讀取其錄製的流量紀錄。除非在「設定」中允許，否則不會包含權杖和 Cookie。在 Claude Code 中：

# Elsewhere, in the tool's MCP settings:
tec38833c = 在其他工具中，請在其 MCP 設定中新增：

# Flows
t4db77db2 = 流量紀錄

# Choose the flow above or below
tecb3adec = 選取上一筆或下一筆流量紀錄

# The first or the last flow
t7e278a00 = 第一筆或最後一筆流量紀錄

# A page up or down
t23b1814b = 向上或向下翻頁

# Filter
tf4a9c097 = 篩選

# Pause or resume recording
t6e21519a = 暫停或繼續錄製

# Clear the list
tf2a55d1f = 清除清單

# The chosen flow
te0a729de = 選取的流量紀錄

# Its request
tb3970784 = 檢視請求

# Its response
t221fc168 = 檢視回應

# Send it again
tef7d1944 = 重新傳送

# Edit it and send it
t6bc99e5c = 編輯後傳送

# Copy it as cURL
tc99a11a3 = 複製為 cURL

# The part before or after the one shown
tdf4221d9 = 目前分頁的前一個或後一個

# Save the body shown
t20d9b7c0 = 儲存目前顯示的本文

# Select all of the part shown
tf9168168 = 全選目前顯示的部分

# Copy the selected text
t00b090d3 = 複製所選文字

# Send it on, as edited
t8c35fa5a = 依修改後的內容送出

# Send it on as it was
t16f7abb5 = 依原樣送出

# Drop it
t4e7f48b9 = 丟棄

# The window
t2af18488 = 視窗

# Set up, Traffic, Rules, Paused, Compose, Sessions, Settings
t9df89562 = 初始設定、流量、規則、已暫停、撰寫、工作階段、設定

# Show or hide the sidebar
t67adca04 = 顯示或隱藏側邊欄

# Show or hide the inspector
ta5a7b9e9 = 顯示或隱藏檢查器

# The inspector beside the list or below it
t51e214f7 = 將檢查器置於清單旁邊或下方

# The next or the previous field
ta4b4bd5e = 下一個或上一個輸入欄

# These shortcuts
tbdf3b62b = 此快速鍵清單

# Close them
tf3a4b1ef = 關閉快速鍵清單

# Keyboard shortcuts
t31eaddb7 = 鍵盤快速鍵

# Close
tcd86acc3 = 關閉

# The certificate: {e}
t0235648c = 憑證：{ $e }

# The terminal
tf542b2f4 = 終端機

# The system proxy: {e}
tc99726ce = 系統代理伺服器：{ $e }

# Blocking HTTP/3: {e}
t024ec871 = 封鎖 HTTP/3：{ $e }

# host:, status:4xx, or text in bodies
t00d0cf30 = host:、status:4xx 或本文中的文字

# A name, as iPhone test
t19f89214 = 名稱，例如：測試用 iPhone

# The search index: {e}
te3c971c9 = 搜尋索引：{ $e }

# An earlier session: {name}
t73446a1d = 先前的工作階段：{ $name }

# Read-only. Search covers more as its index is finished. Recording goes on.
td8659d5b = 唯讀。隨著索引逐步完成，搜尋範圍會持續擴大。錄製仍在繼續。

# Read-only. Recording goes on.
te2f35d9e = 唯讀。錄製仍在繼續。

# Back to live
t317c0453 = 返回即時

# {shown} of {total}
tf2cc0563 = { $shown } / { $total }

# Recording
te75cf20c = 錄製

# System proxy off · {addr}
t7e813d25 = 系統代理伺服器已關閉 · { $addr }

# System proxy…
t29e7dcd0 = 系統代理伺服器…

# System proxy on · {addr}
t319ad6b9 = 系統代理伺服器已開啟 · { $addr }

# Apps
t53d91aad = App

# Devices
t27acf320 = 裝置

# All hosts
tfc7bfe37 = 所有主機

# No host has this in its name
t838c1d44 = 沒有名稱包含此內容的主機

# Hidden
t3c2dfe59 = 已隱藏

# Hosts
tcf9aefb4 = 主機

# Newest
t324b6d0b = 最新

# No flow fits this filter
t7eb295f4 = 沒有符合此篩選條件的流量紀錄

# Nothing yet
tbd00747c = 尚無內容

# Turn on the system proxy above, or point an app at {addr}. To try it: curl --proxy http://{addr} --cacert {ca} https://example.com/
t61730a16 = 開啟上方的系統代理伺服器，或將 App 指向 { $addr }。試試看：curl --proxy http://{ $addr } --cacert { $ca } https://example.com/

# Beside the list
t3b4853f8 = 清單旁邊

# Below the list
t23658daf = 清單下方

# Drop it on a place · Esc puts it back
t0fa632ba = 拖放到目標位置 · 按 Esc 還原

# Traffic
t4b589266 = 流量

# Certificate trusted on this computer
t7b93f551 = 這台電腦已信任憑證

# Checking the certificate…
tbf8942c3 = 正在檢查憑證…

# Installing the certificate; the system may ask you to confirm…
t068dcaff = 正在安裝憑證，系統可能會要求你確認…

# Removing the certificate; the system may ask you to confirm…
t913da735 = 正在移除憑證；系統可能會請你確認…

# HTTPS will fail until the certificate is trusted: set up
t4b34beb2 = 憑證受信任前，HTTPS 將無法使用：前往初始設定

# settings.toml was not used: {problem}
tc3e1f9c8 = 未使用 settings.toml：{ $problem }

# A setting was not saved: {problem}
tff1a5018 = 有一項設定未儲存：{ $problem }

# The proxy stopped: {failure}
t04dcff11 = 代理伺服器已停止：{ $failure }

# The disk is nearly full: bodies are not being kept
te99f9959 = 磁碟空間即將用盡：目前不保留本文

# {count} flow not recorded | {count} flows not recorded
tdbd64fe6 =
    { $n ->
       *[other] { $count } 筆流量紀錄未錄製
    }

# {count} flow without bodies | {count} flows without bodies
t249093b0 =
    { $n ->
       *[other] { $count } 筆流量紀錄沒有本文
    }

# {count} oldest flow dropped to stay under {size} | {count} oldest flows dropped to stay under {size}
te30201f0 =
    { $n ->
       *[other] 為維持在 { $size } 以內，已捨棄最舊的 { $count } 筆流量紀錄
    }

# {count} flow | {count} flows
t57bf5200 =
    { $n ->
       *[other] { $count } 筆流量紀錄
    }

# Export as HAR
t95b6b766 = 匯出為 HAR

# Shortcuts {keys}
t4ab0ecb4 = 快速鍵 { $keys }

# Columns
tea156fdc = 欄位
