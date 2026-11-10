# tracewing in simplified Chinese. Each message is found by its English, the
# comment above it; scripts/i18n.py keeps this file in step with the code.

# … {count} line {closer} | … {count} lines {closer}
t2ba67348 = … { $count } 行 { $closer }

# … {size} more
ta8dc6eab = … 还有 { $size }

# Unfold all
t3849d44e = 全部展开

# {size} · {count} line | {size} · {count} lines
t51332550 =
    { $n ->
       *[other] { $size } · { $count } 行
    }

# {size} · reading {percent}%
t49fe2cb3 = { $size } · 正在读取 { $percent }%

# Appearance: {now}. Click for {next}.
t332cfa6d = 外观：{ $now }。点击切换为 { $next }。

# Method
t98a2c750 = 方法

# Status
t005ef20f = 状态

# App
t81f51acc = 应用

# Host
t302af18f = 主机

# Path
teb66e456 = 路径

# Operation
t218e02a6 = 操作

# Size
ta6478e7c = 大小

# Time
tdfe4e404 = 时间

# Number
t61e9eac0 = 序号

# "{method}" is not a method
tacdcf5cb = “{ $method }” 不是有效的方法

# its request body was not kept whole, so it cannot be sent again
t1f9e5368 = 其请求正文未完整保留，因此无法重新发送

# name: value, one a line
t4df37bea = 名称: 值，每行一个

# No body
tfa4c2902 = 无正文

# No flow is chosen
t550b0a98 = 未选择流量记录

# This flow: {e}
t296762f8 = 此流量记录：{ $e }

# Request
t82454152 = 请求

# Headers
t334f5989 = 标头

# Body
t428a1095 = 正文

# Sending…
te946e7bf = 正在发送…

# Send
tf28e14cf = 发送

# New
t8b242db1 = 新建

# … {count} more bytes: the flow in Traffic has them all.
tf43ea06c = … 还有 { $count } 字节：“流量”中的该流量记录包含完整内容。

# Response
t55a982fe = 响应

# Compose
t18049f93 = 编写

# A request of your own, or one sent again. It goes through tracewing, so rules apply and it shows in Traffic too.
t7a9cf113 = 自己编写的请求，或重新发送的请求。它会经过 tracewing，因此规则同样生效，也会显示在“流量”中。

# {version} both sides
t961a5ea0 = 两端均为 { $version }

# {client} to the proxy, {server} to the server
t3d237646 = 与代理之间 { $client }，与服务器之间 { $server }

# Queued
t9e437c74 = 排队

# Connect
tdb89a899 = 连接

# Waiting
tc9450e48 = 等待响应

# Download
t6964bb19 = 下载

# No file picker: {e}
t6935fbd3 = 无法打开文件选择器：{ $e }

# Writing the HAR…
t725a0e89 = 正在写入 HAR…

# {n} flow written to {path} | {n} flows written to {path}
t643f96ee =
    { $n ->
       *[other] 已将 { $n } 条流量记录写入 { $path }
    }

# The HAR was not written: {e}
tef8ec569 = 未能写入 HAR：{ $e }

# Open
t538b10e9 = 打开

# Reading {path}…
tc0e7c8f4 = 正在读取 { $path }…

# , {n} entry left out | , {n} entries left out
td848c31b =
    { $n ->
       *[other] ，省略了 { $n } 个条目
    }

# {n} flow read from a {format} | {n} flows read from a {format}
t610d7810 =
    { $n ->
       *[other] 已从 { $format } 读取 { $n } 条流量记录
    }

# The file was not read: {e}
t7f49510f = 未能读取文件：{ $e }

# it is no longer kept
ta7b4c380 = 已不再保留

# {size} saved to {path}
tc6932467 = 已将 { $size } 保存到 { $path }

# The body was not saved: {e}
t189900b4 = 未能保存正文：{ $e }

# Fiddler session
t9f5c89a3 = Fiddler 会话

# Charles session
tb4f56f2b = Charles 会话

# All
t75afe364 = 全部

# Errors
te9ed60a6 = 错误

# Media
td10fcaa3 = 媒体

# Rule: {rule}
teca2074f = 规则：{ $rule }

# Events
t003420a4 = 事件

# Not decrypted
t8090a1ba = 未解密

# open
td35ec4c9 = 进行中

# Reading the request…
t51c96267 = 正在读取请求…

# The request's body was not kept, so its query cannot be shown.
tba9e5628 = 请求正文未保存，因此无法显示其查询。

# An operation without a name
t38c4d30b = 未命名的操作

# {n} of {count}: {name}
t53045460 = 第 { $n }/{ $count } 个：{ $name }

# Query
t9e89e783 = 查询

# A persisted query: the server keeps its text, and the request sends only its hash.
tf23e3c2a = 持久化查询：服务器保存其文本，请求只发送其哈希。

# Variables
t7996f062 = 变量

# Cut
t58e25863 = 剪切

# Copy
t658f3664 = 复制

# Paste
tb34a0288 = 粘贴

# Select all
t6af21072 = 全选

# This flow is gone.
t3343999e = 此流量记录已不存在。

# This host is not decrypted, so what it sends cannot be seen.
tf85fe84f = 此主机未解密，因此无法查看其发送的内容。

# No body.
t31e4d044 = 无正文。

# This body was not kept: the disk was nearly full, or too much was waiting for it.
t884aad28 = 此正文未保留：磁盘即将用满，或待写入的内容过多。

# This body was not kept.
te46616b5 = 此正文未保留。

# This body is gone: it was dropped with the oldest flows, or cannot be read.
tf9a7ab60 = 此正文已不存在：已随最早的流量记录一起丢弃，或无法读取。

# Cannot open this body: {e}
t620a039b = 无法打开此正文：{ $e }

# Inspector
t4def410e = 检查器

# Choose a flow to see it here
tc7be91e8 = 选择一条流量记录以在此查看

# Not decrypted. New connections to this host go through unread.
ta05ec37a = 未解密。到此主机的新连接将直接通过，不被读取。

# If an app fails with this host, it may pin its certificates.
t3cbba118 = 如果某个应用连接此主机失败，它可能使用了证书锁定。

# Make a rule
t0ac0e062 = 创建规则

# Decrypt this host
tb7367e2c = 解密此主机

# Do not decrypt this host
t5fa113f6 = 不解密此主机

# Picture · {size}
te97ed633 = 图片 · { $size }

# Messages {count}
te61b841a = 消息 { $count }

# Headers {headers}
t04b0146d = 标头 { $headers }

# Cookies {cookies}
td58eb84b = Cookie { $cookies }

# Query {query}
t4756f9f5 = 查询参数 { $query }

# Form {form}
tfa6e7203 = 表单 { $form }

# Connection
t09ea4ff9 = 连接

# Save body
te2c44424 = 保存正文

# Send again
tf5910557 = 重新发送

# Edit and send
tf837e542 = 编辑后发送

# Copied
t8dc21305 = 已复制

# Copy as cURL
t866d1e3e = 复制为 cURL

# Copy as…
t9c1ad448 = 复制为…

# This picture cannot be drawn. Save the body to look at it.
t1e92fb27 = 无法显示此图片。请保存正文后查看。

# Opening…
tc40f2321 = 正在打开…

# No response
t8c2c37b1 = 无响应

# Tunnel, not decrypted
tce693254 = 隧道，未解密

# GraphQL: {operation}
t2d65757d = GraphQL：{ $operation }

# Over connection {id}, made before: no time went on connecting.
t84144a81 = 复用之前建立的连接 { $id }：没有花费连接时间。

# {app}, pid {pid}
t627147b4 = { $app }，pid { $pid }

# Not found: it may have ended, or belong to another user
t0efce58b = 未找到：可能已结束，或属于其他用户

# On another device
t2b964a37 = 在其他设备上

# CLIENT
t6d3cd99e = 客户端

# Device
tbdcbb413 = 设备

# Program
tc4253b6b = 程序

# Address
t700560f3 = 地址

# SERVER
te3b27612 = 服务器

# What this connection was is no longer kept: it went with older flows.
t1ef5a00a = 此连接的信息已不再保留：已随较早的流量记录一起清除。

# Not decrypted: tracewing passed it on without looking.
t0427c9b1 = 未解密：tracewing 未查看内容便直接转发。

# No connection was made.
tf348eb92 = 未建立连接。

# None: it was answered without one.
tac71808e = 无：未建立连接即已响应。

# {id}, given up during TLS
t0ec66a8c = { $id }，在 TLS 阶段放弃

# {id}, made for this flow
t7fe32cfc = { $id }，为此流量记录新建

# {id}, made before and used again
ta904c078 = { $id }，复用之前建立的连接

# Server
t6d7302f2 = 服务器

# None: plain HTTP
t05606349 = 无：纯 HTTP

# THE SERVER SHOWED NO CERTIFICATES
te476fbb8 = 服务器未出示证书

# WHAT THE SERVER SHOWED, ITS OWN FIRST
ta51f3107 = 服务器出示的证书，其自身证书在前

# NO CERTIFICATES
te6f31dce = 无证书

# CERTIFICATE
t1c902790 = 证书

# {n} CERTIFICATE, THE SERVER'S FIRST | {n} CERTIFICATES, THE SERVER'S FIRST
td5295f62 =
    { $n ->
       *[other] { $n } 个证书，服务器自身证书在前
    }

# tracewing would not go on with this server: {error}. If the app trusts it some other way, such as through a company CA, choose “Do not decrypt this host” and the app will check the server itself.
t13ced472 = tracewing 拒绝继续连接此服务器：{ $error }。如果应用通过其他方式信任它（例如通过公司 CA），请选择“不解密此主机”，由应用自行验证服务器。

# Cipher
t5b3bed6a = 加密套件

# none
tada7afdb = 无

# not valid until {when}
t0a14b2d8 = { $when }才生效

# expired {when}
t932538c2 = 已于 { $when }过期

# expires {when}
t679bcb51 = 将于 { $when }过期

# {names}, and {n} more | {names}, and {n} more
t4cd375ad =
    { $n ->
       *[other] { $names }，以及另外 { $n } 个
    }

# Issued by {issuer}
t15a1ce5a = 颁发者：{ $issuer }

# Valid {from} to {to} · {when}
t0e47d4f2 = 有效期 { $from } 至 { $to } · { $when }

# Names {names}
tda75e459 = 名称：{ $names }

# Serial {serial}
td1f0d98b = 序列号：{ $serial }

# These headers were not kept.
tb8739fec = 这些标头未保留。

# No headers.
tf3dacb5e = 没有标头。

# File {filename} · {kind} · {size}
t6dbd239f = 文件 { $filename } · { $kind } · { $size }

# no type
t902a6ece = 无类型

# No cookies were sent.
t58e85ab2 = 未发送 Cookie。

# No cookies were set.
tf57dbabc = 未设置 Cookie。

# JWT · {algorithm} · signature not checked
t2db33271 = JWT · { $algorithm } · 未验证签名

# JWT · signature not checked
t05f8b533 = JWT · 未验证签名

# Header
t114f9980 = 头部

# Payload
ta0639745 = 载荷

# Expires
tc7aa0523 = 过期时间

# Issued
tb1835ecc = 签发时间

# Not before
td10dacd9 = 生效时间

# in {amount}
tfda57c3e = { $amount }后

# {amount} ago
t5f968014 = { $amount }前

# Light
t8ad2f8cf = 浅色

# Dark
t45570065 = 深色

# System
t8f3184bc = 跟随系统

# streamed in {count} chunk | streamed in {count} chunks
t76711a1a =
    { $n ->
       *[other] 分 { $count } 块流式传输
    }

# stopped: {stop}
tfa48fb30 = 停止原因：{ $stop }

# {input} in · {output} out tokens
tc8bbc724 = 输入 { $input } · 输出 { $output } token

# Reading the call…
t33b52a3c = 正在读取调用…

# Answer
tf94ea0ff = 回答

# Thinking: {thinking}
t737ece33 = 思考：{ $thinking }

# Calls {name}{id}
t017882e4 = 调用 { $name }{ $id }

# Nothing yet.
td9b71d16 = 暂无内容。

# {text}… and {count} more character | {text}… and {count} more characters
t6a16c074 =
    { $n ->
       *[other] { $text }… 还有 { $count } 个字符
    }

# Copy the answer
t69975ed7 = 复制回答

# Asked
t4e3af9c3 = 提问

# Tools offered: {tools}
tb871bcf4 = 提供的工具：{ $tools }

# Not kept
tba30b5fa = 未保留

# This message was not kept: too much was waiting, or the disk was nearly full.
tb888a3c3 = 此消息未保留：待处理的内容过多，或磁盘即将用满。

# This message is empty.
tdb0ec01f = 此消息为空。

# No messages yet
t51e7624c = 暂无消息

# {said} kept, of {size}
t3558745a = 已保留 { $said }，共 { $size }

# {what}: {text} is not a whole number
t2a8515ce = { $what }：{ $text } 不是整数

# Latency
t23a45ef9 = 延迟

# Down
ta4995035 = 下行

# Up
t42f35940 = 上行

# Loss
t69db303a = 丢包

# Give a latency, a speed or a loss to slow it down by
t7cfd6404 = 请填写要模拟的延迟、速度或丢包率

# Latency, ms
tefe18ab9 = 延迟（ms）

# Down, kbit/s
t08c4ae33 = 下行（kbit/s）

# Up, kbit/s
td55cfe9a = 上行（kbit/s）

# Loss, %
tea4af439 = 丢包率（%）

# Only these hosts, like *.example.com; all if empty
t77a1f991 = 仅限这些主机，如 *.example.com；留空则为全部

# Not kept for next time: {e}
tdce4d67c = 未能保存以供下次使用：{ $e }

# Slow network
td51ddfce = 慢速网络

# Off: traffic goes as fast as it can
tf0f34db0 = 关闭：流量以最快速度传输

# Custom
t9525b78e = 自定义

# Only these hosts
tfe16992b = 仅限这些主机

# Apply
tca77bbdb = 应用

# Applies from the next request. Latency is added to each round trip; the speed is a link that every slowed flow shares; a lost piece waits to be sent again, as TCP does. Hosts left alone are not slowed.
t5ee4034d = 从下一个请求开始生效。每次往返都会增加延迟；速度是所有被限速的流量记录共享的一条链路；丢失的数据包会像 TCP 一样等待重传。未列出的主机不受限速。

# Header line {line} has no ":"
t891b076d = 标头第 { $line } 行缺少 “:”

# Header line {line} has no name
te3247b96 = 标头第 { $line } 行缺少名称

# URL
t7569633e = URL

# {n} bytes that are not text: they go on as they came. | {n} bytes that are not text: they go on as they came.
td6aec821 =
    { $n ->
       *[other] { $n } 字节非文本内容：将按原样传递。
    }

# Nothing is paused
t8cf68b4c = 没有已暂停的项

# "{status}" is not a status
t40a529af = “{ $status }” 不是有效的状态码

# REQUEST
t1f3f6dd2 = 请求

# RESPONSE
tc85f77de = 响应

# Nothing is paused. Add a Pause rule on the Rules screen to stop traffic here.
tf9ab4327 = 没有已暂停的项。在“规则”页面添加“暂停”规则，即可在此拦停流量。

# for {method} {url}
t093a86f6 = 针对 { $method } { $url }

# The request, before the server gets it
tc840fed6 = 服务器收到之前的请求

# The response, before the app gets it
t6683d72a = 应用收到之前的响应

# Send as it came
tb593e730 = 按原样发送

# Drop
t108524ac = 丢弃

# Paused
tde281feb = 已暂停

# Nothing is waiting.
tea613769 = 没有等待中的项。

# {n} waits to be sent on. | {n} wait to be sent on.
t745634c6 =
    { $n ->
       *[other] { $n } 项等待继续发送。
    }

# Send all as they came
tdce25640 = 全部按原样发送

# Block
tbd074002 = 拦截

# Answer from a file
td8edab50 = 用文件响应

# Send to another host
te3cd85d7 = 转发到其他主机

# Set a request header
tfad4aaaa = 设置请求标头

# Set a response header
tcbbc20d0 = 设置响应标头

# Remove a request header
t5228dd84 = 移除请求标头

# Remove a response header
teeb5735a = 移除响应标头

# Set a query parameter
t73d9419b = 设置查询参数

# Remove a query parameter
t12633da9 = 移除查询参数

# Change the request body
t41101201 = 修改请求正文

# Change the response body
tdd259b73 = 修改响应正文

# Change the status
t4bbdfba8 = 修改状态码

# Pause the request
t40ecc39b = 暂停请求

# Pause the response
t18a34799 = 暂停响应

# Status to answer with, like 403
t8f3ebb6c = 要返回的状态码，如 403

# Full path of a file or folder
t9fb9ae4c = 文件或文件夹的完整路径

# Where to, like http://localhost:3000
tdfedd004 = 目标地址，如 http://localhost:3000

# Header's name
t80793085 = 标头名称

# Its value
t43968778 = 值

# Parameter's name
t313c8e35 = 参数名称

# Text to find, or empty for the whole body
t8255fc87 = 要查找的文本，留空则替换整个正文

# What to put in its place
t03671fd5 = 替换成的内容

# Status the client gets, like 500
t5e4d3088 = 客户端收到的状态码，如 500

# "{first}" is not a status
td43128d3 = “{ $first }” 不是有效的状态码

# Block with {status}
t1727e984 = 以 { $status } 拦截

# Answer from {path}
taefc08fa = 用 { $path } 响应

# Send to {to}
t5e256873 = 转发到 { $to }

# Request: {header}: {value}
taa230766 = 请求：{ $header }: { $value }

# Response: {header}: {value}
t30f9e2c2 = 响应：{ $header }: { $value }

# Request: no {header}
t6f0b3f5a = 请求：去掉 { $header }

# Response: no {header}
t23960a2e = 响应：去掉 { $header }

# Query: {name}={value}
t0ee6c1c0 = 查询：{ $name }={ $value }

# Query: no {name}
tf2e7bf27 = 查询：去掉 { $name }

# Status {status}
tad048e53 = 状态码 { $status }

# Request body: {replace}
te4856c36 = 请求正文：{ $replace }

# Response body: {replace}
t8600dc42 = 响应正文：{ $replace }

# Request body: {find} → {replace}
taec471a9 = 请求正文：{ $find } → { $replace }

# Response body: {find} → {replace}
tc1449525 = 响应正文：{ $find } → { $replace }

# Name
t0fe07306 = 名称

# URL, like api.myapp.dev/v1/*
t4dd91658 = URL，如 api.myapp.dev/v1/*

# Method, or empty for all
tb2b869ac = 方法，留空则匹配全部

# {n} rule written to {path} | {n} rules written to {path}
t4a2e843a =
    { $n ->
       *[other] 已将 { $n } 条规则写入 { $path }
    }

# The rules were not written: {e}
te7ac257f = 未能写入规则：{ $e }

# Import
t573e0af4 = 导入

# {path} could not be read: {e}
t32a3ce94 = 无法读取 { $path }：{ $e }

# Nothing was imported: {e}
tb9f7257a = 未导入任何内容：{ $e }

# Edit
tc2c76cb1 = 编辑

# Delete
t5797ea6a = 删除

# Change this rule
t9160f393 = 修改此规则

# A new rule
t55a722a0 = 新规则

# It waits under Paused until you send it on, changed or not.
tca566a57 = 它会在“已暂停”中等待，直到你继续发送（修改与否均可）。

# Save
t4d2d5d68 = 保存

# Add
t9dc3aa14 = 添加

# Cancel
t35afca3b = 取消

# Rules
t428b2d80 = 规则

# They apply from the next request, while recording is paused too. Top to bottom: the first block, file or host wins, and every other rule that fits applies, in order. A rule that changes a body or a status changes what the server sent.
tbe44b31e = 规则从下一个请求起生效，暂停记录时也一样。自上而下：第一个匹配的拦截、文件或主机规则生效，其他匹配的规则按顺序全部生效。修改正文或状态码的规则，改的是服务器发来的内容。

# Import rules…
t5edc205d = 导入规则…

# Export rules…
t6e09ca20 = 导出规则…

# {n} rule added | {n} rules added
tbc5e0866 =
    { $n ->
       *[other] 已添加 { $n } 条规则
    }

# {n} here already | {n} here already
t65dc1281 =
    { $n ->
       *[other] { $n } 条已存在
    }

# left out: {why}
ta81e95e2 = 已跳过：{ $why }

# Copy all
t6aef5c37 = 全部复制

# Too large to copy all
t2fe8ff9f = 太大，无法全部复制

# No flows
t6dc463eb = 无流量记录

# Name it to keep it
t52c1a402 = 命名后即可保留

# {size} · written by another version, so it cannot be opened
t878e5c80 = { $size } · 由其他版本写入，无法打开

# {count} flow · {size} | {count} flows · {size}
t3f3c0eb4 =
    { $n ->
       *[other] { $count } 条流量记录 · { $size }
    }

# Recording now
t3cb3e6fc = 正在录制

# Export HAR
t4d6458cc = 导出 HAR

# Delete for good?
ta7737f13 = 永久删除？

# Sessions
tb69a8aec = 会话

# A session with a name is kept until you delete it. Of the others, the newest {kept} stay.
t6391f080 = 已命名的会话会一直保留，直到你将其删除。其余会话只保留最新的 { $kept } 个。

# Working…
td5507854 = 正在处理…

# Open a session file
tae8622fc = 打开会话文件

# HAR, Fiddler .saz, or Charles .chlsj
td8955e6d = HAR、Fiddler .saz 或 Charles .chlsj

# Exported HAR files leave out tokens and cookies
td4a2f543 = 导出的 HAR 文件不包含令牌和 Cookie

# Other devices
t9e7d8362 = 其他设备

# Let other devices use the proxy
t13e626b3 = 允许其他设备使用代理

# Phones and other computers on your network can then send their traffic here. So can anyone else on it: leave this off on a network you do not trust. Takes effect when tracewing starts again.
t2e14229d = 开启后，你网络中的手机和其他电脑可以将流量发送到这里。但同一网络中的任何人也都可以：在不信任的网络上请保持关闭。重新启动 tracewing 后生效。

# Servers
tcd0cf113 = 服务器

# Trust what this computer trusts
tfea2d91b = 信任本机信任的证书

# Servers with a certificate from an authority installed on this computer, such as a company's own, are reached as this computer's browsers reach them. Off, only the public authorities are trusted. Takes effect when tracewing starts again.
t1705bfb4 = 对于持有本机已安装的证书颁发机构（例如公司自己的 CA）所颁发证书的服务器，将像本机浏览器一样正常访问。关闭时，仅信任公共证书颁发机构。重新启动 tracewing 后生效。

# AI tools
t294d7ea4 = AI 工具

# Let AI tools see tokens and cookies
te1d26d3c = 允许 AI 工具查看令牌和 Cookie

# AI tools read recorded traffic through tracewing's MCP server. Off, the values of Authorization, Cookie and Set-Cookie are left out of what they get. Applies at once.
tb496980f = AI 工具通过 tracewing 的 MCP 服务器读取已录制的流量。关闭时，提供给它们的内容不包含 Authorization、Cookie 和 Set-Cookie 的值。立即生效。

# HTTP/3
tfec0f477 = HTTP/3

# Keep apps off HTTP/3
te42733a9 = 阻止应用使用 HTTP/3

# HTTP/3 goes over UDP, which no proxy carries, so an app that uses it goes around tracewing. This takes servers' offers of it out of their responses, and apps stay on HTTP/2. Flows still show the offers as servers sent them. Takes effect when tracewing starts again.
t2aa7be0d = HTTP/3 基于 UDP，而代理无法转发 UDP，因此使用 HTTP/3 的应用会绕过 tracewing。此选项会从服务器响应中移除 HTTP/3 通告，让应用继续使用 HTTP/2。流量记录中仍会显示服务器发送的原始通告。重新启动 tracewing 后生效。

# Size of a session
ta20fa102 = 会话大小

# Past this, the oldest flows are dropped.
td2914f46 = 超过此值时，将丢弃最早的流量记录。

# Room to leave on the disk
t8674a3b3 = 磁盘保留空间

# With less than this free, flows are still listed but bodies are not kept.
t5b3a31a3 = 可用空间低于此值时，流量记录仍会列出，但不保留正文。

# Earlier sessions to keep
td356194a = 保留的历史会话数

# Older ones are deleted when a new session starts.
t66556513 = 新会话开始时，将删除更早的会话。

# Bodies
tdb4d0409 = 正文

# Most kept of one body
t777508dd = 单个正文最大保留量

# It still reaches the client whole.
t386fee28 = 客户端仍会收到完整内容。

# Most unpacked for viewing
tc3fe2efd = 查看时最大解压量

# Stops a small download that unpacks to many GB.
tf9c49515 = 防止体积很小的下载内容解压后膨胀到数 GB。

# Small body size
ta8996af1 = 小正文阈值

# Smaller bodies wait in memory and share a file. Larger ones get a file of their own as they arrive.
t20c108b4 = 较小的正文先暂存在内存中，并共用一个文件。较大的正文在接收时各自写入单独的文件。

# Compress plain text on disk
tee503c2e = 在磁盘上压缩纯文本

# Takes about half the room, and is skipped when traffic is too fast for it.
t6d70f53f = 约可节省一半空间；流量过大来不及压缩时会跳过。

# Compression level
td386c9ef = 压缩级别

# 1 to 19. Higher is smaller and slower.
td42a987d = 1 到 19。数值越高，体积越小、速度越慢。

# Search
tc646a2c9 = 搜索

# Search URLs
t0c3e0b3d = 搜索 URL

# Finds any part of a URL as you type. Costs little.
t3f22d666 = 输入时即可匹配 URL 的任意部分。开销很小。

# Search bodies
t8eec0293 = 搜索正文

# Finds words in bodies. Can take as much disk as the bodies do.
tcac3872a = 在正文中查找文字。占用的磁盘空间可能与正文本身相当。

# Searched part of each body
tab293b21 = 每个正文的搜索范围

# Words past this are not found. More takes more disk.
t0bf65f4a = 超出此范围的文字不会被搜到。范围越大，占用磁盘越多。

# Under heavy traffic
ta99888cb = 高流量时

# Memory for bodies waiting to be written
t7d6ff799 = 待写入正文的内存上限

# Past this, flows are listed without their bodies. Traffic is never held back.
t5b268fb2 = 超过此值时，流量记录将不带正文列出。流量本身永远不会被阻塞。

# Memory for large bodies waiting to be written
t07b9de08 = 待写入大正文的内存上限

# Past this, large bodies stop being kept.
tfe61a4f3 = 超过此值时，将不再保留大正文。

# {key}: "{text}" is not a whole number
ta8606963 = { $key }：“{ $text }” 不是整数

# {key}: must not be negative
t9c4debaa = { $key }：不能为负数

# Saved. The limits apply the next time tracewing starts.
tc8bd2065 = 已保存。这些限制将在下次启动 tracewing 时生效。

# Settings
t4b058728 = 设置

# Back to defaults
ta3162a13 = 恢复默认值

# Hosts not to decrypt
tb457f0b3 = 不解密的主机

# One a line: a host, or *.example.com for it and everything under it. Their traffic goes through unread. This list applies as soon as it is saved.
t7647c310 = 每行一个：主机名，或 *.example.com（匹配该域名及其所有子域名）。这些主机的流量将直接通过，不被读取。此列表保存后立即生效。

# Language
t9a73db9b = 语言

# System ({system_name})
tb5c7a1d4 = 跟随系统（{ $system_name }）

# Applies at once.
tefb7a0f9 = 立即生效。

# Your browser
tb5a48dd2 = 你的浏览器

# Its private key is in the system keychain.
t62a47808 = 其私钥存放在系统钥匙串中。

# Its private key is in ca.key, which only you can read.
td9c6165d = 其私钥存放在 ca.key 中，只有你能读取。

# Its private key is in ca.key, which only you can read, as the system keychain cannot take it: {why}.
t453607d0 = 其私钥存放在 ca.key 中，只有你能读取，因为系统钥匙串无法存储它：{ $why }。

# Open the page. If the browser warns about the certificate, or cannot find the page, one of the steps above is not done for that browser.
t2e93b53b = 打开该页面。如果浏览器发出证书警告，或找不到页面，说明该浏览器尚未完成上述某个步骤。

# {name} goes through tracewing and trusts it.
t82bd2456 = { $name } 已通过 tracewing 连接，并信任 tracewing。

# {name} goes through tracewing, but opened the page without HTTPS. Open it again over https://.
tfb6feb04 = { $name } 已通过 tracewing 连接，但未使用 HTTPS 打开页面。请通过 https:// 重新打开。

# A phone or another computer can send its traffic here too. Turn on “Let other devices use the proxy” in Settings, then start tracewing again.
t9f95b7b8 = 手机或其他电脑也可以将流量发送到这里。在“设置”中开启“允许其他设备使用代理”，然后重新启动 tracewing。

# this computer's address, port {port}
t2188b3f4 = 本机地址，端口 { $port }

# On the device, set the Wi-Fi proxy to {addr}. Then open http://{host} there, get the certificate and trust it. Its flows show its address, and can be filtered with device:. Name it from one of them, in Connection.
tc8ccf306 = 在该设备上，将 Wi-Fi 代理设置为 { $addr }。然后在设备上打开 http://{ $host }，下载证书并信任它。其流量记录会显示设备地址，可用 device: 过滤。可在任一条流量记录的“连接”中为设备命名。

# Looking for it…
t50ee7452 = 正在查找…

# Installing. The system may ask you to confirm.
ta41c38ec = 正在安装。系统可能会要求你确认。

# Removing. The system may ask you to confirm.
t62b22726 = 正在移除。系统可能会请你确认。

# This computer trusts it.
t27d6bacc = 本机已信任该证书。

# Installed in: {places}.
tb5bcc244 = 已安装到：{ $places }。

# Command-line tools trust it, but browsers keep stores of their own. Until it is in them, HTTPS through tracewing fails in a browser.
t327bf8fc = 命令行工具已信任该证书，但浏览器有各自的证书存储。在证书加入其中之前，浏览器中经过 tracewing 的 HTTPS 将会失败。

# tracewing reads HTTPS with a certificate made for this computer alone. Until it is trusted, HTTPS through tracewing fails.
tafd8dae2 = tracewing 使用专为本机生成的证书来读取 HTTPS。在该证书受信任之前，经过 tracewing 的 HTTPS 将会失败。

# Trust the certificate
t26618699 = 信任证书

# Install
tb50cbd40 = 安装

# Also for command-line tools
t0db8faea = 同时用于命令行工具

# Remove
t21a5901d = 移除

# Take the certificate out of every store on this computer? HTTPS through tracewing then fails until it is installed again. tracewing keeps it, to install again later.
t3c9fc304 = 要从这台电脑的所有证书库中移除该证书吗？重新安装之前，经过 tracewing 的 HTTPS 都会失败。tracewing 会保留它，以便以后重新安装。

# Browsers on Linux need certutil to be told to trust it. Install it, then Install here:
t76cf4bfa = 在 Linux 上，需要 certutil 才能让浏览器信任它。先安装 certutil，再在这里点“安装”：

# Browsers on Linux need certutil to be told to trust it. Install your distribution's NSS tools, then Install here.
td2606dbb = 在 Linux 上，需要 certutil 才能让浏览器信任它。先安装发行版的 NSS 工具，再在这里点“安装”。

# This desktop has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
tb13a87e6 = 这个桌面没有 tracewing 能更改的系统代理设置。请在下方“浏览器”中单独打开一个浏览器，或打开终端。

# {desktop} has no system proxy setting that tracewing can change. Open a browser on its own from Browsers, or a terminal, below.
t8d1ad87a = { $desktop } 没有 tracewing 能更改的系统代理设置。请在下方“浏览器”中单独打开一个浏览器，或打开终端。

# Apps that use the system's setting now go through {addr}. It goes back to what it was when tracewing closes, or if it crashes.
t83387edf = 使用系统设置的应用现在会通过 { $addr }。tracewing 关闭或崩溃时，将恢复为原来的设置。

# Sends this computer's traffic through tracewing, for apps that use the system's setting. Or point one app at {addr} yourself.
tdcc53241 = 让使用系统设置的应用将本机流量经由 tracewing 发送。也可以自行将某个应用指向 { $addr }。

# Turn on the system proxy
t52b88441 = 开启系统代理

# Turn off
tcd03e04b = 关闭

# Turn on
t65a0c5a7 = 开启

# Check it in your browser
tbed6e83a = 在浏览器中检查

# Open the test page
te4e36f65 = 打开测试页面

# Set up
t43c06abc = 配置

# Three steps to see HTTPS traffic from this computer.
tf3a7f60b = 三步即可查看本机的 HTTPS 流量。

# See traffic
t980eee79 = 查看流量

# the browsers' own stores
tddf6836c = 各浏览器自己的证书库

# Trusts the certificate.
tc5bba858 = 信任该证书。

# Does not trust the certificate yet.
tf2a0b8c5 = 尚未信任该证书。

# Looking…
t284579ba = 正在查找…

# Has no profile yet. Open it once, then look again.
t00e4c88d = 还没有配置文件。先打开一次，再重新查找。

# {holding} of its {n} profile trusts the certificate. | {holding} of its {n} profiles trust the certificate.
t2a9826b9 = { $n } 个配置文件中有 { $holding } 个信任该证书。

# Has not opened the test page yet.
t67f9cb49 = 尚未打开测试页。

# Opened the test page through tracewing, over HTTPS.
t99f8c07e = 已通过 tracewing 以 HTTPS 打开测试页。

# Opened the test page through tracewing, but without HTTPS.
t7d4b60bb = 已通过 tracewing 打开测试页，但未使用 HTTPS。

# Browsers
td580efd4 = 浏览器

# Each keeps its own idea of what to trust and where to send its traffic. Check opens the test page in it. Open on its own starts it with a profile of its own that goes through tracewing, the system proxy left alone.
tce72f1ec = 每个浏览器各自决定信任什么、把流量发往何处。“检查”会在该浏览器中打开测试页。“单独打开”会用一个经过 tracewing 的独立配置文件启动它，不改动系统代理。

# Look again
t98416222 = 重新查找

# No browser found on this computer.
t0e46bb34 = 在这台电脑上没有找到浏览器。

# Check
t592c8de7 = 检查

# Open on its own
t0da7783c = 单独打开

# UDP port 443 is blocked on this computer, so apps fall back from HTTP/3 to HTTP/2 and go through tracewing. It stops when tracewing closes, or if it crashes.
t3c727cd5 = 本机已阻止 UDP 443 端口，因此应用会从 HTTP/3 回退到 HTTP/2 并经过 tracewing。tracewing 关闭或崩溃时，阻止将自动解除。

# Some apps speak HTTP/3, over UDP, and go around the proxy. Blocking UDP port 443 on this computer makes them fall back to HTTP/2 through tracewing. It takes an administrator's password, and stops when tracewing closes. A VPN that uses UDP port 443 stops working meanwhile.
t64ab600f = 有些应用通过 UDP 使用 HTTP/3，会绕过代理。在本机阻止 UDP 443 端口，可让它们回退到 HTTP/2 并经过 tracewing。此操作需要管理员密码，tracewing 关闭时会自动解除。在此期间，使用 UDP 443 端口的 VPN 将无法工作。

# Apps on HTTP/3
t2cfe5270 = 使用 HTTP/3 的应用

# Stop blocking
t318358d8 = 停止阻止

# Block HTTP/3
t88231e40 = 阻止 HTTP/3

# Or one app at a time
t91c63f1c = 或者逐个应用设置

# A terminal that goes through tracewing while the system proxy and every other app are left as they are. For a browser, Open on its own in Browsers does the same with a profile of its own.
tc21013a9 = 一个经过 tracewing 的终端，系统代理和其他应用都保持原样。浏览器可用“浏览器”中的“单独打开”，以独立配置文件做到同样的事。

# Open a terminal
t4b629c72 = 打开终端

# iPhone and iPad
ta550cc97 = iPhone 和 iPad

# Flutter apps
ta04ee397 = Flutter 应用

# 1. In Settings, Wi-Fi, choose the network, then Configure Proxy, Manual: server {server}, port {port}.
t996be047 = 1. 在“设置”>“无线局域网”中选择该网络，然后选择“配置代理”>“手动”：服务器填 { $server }，端口填 { $port }。

# 2. Scan the code with the camera, or open {page} in Safari, and get the certificate. Allow the download.
t4fa14138 = 2. 用相机扫描二维码，或在 Safari 中打开 { $page }，下载证书。在提示时选择“允许”。

# 3. In Settings, General, VPN & Device Management, choose the tracewing CA profile and install it.
t7127295f = 3. 在“设置”>“通用”>“VPN与设备管理”中，选择 tracewing CA 描述文件并安装。

# 4. In Settings, General, About, Certificate Trust Settings, turn on full trust for tracewing CA.
t1b22723c = 4. 在“设置”>“通用”>“关于本机”>“证书信任设置”中，为 tracewing CA 开启完全信任。

# 5. Open https://{host} in Safari. If it says it works, apps that use the system's settings go through tracewing.
t4adfec01 = 5. 在 Safari 中打开 https://{ $host }。如果页面显示工作正常，使用系统设置的应用就会经过 tracewing。

# 1. In Settings, Wi-Fi, edit the network, and under Advanced set the proxy to Manual: host {host}, port {port}.
t4f837b10 = 1. 在“设置”>“WLAN”中修改该网络，在“高级选项”中将“代理”设为“手动”：主机名填 { $host }，端口填 { $port }。

# 2. Scan the code, or open {page} in the browser, and get the certificate.
t654a829b = 2. 扫描二维码，或在浏览器中打开 { $page }，下载证书。

# 3. In Settings, Security, Encryption & credentials (the names vary by maker), choose Install a certificate, then CA certificate, and pick tracewing.crt.
t8263ba6b = 3. 在“设置”>“安全”>“加密与凭据”中（名称因厂商而异），选择“安装证书”>“CA 证书”，然后选择 tracewing.crt。

# 4. Since Android 7, apps trust certificates the user added only if they say so. For a debug build of your own app, add res/xml/network_security_config.xml:
tb7fa0462 = 4. 从 Android 7 开始，应用只有在明确声明时才会信任用户添加的证书。对于你自己应用的调试版本，请添加 res/xml/network_security_config.xml：

# and point to it from AndroidManifest.xml:
t02ad844b = 并在 AndroidManifest.xml 中引用它：

# Other apps keep refusing the certificate: their traffic shows as failed flows.
t02e57826 = 其他应用仍会拒绝该证书：它们的流量将显示为失败的流量记录。

# Dart's HttpClient, which Flutter's http and Dio use, does not follow the phone's proxy setting, and on Android does not trust certificates the user added. In a debug build, send it here:
t4ffc0abf = Dart 的 HttpClient（Flutter 的 http 和 Dio 都基于它）不会遵循手机的代理设置，并且在 Android 上不信任用户添加的证书。在调试版本中，可将其指向这里：

# badCertificateCallback lets any certificate through, which is why it is for debug builds only.
t991340f1 = badCertificateCallback 会放行任何证书，因此只能用于调试版本。

# Open settings
t1336b4d6 = 打开设置

# Phones and other devices
t2e3c591c = 手机和其他设备

# A phone, a tablet or another computer on this network can go through tracewing at {proxy}. The code opens the page with its certificate. Its flows show its address, can be filtered with device:, and can be named from one of them, in Connection.
td0629ca6 = 此网络中的手机、平板或其他电脑可以通过 { $proxy } 经由 tracewing 连接。扫描二维码即可打开证书下载页面。其流量记录会显示设备地址，可用 device: 过滤，也可在任一条流量记录的“连接”中为设备命名。

# Set up. The certificate is among the system's until the device restarts.
t84db6961 = 已设置完成。在设备重启之前，该证书会保留在系统证书中。

# Set up. The certificate is in Downloads: in the Settings just opened, choose Encryption & credentials, Install a certificate, CA certificate, and pick tracewing.crt.
tbcad7fa7 = 已设置完成。证书位于“下载”文件夹：在刚打开的“设置”中，选择“加密与凭据”>“安装证书”>“CA 证书”，然后选择 tracewing.crt。

# Apps that trust only the system's certificates will still refuse it.
t6fb5ba4b = 只信任系统证书的应用仍会拒绝它。

# An emulator.
t14b6b86d = 模拟器。

# On USB.
t67dafc14 = 通过 USB 连接。

# Accept the USB debugging prompt on the device, then look again.
tff6a046e = 请在设备上接受 USB 调试提示，然后重新查找。

# Still starting. Look again in a moment.
t13f68b64 = 仍在启动中。请稍后重新查找。

# adb says it is {state}.
t2b7d71da = adb 报告其状态为 { $state }。

# Sets the proxy on an Android emulator, or a phone plugged in with USB debugging on, and installs the certificate. The traffic comes over ADB, so neither has to be on this network.
tc28e5d10 = 为 Android 模拟器或已开启 USB 调试并连接的手机设置代理，并安装证书。流量通过 ADB 传输，因此它们无需连接到此网络。

# adb was not found. Install Android Studio, or Android's platform tools, then look again.
t4af01006 = 未找到 adb。请安装 Android Studio 或 Android 平台工具，然后重新查找。

# No device is connected. Start an emulator, or plug in a phone with USB debugging on, then look again.
tc65b7cf7 = 未连接任何设备。请启动模拟器，或连接已开启 USB 调试的手机，然后重新查找。

# Set up a device to send its traffic here. Undo, or closing tracewing, takes the proxy off it again.
t9144d1fc = 设置设备，将其流量发送到这里。点击“撤销”或关闭 tracewing，即可移除设备上的代理设置。

# Undo
t71fd4acf = 撤销

# Android emulators and USB devices
t379e433c = Android 模拟器和 USB 设备

# Find devices
t54d4af39 = 查找设备

# Claude Code, Cursor, VS Code and other AI tools can list and read the flows tracewing records, through its MCP server. Tokens and cookies stay out unless Settings let them in. In Claude Code:
t2dc39047 = Claude Code、Cursor、VS Code 等 AI 工具可以通过 tracewing 的 MCP 服务器列出并读取其录制的流量记录。除非在“设置”中允许，否则不会包含令牌和 Cookie。在 Claude Code 中：

# Elsewhere, in the tool's MCP settings:
tec38833c = 在其他工具中，请在其 MCP 设置中添加：

# Flows
t4db77db2 = 流量记录

# Choose the flow above or below
tecb3adec = 选择上一条或下一条流量记录

# The first or the last flow
t7e278a00 = 第一条或最后一条流量记录

# A page up or down
t23b1814b = 向上或向下翻页

# Filter
tf4a9c097 = 过滤

# Pause or resume recording
t6e21519a = 暂停或继续录制

# Clear the list
tf2a55d1f = 清空列表

# The chosen flow
te0a729de = 所选流量记录

# Its request
tb3970784 = 查看请求

# Its response
t221fc168 = 查看响应

# Send it again
tef7d1944 = 重新发送

# Edit it and send it
t6bc99e5c = 编辑后发送

# Copy it as cURL
tc99a11a3 = 复制为 cURL

# Select all of the part shown
tf9168168 = 全选当前显示的部分

# Copy the selected text
t00b090d3 = 复制所选文本

# The window
t2af18488 = 窗口

# Set up, Traffic, Rules, Paused, Compose, Sessions, Settings
t9df89562 = 配置、流量、规则、已暂停、编写、会话、设置

# Show or hide the sidebar
t67adca04 = 显示或隐藏侧边栏

# Show or hide the inspector
ta5a7b9e9 = 显示或隐藏检查器

# The inspector beside the list or below it
t51e214f7 = 将检查器置于列表旁边或下方

# These shortcuts
tbdf3b62b = 此快捷键列表

# Close them
tf3a4b1ef = 关闭快捷键列表

# Keyboard shortcuts
t31eaddb7 = 键盘快捷键

# Close
tcd86acc3 = 关闭

# The certificate: {e}
t0235648c = 证书：{ $e }

# The terminal
tf542b2f4 = 终端

# The system proxy: {e}
tc99726ce = 系统代理：{ $e }

# Blocking HTTP/3: {e}
t024ec871 = 阻止 HTTP/3：{ $e }

# host:, status:4xx, or text in bodies
t00d0cf30 = host:、status:4xx 或正文中的文字

# A name, as iPhone test
t19f89214 = 名称，如：测试用 iPhone

# The search index: {e}
te3c971c9 = 搜索索引：{ $e }

# An earlier session: {name}
t73446a1d = 历史会话：{ $name }

# Read-only. Search covers more as its index is finished. Recording goes on.
td8659d5b = 只读。随着索引逐步完成，搜索范围会不断扩大。录制仍在继续。

# Read-only. Recording goes on.
te2f35d9e = 只读。录制仍在继续。

# Back to live
t317c0453 = 返回实时

# {shown} of {total}
tf2cc0563 = { $shown } / { $total }

# Recording
te75cf20c = 录制

# System proxy off · {addr}
t7e813d25 = 系统代理已关闭 · { $addr }

# System proxy…
t29e7dcd0 = 系统代理…

# System proxy on · {addr}
t319ad6b9 = 系统代理已开启 · { $addr }

# Apps
t53d91aad = 应用

# Devices
t27acf320 = 设备

# Hosts
tcf9aefb4 = 主机

# All hosts
tfc7bfe37 = 所有主机

# Hidden
t3c2dfe59 = 已隐藏

# Newest
t324b6d0b = 最新

# No flow fits this filter
t7eb295f4 = 没有符合此过滤条件的流量记录

# Nothing yet
tbd00747c = 暂无内容

# Turn on the system proxy above, or point an app at {addr}. To try it: curl --proxy http://{addr} --cacert {ca} https://example.com/
t61730a16 = 开启上方的系统代理，或将应用指向 { $addr }。试一试：curl --proxy http://{ $addr } --cacert { $ca } https://example.com/

# Beside the list
t3b4853f8 = 列表旁边

# Below the list
t23658daf = 列表下方

# Drop it on a place · Esc puts it back
t0fa632ba = 拖放到目标位置 · 按 Esc 还原

# Traffic
t4b589266 = 流量

# Certificate trusted on this computer
t7b93f551 = 本机已信任证书

# Checking the certificate…
tbf8942c3 = 正在检查证书…

# Installing the certificate; the system may ask you to confirm…
t068dcaff = 正在安装证书，系统可能会要求你确认…

# Removing the certificate; the system may ask you to confirm…
t913da735 = 正在移除证书；系统可能会请你确认…

# HTTPS will fail until the certificate is trusted: set up
t4b34beb2 = 在证书受信任之前，HTTPS 将会失败：去配置

# settings.toml was not used: {problem}
tc3e1f9c8 = 未使用 settings.toml：{ $problem }

# The proxy stopped: {failure}
t04dcff11 = 代理已停止：{ $failure }

# The disk is nearly full: bodies are not being kept
te99f9959 = 磁盘即将用满：暂不保留正文

# {count} flow not recorded | {count} flows not recorded
tdbd64fe6 =
    { $n ->
       *[other] { $count } 条流量记录未录制
    }

# {count} flow without bodies | {count} flows without bodies
t249093b0 =
    { $n ->
       *[other] { $count } 条流量记录没有正文
    }

# {count} oldest flow dropped to stay under {size} | {count} oldest flows dropped to stay under {size}
te30201f0 =
    { $n ->
       *[other] 为保持在 { $size } 以内，已丢弃最早的 { $count } 条流量记录
    }

# {count} flow | {count} flows
t57bf5200 =
    { $n ->
       *[other] { $count } 条流量记录
    }

# Export as HAR
t95b6b766 = 导出为 HAR

# Shortcuts {keys}
t4ab0ecb4 = 快捷键 { $keys }

# Columns
tea156fdc = 列
